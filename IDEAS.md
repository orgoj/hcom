# Ideas

- Support automatic or controlled compaction by having hcom send or inject `/compact` into a running agent CLI, either on demand or after a configured number of steps or tokens.
- Support step-by-step session lifecycle management: after an agent completes a phase or step, stop it and start a fresh session with bridged context (for example, a handoff bundle) to reduce token usage and prevent context degradation.
