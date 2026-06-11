use anyhow::Result;

use crate::agent::runner::run_agent_with_stdin;
use crate::agent::types::{AgentConfig, AgentResult};
use crate::config::Config;
use crate::worktree::{Worktree, ensure_git_repo};

pub struct ParallelPlan {
    pub sub_tasks: Vec<String>,
    pub agents: Vec<AgentConfig>,
}

pub fn split_work(config: &Config, goal: &str, state: &str) -> Result<Option<ParallelPlan>> {
    if !config.parallel || config.agents.enabled.len() < 2 {
        return Ok(None);
    }

    let lead_agent = build_lead_agent(config)?;

    let prompt = format!(
        r#"You are a project manager splitting work across multiple agents.

## Goal
{}

## Current State
{}

Can this work be split into independent parallel sub-tasks?
Respond with:
PARALLEL: true/false
TASKS:
- task description 1
- task description 2
- task description 3"#,
        goal, state
    );

    let result = run_agent_with_stdin(&lead_agent, &prompt)?;
    parse_split(result.stdout)
}

fn parse_split(output: String) -> Result<Option<ParallelPlan>> {
    let mut parallel = false;
    let mut sub_tasks = Vec::new();

    for line in output.lines() {
        if line.starts_with("PARALLEL:") {
            parallel = line
                .trim_start_matches("PARALLEL:")
                .trim()
                .eq_ignore_ascii_case("true");
        } else if line.starts_with("- ") {
            sub_tasks.push(line.trim_start_matches("- ").to_string());
        }
    }

    if !parallel || sub_tasks.is_empty() {
        return Ok(None);
    }

    Ok(Some(ParallelPlan {
        sub_tasks,
        agents: Vec::new(),
    }))
}

pub fn execute_parallel(plan: ParallelPlan, config: &Config) -> Result<Vec<AgentResult>> {
    ensure_git_repo()?;

    let agents = &config.agents.enabled;
    let mut handles = Vec::new();
    let mut results = Vec::new();

    for (i, task) in plan.sub_tasks.iter().enumerate() {
        let agent_name = agents.get(i % agents.len()).cloned().unwrap_or_default();
        let agent_config = config
            .agents
            .configs
            .get(&agent_name)
            .cloned()
            .unwrap_or_default();

        let agent = AgentConfig {
            name: agent_name.clone(),
            model: agent_config.model.clone(),
            model_config: agent_config.model_config.clone(),
        };

        let task = task.clone();

        handles.push(std::thread::spawn(move || -> Result<AgentResult> {
            let worktree = Worktree::create("main", &format!("task-{}", i))?;

            let result = run_agent_with_stdin(
                &agent,
                &format!(
                    "Execute this task independently:\n\n{}\n\nMake changes directly to the codebase.",
                    task
                ),
            )?;

            worktree.merge_and_cleanup()?;
            Ok(result)
        }));
    }

    for handle in handles {
        match handle.join() {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(e)) => eprintln!("Parallel task failed: {}", e),
            Err(_) => eprintln!("Parallel task thread panicked"),
        }
    }

    Ok(results)
}

fn build_lead_agent(config: &Config) -> Result<AgentConfig> {
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

    Ok(AgentConfig {
        name,
        model: agent_cfg.model,
        model_config: agent_cfg.model_config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_split_parallel() {
        let output = "PARALLEL: true\nTASKS:\n- Implement feature A\n- Implement feature B\n";
        let plan = parse_split(output.to_string()).unwrap().unwrap();
        assert_eq!(plan.sub_tasks.len(), 2);
        assert_eq!(plan.sub_tasks[0], "Implement feature A");
    }

    #[test]
    fn test_parse_split_not_parallel() {
        let output = "PARALLEL: false\nTASKS:\n- Do this sequentially\n";
        let plan = parse_split(output.to_string()).unwrap();
        assert!(plan.is_none());
    }

    #[test]
    fn test_parse_split_no_tasks() {
        let output = "PARALLEL: true\nTASKS:\n";
        let plan = parse_split(output.to_string()).unwrap();
        assert!(plan.is_none());
    }
}
