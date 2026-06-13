# Loop Engineering: Building an Autonomous Agent CLI

## The Idea

Inspired by Addy Osmani's post on [Loop Engineering](https://addyosmani.com/blog/loop-engineering/), I built OpenLoop — a CLI tool that delegates work to coding agents in a loop until a goal is achieved.

The core insight: instead of prompting an agent manually for every single task, you define a goal once and let the system iterate toward it autonomously.

## How It Works

```
                     ┌─────────────────┐
                     │   Read GOAL.md   │
                     └────────┬────────┘
                              ▼
                     ┌─────────────────┐
                     │   Plan Next     │◄──── Ask agent
                     │   Step (AI)     │
                     └────────┬────────┘
                              ▼
                     ┌─────────────────┐
                     │   Dispatch to   │
                     │   Agent CLI     │
                     └────────┬────────┘
                              ▼
                     ┌─────────────────┐
                     │   Verify Goal   │◄──── Ask agent
                     │   (AI Checker)  │
                     └────────┬────────┘
                              ▼
                     ┌─────────────────┐
                     │  Goal Met?      │────► Done
                     └────────┬────────┘
                              │ No → Loop
```

## The Interactive Wizard

Run `openloop` with no arguments and it launches a wizard that:

1. **Detects agents** on your $PATH (opencode, copilot, claude, aider)
2. **Co-creates the goal** — you describe your idea, the agent asks clarifying questions, then drafts a structured GOAL.md
3. **Lets you pick agents and models** — including dynamic model discovery from `opencode models`
4. **Configures execution mode** — step-by-step or full autopilot

## The Loop

Each iteration:
1. The lead agent plans the next step
2. The task is dispatched to the agent CLI
3. A separate verifier agent checks if the goal is met
4. State is persisted to `.openloop/state.md`
5. If goal is met → exit. If not → loop again.

## Parallel Execution

With the `--parallel` flag, OpenLoop can split work across multiple agents using git worktrees. Each agent works on their own branch, and results are merged automatically.

## Tech Stack

- **Language**: Rust (edition 2024)
- **CLI Framework**: clap with derive macros
- **Interactive Prompts**: inquire
- **Configuration**: TOML via serde
- **Terminal Output**: colored crate
- **CI/CD**: GitHub Actions (5 platforms), cargo-deny, cargo-audit

## Getting Started

```bash
cargo install openloop
openloop
```

Or for headless mode:
```bash
openloop --agent-cli opencode --model openrouter/anthropic/claude-sonnet-4 --autopilot
```

## The Future

- **Live TUI** with ratatui — split-pane view with real-time agent output
- **Homebrew formula** and winget package
- **Plugin system** for custom agents

## Links

- GitHub: https://github.com/gsriraj/openloop
- crates.io: https://crates.io/crates/openloop
- License: MIT
