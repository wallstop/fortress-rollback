# Fortress Rollback Plan

> Active and ordered future work only. Put scope and discussion in the linked issue or skill,
> and put completed work and evidence in local, gitignored `progress/session-*.md` files (never commit them). See
> [Plan hygiene](.agents/skills/fortress-development/SKILL.md#plan-hygiene).

## In progress

- **P1 — Verify portability and long-session determinism.** The strict-build CI lane now replays the
  checked-in corpus under `-C overflow-checks=on -C debug-assertions=on` and compares the full
  trace/artifact identity; native OS matrix runs already replay the same corpus. A checked-in
  mixed-transport corpus schedule proves unreliable and reliable-retransmission links agree on
  confirmed prefixes. One captured replay artifact (`tests/replay-fixture/`) replays to the same
  request/state digest on native, browser `wasm32-unknown-unknown`, and Godot
  `wasm32-unknown-emscripten`; the browser lane also verifies background-tab interruption,
  resume, and timeout. Remaining: extended frame/ring operation. **Exit:** under one captured
  replay/schedule, requests and artifacts are byte-identical; live mixed transports agree on
  confirmed checkpoints despite legal speculative timing differences. No panic or unexpected
  violation occurs; any platform limitation has a compatibility regression and narrowed support
  documentation. A simulator-only pass is not cross-platform evidence.

## Future

1. **P2 — Improve search only after the preceding oracle is calibrated.** Start with the existing sometimes-state census:
   evaluate its multi-night stability before any ratchet. Then use the
   [fleet-search roadmap](.agents/skills/dst/references/fleet-search-roadmap.md) to compare violation-site coverage, PCT,
   prefix branching, and LDFI-lite against fresh-seed search at equal compute. **Exit:**
   retain a technique only when it materially increases unique useful states or defect discovery per compute, preserves
   exact replay and bounded artifacts, and contributes a minimized regression; otherwise record the rejected experiment
   and keep the simpler fleet.
