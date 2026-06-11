use anyhow::Result;

use crate::agent::runner::run_agent;
use crate::agent::types::{AgentConfig, AgentPlan};

pub fn plan_next_step(agent: &AgentConfig, goal: &str, state: &str) -> Result<AgentPlan> {
    let prompt = format!(
        r#"You are a lead software engineer executing a project plan.

## Goal
{}

## Current State
{}

Your task: Determine the single most impactful next step to make progress toward the goal.
Output a short summary of what to do and a list of sub-tasks (1-3 items).
Format your response as:
SUMMARY: <one-line summary>
TASKS:
- <task 1>
- <task 2>"#,
        goal, state
    );

    let result = run_agent(agent, &prompt)?;
    parse_plan(&result.stdout)
}

fn parse_plan(output: &str) -> Result<AgentPlan> {
    let mut summary = String::new();
    let mut sub_tasks = Vec::new();

    for line in output.lines() {
        if let Some(s) = line.strip_prefix("SUMMARY:") {
            summary = s.trim().to_string();
        } else if let Some(t) = line.strip_prefix("- ") {
            sub_tasks.push(t.trim().to_string());
        }
    }

    if summary.is_empty() {
        summary = "Continue working on the goal".to_string();
    }

    Ok(AgentPlan { summary, sub_tasks })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plan() {
        let output = "SUMMARY: Implement the CLI parser\nTASKS:\n- Add clap definitions\n- Test argument parsing\n";
        let plan = parse_plan(output).unwrap();
        assert_eq!(plan.summary, "Implement the CLI parser");
        assert_eq!(plan.sub_tasks.len(), 2);
    }

    #[test]
    fn test_parse_plan_fallback() {
        let output = "Some random output without the expected format";
        let plan = parse_plan(output).unwrap();
        assert_eq!(plan.summary, "Continue working on the goal");
        assert!(plan.sub_tasks.is_empty());
    }
}
