use anyhow::Result;

use crate::agent::runner::run_agent;
use crate::agent::types::{AgentConfig, VerificationResult};

pub fn verify_goal(agent: &AgentConfig, goal: &str, state: &str) -> Result<VerificationResult> {
    let prompt = format!(
        r#"You are a QA engineer verifying whether a project goal has been achieved.

## Goal
{}

## Current Work
{}

Review the goal's success criteria carefully. For each criterion, determine if it is met.
Output:
GOAL_MET: true/false
REASON: <brief explanation of your verdict>
REMAINING:
- <any remaining work, or "none" if goal is met>"#,
        goal, state
    );

    let result = run_agent(agent, &prompt)?;
    parse_verification(&result.stdout)
}

fn parse_verification(output: &str) -> Result<VerificationResult> {
    let mut goal_met = false;
    let mut reason = String::new();
    let mut remaining_items = Vec::new();

    for line in output.lines() {
        if let Some(val) = line.strip_prefix("GOAL_MET:") {
            goal_met = val.trim().eq_ignore_ascii_case("true");
        } else if let Some(val) = line.strip_prefix("REASON:") {
            reason = val.trim().to_string();
        } else if let Some(item) = line.strip_prefix("- ") {
            remaining_items.push(item.trim().to_string());
        }
    }

    if reason.is_empty() {
        reason = if goal_met { "Goal appears to be met".into() } else { "Goal not yet met".into() };
    }

    Ok(VerificationResult {
        goal_met,
        reason,
        remaining_items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_verification_met() {
        let output = "GOAL_MET: true\nREASON: All criteria satisfied\nREMAINING:\n- none\n";
        let vr = parse_verification(output).unwrap();
        assert!(vr.goal_met);
        assert_eq!(vr.reason, "All criteria satisfied");
    }

    #[test]
    fn test_parse_verification_not_met() {
        let output = "GOAL_MET: false\nREASON: CLI parser not implemented\nREMAINING:\n- Add argument parsing\n- Write tests\n";
        let vr = parse_verification(output).unwrap();
        assert!(!vr.goal_met);
        assert!(!vr.remaining_items.is_empty());
    }

    #[test]
    fn test_parse_verification_fallback() {
        let output = "Some random output";
        let vr = parse_verification(output).unwrap();
        assert!(!vr.goal_met);
    }
}