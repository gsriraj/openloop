//! OpenLoop — a CLI tool that delegates work to coding agents in a loop until a goal is achieved.
//!
//! # Architecture
//!
//! The core loop runs in four phases:
//!
//! 1. **Plan** — Ask the lead agent: "What should we do next?"
//! 2. **Dispatch** — Execute the plan via subprocess agent CLI
//! 3. **Verify** — Ask a checker agent: "Is the goal met?"
//! 4. **Update State** — Persist progress and either exit or loop
//!
//! # Interactive Mode
//!
//! Running `openloop` with no arguments launches a setup wizard
//! that guides you through goal creation, agent selection, and
//! configuration before starting the loop.
//!
//! # Headless Mode
//!
//! All flags and options work without interaction, suitable for
//! CI/CD pipelines or automated workflows.

pub mod agent;
pub mod checker;
pub mod cli;
pub mod config;
pub mod engine;
pub mod goal;
pub mod parallel;
pub mod plan;
pub mod state;
pub mod tui;
pub mod wizard;
pub mod worktree;
