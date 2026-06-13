use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use super::types::{AgentConfig, AgentResult};

const AGENT_TIMEOUT: Duration = Duration::from_secs(600);

pub fn run_agent(agent: &AgentConfig, prompt: &str) -> Result<AgentResult> {
    let start = Instant::now();
    let args = build_agent_args(agent, prompt);

    let output = Command::new(&agent.name)
        .args(&args)
        .output()
        .with_context(|| format!("Failed to execute agent CLI '{}'", agent.name))?;

    let duration = start.elapsed();
    Ok(result_from_output(agent, output, duration))
}

pub fn run_agent_with_stdin(agent: &AgentConfig, prompt: &str) -> Result<AgentResult> {
    let start = Instant::now();
    let args = build_agent_args(agent, prompt);

    let mut child = Command::new(&agent.name)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn agent CLI '{}'", agent.name))?;

    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes())?;
    }

    let output = wait_with_timeout(&mut child, AGENT_TIMEOUT)?;
    let duration = start.elapsed();
    Ok(result_from_output(agent, output, duration))
}

pub fn run_noninteractive(agent: &AgentConfig, prompt: &str) -> Result<String> {
    let args = build_agent_args(agent, prompt);

    let mut child = Command::new(&agent.name)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("Failed to spawn '{}'", agent.name))?;

    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes())?;
    }

    let output = wait_with_timeout(&mut child, AGENT_TIMEOUT)?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !stdout.trim().is_empty() {
        return Ok(stdout);
    }

    // Fallback: pass prompt as argument
    let fallback_args = build_agent_args(agent, prompt);
    let result2 = Command::new(&agent.name)
        .args(&fallback_args)
        .output()
        .with_context(|| format!("Failed to run '{}' with args", agent.name))?;
    let fallback_out = String::from_utf8_lossy(&result2.stdout).to_string();
    let fallback_err = String::from_utf8_lossy(&result2.stderr).to_string();
    // Some agents (copilot) output to stderr — check both
    let combined = if !fallback_out.trim().is_empty() {
        fallback_out
    } else {
        fallback_err
    };
    Ok(strip_ansi_escapes(&combined))
}

/// Remove ANSI escape sequences from output
fn strip_ansi_escapes(s: &str) -> String {
    s.chars()
        .fold((String::new(), false), |(mut acc, in_escape), c| {
            if in_escape {
                if c == 'm' || c == 'H' || c == 'J' || c == 'K' {
                    (acc, false)
                } else {
                    (acc, true)
                }
            } else if c == '\x1b' {
                (acc, true)
            } else {
                acc.push(c);
                (acc, false)
            }
        })
        .0
}

fn build_agent_args(agent: &AgentConfig, prompt: &str) -> Vec<String> {
    let name = agent.name.as_str();
    match name {
        "opencode" => vec![
            "run".to_string(),
            "-m".to_string(),
            agent.model.clone(),
            "--no-replay".to_string(),
            "--dangerously-skip-permissions".to_string(),
            prompt.to_string(),
        ],
        "copilot" => vec![
            "-p".to_string(),
            prompt.to_string(),
            "-m".to_string(),
            agent.model.clone(),
        ],
        "claude" => vec!["-p".to_string(), prompt.to_string()],
        _ => vec![
            "--model".to_string(),
            agent.model.clone(),
            prompt.to_string(),
        ],
    }
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<Output> {
    let start = Instant::now();
    let mut last_heartbeat = Instant::now();
    loop {
        if start.elapsed() > timeout {
            let _ = child.kill();
            anyhow::bail!("Agent timed out after {}s", timeout.as_secs());
        }
        if last_heartbeat.elapsed() > Duration::from_secs(30) {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().ok();
            last_heartbeat = Instant::now();
        }
        if let Ok(Some(_)) = child.try_wait() {
            // Child has exited — collect remaining output from pipes
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            use std::io::Read;
            if let Some(ref mut out) = child.stdout {
                let _ = out.read_to_end(&mut stdout);
            }
            if let Some(ref mut err) = child.stderr {
                let _ = err.read_to_end(&mut stderr);
            }
            // Wait to reap the process and get the exit status
            let output = child.wait()?;
            return Ok(std::process::Output {
                status: output,
                stdout,
                stderr,
            });
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn result_from_output(agent: &AgentConfig, output: Output, duration: Duration) -> AgentResult {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    AgentResult {
        agent: agent.name.clone(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration_ms: duration.as_millis() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_build_agent_args_opencode() {
        let agent = AgentConfig {
            name: "opencode".into(),
            model: "claude-sonnet-4-20250514".into(),
            model_config: HashMap::new(),
        };
        let args = build_agent_args(&agent, "hello");
        assert_eq!(args[0], "run");
        assert_eq!(args[1], "-m");
        assert_eq!(args[2], "claude-sonnet-4-20250514");
        assert!(args.last().unwrap() == "hello");
    }

    #[test]
    fn test_build_agent_args_copilot() {
        let agent = AgentConfig {
            name: "copilot".into(),
            model: "gpt-4o".into(),
            model_config: HashMap::new(),
        };
        let args = build_agent_args(&agent, "hello");
        assert_eq!(args[0], "-p");
        assert!(args.contains(&"gpt-4o".to_string()));
    }

    #[test]
    fn test_build_agent_args_claude() {
        let agent = AgentConfig {
            name: "claude".into(),
            model: "sonnet".into(),
            model_config: HashMap::new(),
        };
        let args = build_agent_args(&agent, "hello");
        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "hello");
    }

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

    #[test]
    fn test_strip_ansi_escapes() {
        let input = "\x1b[32mHello\x1b[0m World";
        assert_eq!(strip_ansi_escapes(input), "Hello World");
    }

    #[test]
    fn test_strip_ansi_escapes_clean() {
        let input = "Hello World";
        assert_eq!(strip_ansi_escapes(input), "Hello World");
    }
}
