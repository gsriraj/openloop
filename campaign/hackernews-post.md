OpenLoop — a CLI that delegates work to coding agents in a loop until a goal is achieved

https://github.com/gsriraj/openloop

Inspired by the "Loop Engineering" concept — instead of prompting an agent manually for each task, you define a goal once and let the system iterate toward it autonomously.

```
openloop
  → Interactive wizard detects agents on $PATH
  → Guides you through goal creation with AI coaching
  → Writes config + GOAL.md
  → Starts loop: plan → execute → verify → repeat
```

Works with opencode, copilot, claude, aider. Parallel mode splits work across agents using git worktrees.

Written in Rust. MIT. Would love feedback.

```bash
cargo install openloop
```