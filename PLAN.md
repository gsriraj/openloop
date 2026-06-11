# OpenLoop — Loop Engineering CLI

A Rust CLI tool that reads a `GOAL.md`, then autonomously loops through **plan → dispatch → collect → verify → update state → repeat** until the goal is achieved. It delegates work to any coding agent CLI (opencode, copilot, claude, etc.) via subprocess calls.

Inspired by [Loop Engineering](https://addyosmani.com/blog/loop-engineering/) — designing systems that prompt agents instead of prompting them manually.

---

## CLI Interface

```
openloop                         # Interactive TUI wizard (no args)
openloop [FLAGS] [OPTIONS]       # Headless mode with flags

Flags:
  --autopilot          Run fully autonomously (no human confirmations)
  --init               Scaffold .openloop/ config directory + example GOAL.md
  --status             Display current loop state and progress
  --parallel           Allow parallel agent execution (uses git worktrees)
  --help               Print help

Options:
  --goal <PATH>                 Goal file path (default: ./GOAL.md)
  --agent-cli <NAME>...         Agent CLI(s) to use (repeatable, e.g. opencode copilot)
  --model <NAME>                Model override for all agents
  --model-config <KEY=VAL>...   Model params (repeatable, e.g. temperature=0.7)
  --max-iterations <N>          Max loop iterations (default: 50)
  --state-dir <PATH>            Config/state directory (default: .openloop)
```

### Behavior

| Invocation | What happens |
|------------|--------------|
| `openloop` (no args, no `.openloop/config.toml`) | Launch interactive setup wizard |
| `openloop` (no args, `.openloop/config.toml` exists) | Load config, start loop |
| `openloop --goal GOAL.md [FLAGS]` | Headless mode — same as original plan |

### Examples

```bash
# Interactive wizard — guides you through setup
openloop

# Single agent, autopilot (headless)
openloop --agent-cli opencode --model claude-sonnet-4-20250514 --autopilot

# Parallel with two agents (headless)
openloop --agent-cli opencode --agent-cli copilot --parallel --autopilot

# Custom config (headless)
openloop --agent-cli opencode --model-config temperature=0.3 --max-iterations 100

# Initialize a new project
openloop --init
```

---

## Core Loop (engine.rs)

```
                    ┌─────────────────┐
                    │   Read GOAL.md   │
                    │ (free-form, AI   │
                    │  interprets it)  │
                    └────────┬────────┘
                             ▼
                    ┌─────────────────┐
                    │   Load State    │
                    │ (last iteration, │
                    │  what was tried) │
                    └────────┬────────┘
                             ▼
                    ┌─────────────────┐
                    │   Plan Next     │◄──── Ask agent: "What
                    │   Step (AI)     │      should we do next?"
                    └────────┬────────┘
                             ▼
                    ┌─────────────────┐
              ┌────►│   Dispatch to   │
              │     │   Agent CLI     │
              │     └────────┬────────┘
              │              ▼
              │     ┌─────────────────┐
              │     │   Collect       │
              │     │   Results       │
              │     └────────┬────────┘
              │              ▼
              │     ┌─────────────────┐
              │     │   Verify Goal   │◄──── Ask agent: "Is
              │     │   (AI Checker)  │      the goal met?"
              │     └────────┬────────┘
              │              ▼
              │     ┌─────────────────┐
              │     │  Update State   │
              │     │  (state.md)     │
              │     └────────┬────────┘
              │              ▼
              │     ┌─────────────────┐
              │     │  Goal Met?      │────► Yes → Report & exit
              │     └────────┬────────┘
              │              │ No
              │              ▼
              │     ┌─────────────────┐
              │     │  Max Iter?      │────► Yes → Report & exit
              │     └────────┬────────┘
              └──────────────┘ No, loop again
```

---

## Project Structure

```
openloop/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI entry point, dispatch to commands or wizard
│   ├── cli.rs            # Clap argument definitions
│   ├── config.rs         # TOML config loading + CLI flag merging
│   ├── engine.rs         # Main loop orchestrator
│   ├── wizard.rs         # Interactive setup wizard (inquire prompts)
│   ├── goal.rs           # GOAL.md reader (raw markdown)
│   ├── state.rs          # state.md read/write, checkpointing
│   ├── plan.rs           # Ask lead agent to plan next step
│   ├── checker.rs        # Ask verify agent if goal is met
│   ├── parallel.rs       # Split work, dispatch parallel tasks
│   ├── worktree.rs       # Git worktree create/merge/cleanup
│   └── agent/
│       ├── mod.rs
│       ├── runner.rs     # Subprocess dispatch to agent CLI
│       ├── discovery.rs  # Detect available agent CLIs on $PATH
│       └── types.rs      # AgentConfig, AgentResult structs
├── openloop-config.toml  # Example/default config
└── GOAL.md               # Example goal file
```

---

## Configuration (`.openloop/config.toml`)

```toml
goal = "GOAL.md"
max_iterations = 50
autopilot = false
parallel = false

[agents]
enabled = ["opencode", "copilot"]

[agents.opencode]
model = "claude-sonnet-4-20250514"
model_config = { temperature = 0.7, max_tokens = 8192 }

[agents.copilot]
model = "gpt-4o"
model_config = { temperature = 0.5 }

[state]
file = "state.md"
```

---

## How Parallel Execution Works

1. Each iteration, the lead agent is asked: *"Does this goal warrant parallel sub-tasks?"*
2. If yes, the agent returns a plan with independent sub-tasks
3. OpenLoop creates git worktrees (one per sub-task)
4. Dispatches each sub-task to an available agent CLI in parallel
5. Collects all results, merges, verifies goal completion, updates state

Triggered by the `--parallel` flag or `parallel = true` in config. The agent itself decides *if* parallelism is useful — OpenLoop just enables the infrastructure.

---

## Autopilot Mode

| Mode | Behavior |
|------|----------|
| `--autopilot` | No prompts — full autonomy. Loop runs, iterates, decides next steps, verifies itself |
| No flag | Pause between iterations; show plan/diff, ask "Continue? [Y/n]" |

---

## Interactive Setup Wizard

When `openloop` is invoked with no arguments and no existing config, it launches an interactive setup wizard that guides the user through goal creation, agent selection, and configuration.

```
$ openloop
  │
  ├── 1. Detect agent CLIs on $PATH
  │      (opencode, copilot, claude, etc. — checked via `which`)
  │
  ├── 2. Goal co-creation
  │      │
  │      ├── "Describe your project in one sentence."
  │      │     └── User types rough idea (e.g. "A CLI to manage todo lists")
  │      │
  │      ├── Agent asks clarifying questions (≤ 3 rounds):
  │      │     "What's the target platform? Should it support deadlines?"
  │      │     "Do you want subtasks, tags, priorities?"
  │      │
  │      ├── AI drafts a structured GOAL.md with stringent success criteria
  │      │
  │      └── "Edit the goal? [Y/n]" → opens $EDITOR if yes
  │
  ├── 3. Agent selection
  │      ┌─────────────────────────────────┐
  │      │ Select agent CLIs to use:       │
  │      │                                 │
  │      │ [x] opencode   (detected)       │
  │      │ [ ] copilot    (not found)      │
  │      │ [ ] claude     (detected)       │
  │      └─────────────────────────────────┘
  │
  ├── 4. Model configuration
  │      ┌────────────────────────────────────────┐
  │      │ Planning model:                        │
  │      │ > claude-sonnet-4-20250514 (recommended)│
  │      └────────────────────────────────────────┘
  │      ┌────────────────────────────────────────┐
  │      │ Same model for verification? [Y/n]     │
  │      └────────────────────────────────────────┘
  │      → If no, prompt for verification model
  │
  ├── 5. Execution mode
  │      ┌──────────────────────────────────────┐
  │      │ Execution mode:                      │
  │      │ > Step-by-step (recommended)         │
  │      │   Full autopilot                     │
  │      └──────────────────────────────────────┘
  │
  ├── 6. Write .openloop/config.toml
  │
  └── 7. Start the loop
         (simple log output + confirm prompts — no fancy TUI yet)
```

Step 2 (goal co-creation) uses the selected agent CLI to submit the user's rough idea and collect clarifying responses. This means the agent must be available on `$PATH` *before* the loop starts properly.

---

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
anyhow = "1"
colored = "2"
inquire = "0.7"
```

---

## Implementation Order

| Step | File(s) | What |
|------|---------|------|
| 1 | `main.rs`, `cli.rs` | Clap argument parsing — foundation that defines the interface |
| 2 | `config.rs` | TOML config loading, CLI flag merge logic |
| 3 | `agent/types.rs`, `agent/runner.rs`, `agent/discovery.rs` | Agent types, subprocess dispatch, `$PATH` detection |
| 4 | `wizard.rs` | Interactive setup: goal co-creation, agent/model selection, config gen |
| 5 | `goal.rs`, `state.rs` | Read GOAL.md, persist iteration state |
| 6 | `engine.rs` | Main loop — wire everything together |
| 7 | `plan.rs`, `checker.rs` | AI-driven planning and goal verification |
| 8 | `worktree.rs`, `parallel.rs` | Git worktrees + parallel task dispatch |
| 9 | `main.rs` (init command) | `--init` scaffolding for new projects |

---

### Phase 2 — Live TUI

After the core is stable, replace basic log output with a `ratatui` split-pane view:

- Top pane: current plan / iteration summary
- Bottom pane: live agent output stream
- Status bar: iteration count, goal check result, elapsed time
- Hotkeys: `p` pause, `s` skip iteration, `q` quit

---

### Open Source & Production Readiness

#### Repository Scaffolding

| File | What |
|------|------|
| `.github/ISSUE_TEMPLATE/bug_report.md` | Structured bug report form |
| `.github/ISSUE_TEMPLATE/feature_request.md` | Feature request template |
| `.github/PULL_REQUEST_TEMPLATE.md` | PR checklist |
| `.github/dependabot.yml` | Auto-update Rust + Action dependencies |
| `CONTRIBUTING.md` | Build, test, commit convention, PR process |
| `CODE_OF_CONDUCT.md` | Contributor Covenant v2.1 |
| `SECURITY.md` | Vulnerability reporting |

#### Commit Convention

All commits follow Conventional Commits enforced by commitlint CI:

```
feat: add interactive setup wizard
fix: handle missing agent CLI gracefully
docs: update README with wizard flow
test: add integration test for wizard config output
chore: bump clap to 4.5
```

#### Testing Strategy

| Level | What | Where |
|-------|------|-------|
| Unit tests | Every module — cli, config, state, agent/runner, agent/discovery, goal | `#[cfg(test)] mod tests` in each source file |
| Integration tests | End-to-end: spawn binary with `--help`, `--init`, mock agent scripts | `tests/` directory |
| Property-based | Config round-trip, state serialization | `proptest` in `tests/proptest.rs` |
| Mock agent | Shell script that returns canned responses | `tests/fixtures/mock-agent.sh` |

#### CI/CD (GitHub Actions)

| Workflow | Triggers | What |
|----------|----------|------|
| `ci.yml` | push + PR | `cargo check` → `cargo fmt --check` → `cargo clippy --deny warnings` → `cargo test` |
| `audit.yml` | weekly + Dependabot | `cargo audit` |
| `release.yml` | tag `v*` | Build binaries (Linux + macOS, x86_64 + aarch64), create GitHub Release, publish to crates.io |

#### Documentation

| Where | What |
|-------|------|
| README.md | Badges (CI, crate version, docs.rs, license), quick install, usage, link to docs.rs |
| docs.rs | All public APIs documented with `///` doc comments |
| Cargo.toml | `description`, `repository`, `homepage`, `documentation`, `keywords`, `categories`, `license` |
| `--help` output | Clear subcommand and flag descriptions |
