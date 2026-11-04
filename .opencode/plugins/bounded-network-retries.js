import { Plugin } from "@opencode/plugin"

const MAX_NETWORK_RETRY_ATTEMPT = 3
const MAX_NETWORK_RETRY_WINDOW_MS = 120_000
const INTERRUPT_TIMEOUT_MS = 10_000
const NETWORK_FAILURE_PATTERN =
  /fetch failed|failed to fetch|network(?: error)?|socket|connection (?:closed|error|lost|refused|reset)|econn|enotfound|eai_again|etimedout|timed out|timeout|terminated|upstream connect|reset before headers/i

// OpenCode retries transient provider failures internally. Bound that recovery
// path so a broken uplink becomes an actionable failure instead of a stuck run.
export default Plugin.define({
  id: "fortress.bounded-network-retries",
  setup(ctx) {
    const retryTimers = new Map()
    const interruptTimeouts = new Set()
    const interruptingSessions = new Set()
    const retryingSessions = new Set()
    const sessionGenerations = new Map()
    const controller = new AbortController()

    const clearRetryTimer = (sessionID) => {
      const timer = retryTimers.get(sessionID)
      if (timer === undefined) return
      clearTimeout(timer)
      retryTimers.delete(sessionID)
    }

    const scheduleRetryTimer = (sessionID) => {
      if (
        controller.signal.aborted ||
        !retryingSessions.has(sessionID) ||
        retryTimers.has(sessionID)
      )
        return
      const timer = setTimeout(() => {
        void interruptSession(sessionID, "network retry window expired")
      }, MAX_NETWORK_RETRY_WINDOW_MS)
      timer.unref?.()
      retryTimers.set(sessionID, timer)
    }

    const interruptSession = async (sessionID, reason) => {
      if (
        controller.signal.aborted ||
        !retryingSessions.has(sessionID) ||
        interruptingSessions.has(sessionID)
      )
        return
      const generation = sessionGenerations.get(sessionID) ?? 0
      interruptingSessions.add(sessionID)
      clearRetryTimer(sessionID)
      console.error(`[bounded-network-retries] Interrupting session ${sessionID}: ${reason}`)
      let timeout
      try {
        await Promise.race([
          ctx.session.interrupt({ sessionID, resume: false }),
          new Promise((_, reject) => {
            timeout = setTimeout(() => reject(new Error("session interrupt timed out")), INTERRUPT_TIMEOUT_MS)
            timeout.unref?.()
            interruptTimeouts.add(timeout)
          }),
        ])
      } catch (error) {
        if ((sessionGenerations.get(sessionID) ?? 0) !== generation) return
        // A failed or hung interrupt must not suppress future events permanently.
        interruptingSessions.delete(sessionID)
        if (!controller.signal.aborted && retryingSessions.has(sessionID)) {
          scheduleRetryTimer(sessionID)
          console.error(`[bounded-network-retries] Failed to interrupt session ${sessionID}:`, error)
        }
      } finally {
        if (timeout !== undefined) {
          clearTimeout(timeout)
          interruptTimeouts.delete(timeout)
        }
      }
    }

    void (async () => {
      try {
        for await (const event of ctx.event.subscribe({
          types: ["session.status"],
          signal: controller.signal,
        })) {
          const { sessionID, status } = event.data
          if (status.type === "idle") {
            sessionGenerations.set(sessionID, (sessionGenerations.get(sessionID) ?? 0) + 1)
            clearRetryTimer(sessionID)
            retryingSessions.delete(sessionID)
            interruptingSessions.delete(sessionID)
            continue
          }
          if (status.type !== "retry" || !NETWORK_FAILURE_PATTERN.test(status.message)) continue

          retryingSessions.add(sessionID)
          if (status.attempt >= MAX_NETWORK_RETRY_ATTEMPT) {
            void interruptSession(sessionID, `network retry attempt ${status.attempt} reached the limit`)
            continue
          }

          scheduleRetryTimer(sessionID)
        }
      } catch (error) {
        if (!controller.signal.aborted) {
          console.error("[bounded-network-retries] Event subscription failed:", error)
        }
      }
    })()

    return () => {
      controller.abort()
      for (const timer of retryTimers.values()) clearTimeout(timer)
      for (const timer of interruptTimeouts) clearTimeout(timer)
      retryTimers.clear()
      interruptTimeouts.clear()
      retryingSessions.clear()
      interruptingSessions.clear()
      sessionGenerations.clear()
    }
  },
})
