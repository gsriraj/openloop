1/ You know what's broken about AI coding tools?

You describe your project, the agent builds something, then you're back to square one for the next feature.

No continuity. No persistence. No iteration.

2/ I built OpenLoop — a CLI that turns coding agents into autonomous builders.

You run `openloop`, tell it your goal once, and it loops:
   Plan → Execute → Verify → Repeat

Until the goal is met.

3/ It works with whatever agent you already use:
   • opencode
   • GitHub Copilot
   • Claude Code
   • aider

Detects them on $PATH automatically.

4/ The setup wizard is AI-guided:
   • Describe your project in one sentence
   • The agent asks clarifying questions
   • It drafts a structured GOAL.md with stringent success criteria
   • Pick your agents and models
   • Go — step-by-step or full autopilot

5/ During the loop you get live feedback:
   ⏳ Planning next step...
   ✓ Executed in 34s
   ⏳ Verifying progress...
   ✓ Goal not yet met: Feature X remaining

6/ Parallel mode splits work across multiple agents using git worktrees — each agent works on their own branch, results get merged automatically.

7/ It's open source. MIT licensed. Written in Rust.

   https://github.com/gsriraj/openloop

   cargo install openloop

8/ Try it on a side project. Let an agent loop build the whole thing while you review.

Star the repo if this resonates → https://github.com/gsriraj/openloop

#opensource #rust #ai #coding #cli
