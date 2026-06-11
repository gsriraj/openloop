use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result};

use super::types::{AgentConfig, AgentResult};

#[allow(dead_code)]
pub fn run_agent(agent: &AgentConfig, prompt: &str) -> Result<AgentResult> {
    let start = Instant::now();

    let output = Command::new(&agent.name)
        .arg("--model")
        .arg(&agent.model)
        .arg(prompt)
        .output()
        .with_context(|| format!("Failed to execute agent CLI '{}'", agent.name))?;

    let duration = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(AgentResult {
        agent: agent.name.clone(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration_ms: duration.as_millis() as u64,
    })
}

#[allow(dead_code)]
pub fn run_agent_with_stdin(agent: &AgentConfig, prompt: &str) -> Result<AgentResult> {
    let start = Instant::now();

    let mut child = Command::new(&agent.name)
        .arg("--model")
        .arg(&agent.model)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn agent CLI '{}'", agent.name))?;

    use std::io::Write;
    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(prompt.as_bytes())?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("Failed to collect output from '{}'", agent.name))?;

    let duration = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(AgentResult {
        agent: agent.name.clone(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration_ms: duration.as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_run_agent_not_found() {
        let agent = AgentConfig {
            name: "nonexistent-agent-cli".into(),
            model: "test-model".into(),
            model_config: HashMap::new(),
        };
        let result = run_agent(&agent, "hello");
        assert!(result.is_err());
    }
}
