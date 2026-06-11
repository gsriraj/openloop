use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
    pub model_config: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub agent: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

#[allow(dead_code)]
impl AgentResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlan {
    pub summary: String,
    pub sub_tasks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub goal_met: bool,
    pub reason: String,
    pub remaining_items: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_result_success() {
        let result = AgentResult {
            agent: "test".into(),
            exit_code: 0,
            stdout: "ok".into(),
            stderr: String::new(),
            duration_ms: 100,
        };
        assert!(result.success());
    }

    #[test]
    fn test_agent_result_failure() {
        let result = AgentResult {
            agent: "test".into(),
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".into(),
            duration_ms: 50,
        };
        assert!(!result.success());
    }
}