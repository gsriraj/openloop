use std::time::Instant;

use anyhow::Result;
use colored::Colorize;

use crate::agent::runner::run_agent_with_stdin;
use crate::agent::types::AgentConfig;
use crate::checker;
use crate::config::Config;
use crate::goal::Goal;
use crate::plan;
use crate::state::LoopState;

pub fn run_loop(config: &Config, state_dir: &str) -> Result<()> {
    let goal = Goal::from_file(&config.goal)?;
    let mut state = LoopState::load(&state_path(state_dir, &config.state.file))?;

    // Override path if empty (first run)
    if state.goal_path.is_empty() {
        state.goal_path = config.goal.clone();
    }

    println!(
        "\n{} {}",
        "Goal:".bright_blue().bold(),
        goal.summary().white()
    );
    println!(
        "{} {}",
        "Starting iteration".bright_blue().bold(),
        (state.iteration + 1).to_string().cyan()
    );

    let plan_agent = build_agent_config(&config, true)?;

    for iteration in state.iteration + 1..=config.max_iterations {
        println!(
            "\n{} {} {}/{}",
            "─".repeat(50).dimmed(),
            "Iteration".bold(),
            iteration.to_string().cyan(),
            config.max_iterations.to_string().dimmed()
        );

        // Phase 1: Plan
        let plan = plan::plan_next_step(&plan_agent, &goal.raw, &state_to_string(&state))?;
        println!("\n{} {}", "Plan:".green().bold(), plan.summary);
        for task in &plan.sub_tasks {
            println!("  {} {}", "→".cyan(), task);
        }

        if !config.autopilot {
            let cont = inquire::Confirm::new("Continue with this plan?")
                .with_default(true)
                .with_help_message("Enter to continue, 'n' to stop")
                .prompt()
                .unwrap_or(false);

            if !cont {
                println!("{}", "Stopped by user.".yellow());
                break;
            }
        }

        // Phase 2: Dispatch
        let exec_agent = build_agent_config(&config, true)?;
        println!("\n{} {}...", "Executing".yellow().bold(), plan.summary.dimmed());
        let start = Instant::now();

        let result = run_agent_with_stdin(&exec_agent, &format!(
            r#"Goal: {}
State: {}
Plan: {}

Execute the plan above. Make concrete changes to the codebase.
Report what you did and any issues encountered."#,
            goal.raw,
            state_to_string(&state),
            plan.summary
        ))?;

        let elapsed = start.elapsed();
        println!(
            "  {} Done in {:.1}s (exit: {})",
            if result.success() { "✔".green() } else { "✘".red() },
            elapsed.as_secs_f64(),
            result.exit_code.to_string().yellow()
        );

        // Phase 3: Verify
        let verifier_agent = build_agent_config(&config, false)?;
        println!("{} Verifying goal progress...", "  🔍".yellow());

        let verification = checker::verify_goal(&verifier_agent, &goal.raw, &format!(
            "## Last Plan\n{}\n\n## Last Result\n{}",
            plan.summary,
            truncate(&result.stdout, 2000)
        ))?;

        if verification.goal_met {
            println!("\n{}", "✓ Goal achieved!".green().bold());
            println!("  Reason: {}", verification.reason);
            state.goal_met = true;
            state.last_plan = plan.summary;
            state.last_result = truncate(&result.stdout, 1000);
            state.iteration = iteration;
            state.save(&state_path(state_dir, &config.state.file))?;
            return Ok(());
        }

        println!(
            "  {} Goal not yet met: {}",
            "◷".yellow(),
            verification.reason
        );
        for item in &verification.remaining_items {
            println!("    {} {}", "•".dimmed(), item);
        }

        // Update state
        state.last_plan = plan.summary;
        state.last_result = truncate(&result.stdout, 1000);
        state.iteration = iteration;
        state.save(&state_path(state_dir, &config.state.file))?;
    }

    println!(
        "\n{} Reached max iterations ({}) without meeting goal.",
        "◼".red(),
        config.max_iterations
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

    let agent_cfg = config.agents.configs.get(&name).cloned().unwrap_or_default();

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