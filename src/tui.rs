use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use colored::Colorize;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

pub struct TuiState {
    pub log_lines: Vec<String>,
    pub iteration: u32,
    pub max_iterations: u32,
    pub status: String,
    pub phase: String,
    pub elapsed: Duration,
}

impl TuiState {
    fn new(max_iterations: u32) -> Self {
        TuiState {
            log_lines: Vec::new(),
            iteration: 0,
            max_iterations,
            status: "Starting".to_string(),
            phase: String::new(),
            elapsed: Duration::ZERO,
        }
    }
}

pub struct TuiHandle {
    pub state: Arc<Mutex<TuiState>>,
    pub paused: Arc<AtomicBool>,
    pub quit: Arc<AtomicBool>,
}

impl TuiHandle {
    pub fn new(max_iterations: u32) -> Self {
        TuiHandle {
            state: Arc::new(Mutex::new(TuiState::new(max_iterations))),
            paused: Arc::new(AtomicBool::new(false)),
            quit: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn push_log(&self, line: String) {
        if let Ok(mut state) = self.state.lock() {
            state.log_lines.push(line);
            if state.log_lines.len() > 1000 {
                state.log_lines.remove(0);
            }
        }
    }

    pub fn set_status(&self, status: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.status = status.to_string();
        }
    }

    pub fn set_phase(&self, phase: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.phase = phase.to_string();
        }
    }

    pub fn set_iteration(&self, iteration: u32) {
        if let Ok(mut state) = self.state.lock() {
            state.iteration = iteration;
        }
    }

    pub fn set_elapsed(&self, elapsed: Duration) {
        if let Ok(mut state) = self.state.lock() {
            state.elapsed = elapsed;
        }
    }

    pub fn is_quit(&self) -> bool {
        self.quit.load(Ordering::Relaxed)
    }
}

pub fn run_tui(handle: &TuiHandle) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;

    let start = Instant::now();
    let mut frame_count = 0u64;

    let result = run_tui_inner(handle, &start, &mut frame_count);

    disable_raw_mode()?;
    stdout.execute(LeaveAlternateScreen)?;

    // Print accumulated log on exit
    if let Ok(state) = handle.state.lock() {
        println!("{}", "\n── Session Summary ──".bright_blue().bold());
        for line in state.log_lines.iter() {
            println!("{}", line);
        }
    }

    result
}

fn run_tui_inner(
    handle: &TuiHandle,
    start: &Instant,
    frame_count: &mut u64,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            handle.quit.store(true, Ordering::Relaxed);
                            break;
                        }
                        KeyCode::Char('p') => {
                            let was_paused = handle.paused.fetch_xor(true, Ordering::Relaxed);
                            handle.push_log(if was_paused {
                                "▶ Resumed".to_string()
                            } else {
                                "⏸ Paused — press 'p' to resume".to_string()
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        let state = handle.state.lock().unwrap();
        let elapsed = start.elapsed();

        render_frame(&state, elapsed, *frame_count)?;
        *frame_count += 1;

        if handle.quit.load(Ordering::Relaxed) {
            break;
        }
    }
    Ok(())
}

fn render_frame(
    state: &TuiState,
    elapsed: Duration,
    _frame: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let mut stdout = std::io::stdout();

    let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));

    // Clear screen
    crossterm::execute!(
        stdout,
        crossterm::cursor::MoveTo(0, 0),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;

    // Title bar
    let title = format!(
        " OpenLoop v{} — Iteration {}/{} ",
        env!("CARGO_PKG_VERSION"),
        state.iteration,
        state.max_iterations
    );
    let title_filled = format!("{:─<width$}", title, width = width as usize);
    writeln!(stdout, "{}", title_filled.bright_white().on_blue())?;

    // Status bar
    let mins = elapsed.as_secs() / 60;
    let secs = elapsed.as_secs() % 60;
    let pause_indicator = if handle_is_paused() {
        " ⏸ PAUSED"
    } else {
        ""
    };
    let status_line = format!(
        " {} │ {} │ {:02}:{:02} │ {} {}",
        state.phase,
        state.status,
        mins,
        secs,
        "Press 'p' pause, 'q' quit".dimmed(),
        pause_indicator,
    );
    writeln!(stdout, "{}", status_line)?;
    writeln!(stdout, "{}", "─".repeat(width as usize).dimmed())?;

    // Log area — show latest lines that fit
    let available = height.saturating_sub(4) as usize;
    let start_line = state.log_lines.len().saturating_sub(available);
    for line in state.log_lines.iter().skip(start_line).take(available) {
        writeln!(stdout, "{}", line)?;
    }

    stdout.flush()?;
    Ok(())
}

fn handle_is_paused() -> bool {
    false // placeholder — real state is in TuiHandle
}
