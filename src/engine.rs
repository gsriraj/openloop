use std::io::Write;
use std::time::Instant;

use anyhow::Result;
use colored::Colorize;

use crate::agent::runner::run_agent_with_stdin;
use crate::agent::types::{AgentConfig, AgentResult};
use crate::checker;
use crate::config::Config;
use crate::goal::Goal;
use crate::plan;
use crate::state::LoopState;
use crate::tui::{LogStyle, TuiHandle};

pub fn run_loop(config: &Config, state_dir: &str) -> Result<()> {
    run_loop_inner(config, state_dir, None)
}

pub fn run_loop_tui(config: &Config, state_dir: &str, handle: &TuiHandle) -> Result<()> {
    run_loop_inner(config, state_dir, Some(handle))
}

fn output(handle: Option<&TuiHandle>, msg: String) {
    output_styled(handle, msg, LogStyle::Normal)
}

fn output_styled(handle: Option<&TuiHandle>, msg: String, style: LogStyle) {
    if let Some(h) = handle {
        h.push_log(msg, style);
    } else {
        println!("{}", msg);
    }
}

fn run_loop_inner(config: &Config, state_dir: &str, tui: Option<&TuiHandle>) -> Result<()> {
    let goal = Goal::from_file(&config.goal)?;
    let mut state = LoopState::load(&state_path(state_dir, &config.state.file))?;

    if state.goal_path.is_empty() {
        state.goal_path = config.goal.clone();
    }

    output(
        tui,
        format!(
            "{} {}",
            "Goal:".bright_blue().bold(),
            goal.summary().white()
        ),
    );
    output(
        tui,
        format!(
            "{} {}",
            "Starting iteration".bright_blue().bold(),
            (state.iteration + 1).to_string().cyan()
        ),
    );

    let plan_agent = build_agent_config(config, true)?;

    if let Some(h) = tui {
        h.set_iteration(state.iteration + 1);
        h.set_status("Planning");
        h.set_phase("Plan");
    }

    for iteration in state.iteration + 1..=config.max_iterations {
        let elapsed = if let Some(h) = tui {
            h.set_iteration(iteration);
            output(
                tui,
                format!(
                    "{} {} {}/{}",
                    "─".repeat(50).dimmed(),
                    "Iteration".bold(),
                    iteration.to_string().cyan(),
                    config.max_iterations.to_string().dimmed()
                ),
            );
            output(tui, "".to_string());
            Instant::now()
        } else {
            output(
                tui,
                format!(
                    "\n{} {} {}/{}",
                    "─".repeat(50).dimmed(),
                    "Iteration".bold(),
                    iteration.to_string().cyan(),
                    config.max_iterations.to_string().dimmed()
                ),
            );
            Instant::now()
        };

        // Phase 1: Plan
        if let Some(h) = tui {
            h.set_phase("Plan");
            h.set_status("Planning next step");
        }
        output(tui, format!("  {} Planning next step...", "⏳".yellow()));
        if tui.is_none() {
            std::io::stdout().flush().ok();
        }
        let plan = plan::plan_next_step(&plan_agent, &goal.raw, &state_to_string(&state))?;
        output(tui, format!("\r  {} Planned", "✓".green()));
        output(
            tui,
            format!("\n{} {}", "Plan:".green().bold(), plan.summary),
        );
        for task in &plan.sub_tasks {
            output(tui, format!("  {} {}", "→".cyan(), task));
        }

        if !config.autopilot {
            let cont = inquire::Confirm::new("Continue with this plan?")
                .with_default(true)
                .with_help_message("Enter to continue, 'n' to stop")
                .prompt()
                .unwrap_or(false);

            if !cont {
                output(tui, format!("{}", "Stopped by user.".yellow()));
                break;
            }
        }

        // Phase 2: Dispatch (parallel or single)
        use crate::parallel;

        let parallel_plan = if config.parallel {
            parallel::split_work(config, &goal.raw, &state_to_string(&state))?
        } else {
            None
        };

        let (result, _used_parallel) = if let Some(pp) = parallel_plan {
            if let Some(h) = tui {
                h.set_phase("Parallel");
            }
            output(
                tui,
                format!("{} {}", "Parallel:".cyan().bold(), pp.sub_tasks.len()),
            );
            for (i, task) in pp.sub_tasks.iter().enumerate() {
                output(tui, format!("  {} [Agent {}] {}", "▸".cyan(), i + 1, task));
            }
            let results = parallel::execute_parallel(pp, config)?;
            let combined = results
                .iter()
                .map(|r| format!("--- {} ---\n{}", r.agent, r.stdout))
                .collect::<Vec<_>>()
                .join("\n");

            let merged_result = AgentResult {
                agent: "parallel".into(),
                exit_code: results.iter().all(|r| r.success()) as i32,
                stdout: combined,
                stderr: String::new(),
                duration_ms: results.iter().map(|r| r.duration_ms).sum(),
            };
            (merged_result, true)
        } else {
            let exec_agent = build_agent_config(config, true)?;
            if let Some(h) = tui {
                h.set_phase("Execute");
                h.set_status(&format!("Executing: {}", truncate(&plan.summary, 50)));
            }
            output(
                tui,
                format!(
                    "  {} Executing: {}",
                    "⏳".yellow(),
                    truncate(&plan.summary, 50)
                ),
            );
            if tui.is_none() {
                std::io::stdout().flush().ok();
            }
            let result = run_agent_with_stdin(
                &exec_agent,
                &format!(
                    r#"Goal: {}
State: {}
Plan: {}

Execute the plan above. Make concrete changes to the codebase.
Report what you did and any issues encountered."#,
                    goal.raw,
                    state_to_string(&state),
                    plan.summary
                ),
            )?;
            (result, false)
        };

        let exec_elapsed = elapsed.elapsed();
        output(
            tui,
            format!(
                "\r  {} Executed in {:.1}s (exit: {})",
                if result.success() {
                    "✓".green()
                } else {
                    "✘".red()
                },
                exec_elapsed.as_secs_f64(),
                result.exit_code.to_string().yellow()
            ),
        );
        if let Some(h) = tui {
            h.set_elapsed(exec_elapsed);
        }

        // Phase 3: Verify
        let verifier_agent = build_agent_config(config, false)?;
        if let Some(h) = tui {
            h.set_phase("Verify");
            h.set_status("Verifying goal progress");
        }
        output(tui, format!("  {} Verifying progress...", "⏳".yellow()));
        if tui.is_none() {
            std::io::stdout().flush().ok();
        }

        let verification = checker::verify_goal(
            &verifier_agent,
            &goal.raw,
            &format!(
                "## Last Plan\n{}\n\n## Last Result\n{}",
                plan.summary,
                truncate(&result.stdout, 2000)
            ),
        )?;
        output(tui, format!("\r  {} Verified", "✓".green()));

        if verification.goal_met {
            output(tui, format!("\n{}", "✓ Goal achieved!".green().bold()));
            output(tui, format!("  Reason: {}", verification.reason));
            state.goal_met = true;
            state.last_plan = plan.summary;
            state.last_result = truncate(&result.stdout, 1000);
            state.iteration = iteration;
            state.save(&state_path(state_dir, &config.state.file))?;
            if let Some(h) = tui {
                h.set_status("Goal achieved!");
            }
            return Ok(());
        }

        output(tui, String::new());
        output(tui, format!("  {} {}", "◷".yellow(), verification.reason));
        for item in &verification.remaining_items {
            output(tui, format!("    {} {}", "•".dimmed(), item));
        }

        // Update state
        state.last_plan = plan.summary;
        state.last_result = truncate(&result.stdout, 1000);
        state.iteration = iteration;
        state.save(&state_path(state_dir, &config.state.file))?;

        if let Some(h) = tui {
            h.set_status(&format!("Iteration {} complete", iteration));
        }
    }

    output(
        tui,
        format!(
            "\n{} Reached max iterations ({}) without meeting goal.",
            "◼".red(),
            config.max_iterations
        ),
    );
    Ok(())
}

