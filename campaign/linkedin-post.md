I built an open-source CLI that turns coding agents into autonomous builders.

Here's the problem: AI coding tools are great for one-shot tasks, but they lack continuity. You ask for a feature, it builds it, then you start from scratch for the next one. There's no persistent goal tracking, no iterative refinement, no autonomous loop.

Enter OpenLoop.

It's a Rust CLI that reads a GOAL.md and then autonomously loops through Plan → Execute → Verify → Update State → Repeat until every success criterion is met.

The interactive setup wizard guides you through goal creation with AI coaching — you describe your project, the agent asks clarifying questions, then drafts a structured goal with testable success criteria.

It works with whatever coding agent you already use (opencode, GitHub Copilot, Claude Code, aider) and supports parallel execution across multiple agents using git worktrees.

Tech stack: Rust, clap (CLI), inquire (interactive prompts), colored (terminal output), serde + toml (config).

GitHub: https://github.com/gsriraj/openloop
Install: cargo install openloop

MIT licensed. Contributions welcome.

Would love feedback from the community.

#opensource #rust #ai #devtools #cli
