use std::path::Path;
use std::process::Command;

fn binary() -> String {
    std::env::var("CARGO_BIN_EXE_OPENLOOP").unwrap_or_else(|_| {
        let cwd = std::env::current_dir().expect("Failed to get current dir");
        cwd.join("target/debug/openloop")
            .to_string_lossy()
            .to_string()
    })
}

const MOCK_AGENT: &str = "./tests/fixtures/mock-agent.sh";

#[test]
fn test_help_flag() {
    let output = Command::new(binary())
        .arg("--help")
        .output()
        .expect("Failed to run openloop --help");

    assert!(output.status.success(), "openloop --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("openloop"), "Help should contain tool name");
    assert!(stdout.contains("--autopilot"), "Help should list flags");
}

#[test]
fn test_version_flag() {
    let output = Command::new(binary())
        .arg("--version")
        .output()
        .expect("Failed to run openloop --version");

    assert!(output.status.success(), "openloop --version should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0"), "Version should be 0.1.0");
}

#[test]
fn test_init_command() {
    let tmpdir = tempfile::tempdir().expect("Failed to create temp dir");
    let state_dir = tmpdir.path().join(".openloop");
    let goal_path = tmpdir.path().join("GOAL.md");

    let output = Command::new(binary())
        .arg("--init")
        .arg("--state-dir")
        .arg(state_dir.to_str().unwrap())
        .current_dir(tmpdir.path())
        .output()
        .expect("Failed to run openloop --init");

    assert!(output.status.success(), "openloop --init should exit 0");

    // Check config file created
    let config_path = state_dir.join("config.toml");
    assert!(config_path.exists(), "Config file should exist");

    // Check goal file created
    assert!(goal_path.exists(), "GOAL.md should exist");
}

#[test]
fn test_init_idempotent() {
    let tmpdir = tempfile::tempdir().expect("Failed to create temp dir");
    let state_dir = tmpdir.path().join(".openloop");

    // Run init twice
    Command::new(binary())
        .arg("--init")
        .arg("--state-dir")
        .arg(state_dir.to_str().unwrap())
        .current_dir(tmpdir.path())
        .output()
        .expect("First init should succeed");

    let output = Command::new(binary())
        .arg("--init")
        .arg("--state-dir")
        .arg(state_dir.to_str().unwrap())
        .current_dir(tmpdir.path())
        .output()
        .expect("Second init should succeed");

    assert!(output.status.success(), "Second --init should also exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("already exists"),
        "Re-init should note existing files"
    );
}

#[test]
fn test_status_no_state() {
    let tmpdir = tempfile::tempdir().expect("Failed to create temp dir");
    let state_dir = tmpdir.path().join(".openloop");

    let output = Command::new(binary())
        .arg("--status")
        .arg("--state-dir")
        .arg(state_dir.to_str().unwrap())
        .current_dir(tmpdir.path())
        .output()
        .expect("Failed to run openloop --status");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Combined output
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("No state file") || combined.contains("state.md"),
        "Should indicate no state file: {}",
        combined
    );
}

#[cfg(unix)]
#[test]
fn test_headless_with_mock_agent() {
    let tmpdir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create a minimal config
    let state_dir = tmpdir.path().join(".openloop");
    std::fs::create_dir_all(&state_dir).expect("Failed to create state dir");

    let config_content = format!(
        r#"
goal = "GOAL.md"
max_iterations = 1
autopilot = true
parallel = false

[agents]
enabled = ["{}"]

[agents.{}]
model = "test-model"
model_config = {{ verify_model = "test-model" }}

[state]
file = "state.md"
"#,
        MOCK_AGENT, MOCK_AGENT
    );
    std::fs::write(state_dir.join("config.toml"), &config_content).expect("Failed to write config");

    // Create a goal file
    let goal_content = "# Test Goal\n\n## Success Criteria\n\n- [ ] Something works\n";
    std::fs::write(tmpdir.path().join("GOAL.md"), goal_content).expect("Failed to write GOAL.md");

    // Create a symlink to the mock agent with a simple name (no slashes for TOML)
    let agent_link = tmpdir.path().join("mock-agent");
    let mock_path = Path::new(MOCK_AGENT);
    let abs_mock = std::fs::canonicalize(mock_path).expect("Failed to resolve mock agent path");
    std::os::unix::fs::symlink(&abs_mock, &agent_link).expect("Failed to create symlink");
    let agent_name = agent_link.to_str().unwrap();

    // Write config with the agent name (no slashes)
    let config_content = format!(
        r#"
goal = "GOAL.md"
max_iterations = 1
autopilot = true
parallel = false

[agents]
enabled = ["mock-agent"]

[agents.mock-agent]
model = "test-model"
model_config = {{ verify_model = "test-model" }}

[state]
file = "state.md"
"#,
    );
    std::fs::write(state_dir.join("config.toml"), &config_content).expect("Failed to write config");

    // Create a goal file
    let goal_content = "# Test Goal\n\n## Success Criteria\n\n- [ ] Something works\n";
    std::fs::write(tmpdir.path().join("GOAL.md"), goal_content).expect("Failed to write GOAL.md");

    let output = Command::new(binary())
        .arg("--agent-cli")
        .arg(agent_name)
        .arg("--goal")
        .arg(tmpdir.path().join("GOAL.md"))
        .arg("--autopilot")
        .arg("--max-iterations")
        .arg("1")
        .arg("--state-dir")
        .arg(state_dir.to_str().unwrap())
        .current_dir(tmpdir.path())
        .output()
        .expect("Failed to run openloop headless");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("Iteration") || output.status.success(),
        "Headless mode should run: exit={}, output={}",
        output.status,
        combined
    );
}