fn build_agent_config(config: &Config, planning: bool) -> Result<AgentConfig> {
    let name = config
        .agents
        .enabled
        .first()
        .cloned()
        .unwrap_or_else(|| "opencode".to_string());

    let agent_cfg = config
        .agents
        .configs
        .get(&name)
        .cloned()
        .unwrap_or_default();

    let model = if planning {
        agent_cfg.model.clone()
    } else {
        agent_cfg
            .model_config
            .get("verify_model")
            .cloned()
            .unwrap_or(agent_cfg.model)
    };

    Ok(AgentConfig {
        name,
        model,
        model_config: agent_cfg.model_config,
    })
}

fn state_path(state_dir: &str, file: &str) -> String {
    format!("{}/{}", state_dir, file)
}

fn state_to_string(state: &LoopState) -> String {
    format!(
        "Iteration: {}\nGoal Met: {}\nLast Plan: {}\nLast Result: {}",
        state.iteration, state.goal_met, state.last_plan, state.last_result
    )
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let s = truncate("hello world", 5);
        assert_eq!(s, "hello...");
    }

    #[test]
    fn test_state_to_string() {
        let state = LoopState::new("GOAL.md");
        let s = state_to_string(&state);
        assert!(s.contains("Iteration: 0"));
        assert!(s.contains("Goal Met: false"));
    }
}
