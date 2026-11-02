const MAX_NETWORK_RETRY_ATTEMPT = 3
const MAX_NETWORK_RETRY_WINDOW_MS = 120_000
const NETWORK_FAILURE_PATTERN =
  /fetch failed|failed to fetch|network(?: error)?|socket|connection (?:closed|error|lost|refused|reset)|econn|enotfound|eai_again|etimedout|timed out|timeout|terminated|upstream connect|reset before headers/i

// OpenCode retries transient provider failures internally. Bound that recovery
// path so a broken uplink becomes an actionable failure instead of a stuck run.
export default async ({ client, directory }) => {
  const retryTimers = new Map()
  const abortingSessions = new Set()

  const clearRetryTimer = (sessionID) => {
    const timer = retryTimers.get(sessionID)
    if (timer === undefined) return
    clearTimeout(timer)
    retryTimers.delete(sessionID)
  }

  const abortSession = async (sessionID, reason) => {
    if (abortingSessions.has(sessionID)) return
    abortingSessions.add(sessionID)
    clearRetryTimer(sessionID)
    console.error(`[bounded-network-retries] Aborting session ${sessionID}: ${reason}`)
    try {
      await client.session.abort({
        path: { id: sessionID },
        query: { directory },
        throwOnError: true,
      })
    } catch (error) {
      console.error(`[bounded-network-retries] Failed to abort session ${sessionID}:`, error)
    }
  }

  return {
    event: async ({ event }) => {
      if (event.type !== "session.status") return

      const { sessionID, status } = event.properties
      if (status.type === "idle") {
        clearRetryTimer(sessionID)
        abortingSessions.delete(sessionID)
        return
      }
      if (status.type !== "retry" || !NETWORK_FAILURE_PATTERN.test(status.message)) return

      if (status.attempt >= MAX_NETWORK_RETRY_ATTEMPT) {
        await abortSession(sessionID, `network retry attempt ${status.attempt} reached the limit`)
        return
      }

      if (retryTimers.has(sessionID)) return
      const timer = setTimeout(() => {
        void abortSession(sessionID, "network retry window expired")
      }, MAX_NETWORK_RETRY_WINDOW_MS)
      timer.unref?.()
      retryTimers.set(sessionID, timer)
    },
    dispose: async () => {
      for (const timer of retryTimers.values()) clearTimeout(timer)
      retryTimers.clear()
      abortingSessions.clear()
    },
  }
}
