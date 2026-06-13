use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

pub struct TuiState {
    pub log_lines: Vec<(String, LogStyle)>,
    pub iteration: u32,
    pub max_iterations: u32,
    pub status: String,
    pub phase: String,
    pub elapsed: Duration,
    pub plan_summary: String,
    pub plan_tasks: Vec<String>,
    pub project_dir: String,
    pub agent_name: String,
    pub model_name: String,
    pub tokens_used: u64,
    pub tokens_max: u64,
    pub cost_dollars: f64,
    pub paused: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LogStyle {
    Normal,
    Info,
    Success,
    Error,
    Warning,
    Dim,
}

impl TuiState {
    fn new(max_iterations: u32) -> Self {
        TuiState {
            log_lines: Vec::new(),
            iteration: 0,
            max_iterations,
            status: "Ready".to_string(),
            phase: String::new(),
            elapsed: Duration::ZERO,
            plan_summary: String::new(),
            plan_tasks: Vec::new(),
            project_dir: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            agent_name: String::new(),
            model_name: String::new(),
            tokens_used: 0,
            tokens_max: 200000,
            cost_dollars: 0.0,
            paused: false,
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

    pub fn push_log(&self, line: String, style: LogStyle) {
        if let Ok(mut state) = self.state.lock() {
            state.log_lines.push((line, style));
            if state.log_lines.len() > 500 {
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

    pub fn set_plan(&self, summary: &str, tasks: &[String]) {
        if let Ok(mut state) = self.state.lock() {
            state.plan_summary = summary.to_string();
            state.plan_tasks = tasks.to_vec();
        }
    }

    pub fn set_agent_info(&self, agent: &str, model: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.agent_name = agent.to_string();
            state.model_name = model.to_string();
        }
    }

    pub fn set_tokens(&self, used: u64, max: u64) {
        if let Ok(mut state) = self.state.lock() {
            state.tokens_used = used;
            state.tokens_max = max;
        }
    }

    pub fn set_cost(&self, cost: f64) {
        if let Ok(mut state) = self.state.lock() {
            state.cost_dollars = cost;
        }
    }

    pub fn is_quit(&self) -> bool {
        self.quit.load(Ordering::Relaxed)
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}

pub fn run_tui(handle: &TuiHandle) -> Result<()> {
    enable_raw_mode()?;
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;

    let start = Instant::now();

    let result = run_tui_inner(&mut terminal, handle, &start);

    disable_raw_mode()?;
    ratatui::restore();

    // Print summary on exit
    if let Ok(state) = handle.state.lock() {
        println!("\n── Session Summary ──");
        for (line, _) in state.log_lines.iter() {
            println!("{}", line);
        }
    }

    result
}

fn run_tui_inner(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    handle: &TuiHandle,
    start: &Instant,
) -> Result<()> {
    let mut last_tick = Instant::now();

    loop {
        // Handle input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            handle.quit.store(true, Ordering::Relaxed);
                            break;
                        }
                        KeyCode::Char('p') => {
                            let was = handle.paused.fetch_xor(true, Ordering::Relaxed);
                            if let Ok(mut s) = handle.state.lock() {
                                s.paused = !was;
                            }
                            handle.push_log(
                                if was {
                                    "▶ Resumed".into()
                                } else {
                                    "⏸ Paused".into()
                                },
                                LogStyle::Info,
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        // Update elapsed and render
        let state = handle.state.lock().unwrap();
        let mut state = state;
        let elapsed = start.elapsed();
        state.elapsed = elapsed;

        terminal.draw(|f| render(f, &state))?;

        if last_tick.elapsed() > Duration::from_secs(1) {
            last_tick = Instant::now();
        }

        if handle.quit.load(Ordering::Relaxed) {
            break;
        }
    }
    Ok(())
}

fn render(frame: &mut Frame, state: &TuiState) {
    let size = frame.area();
    if size.width < 80 || size.height < 12 {
        let text = "Terminal too small — resize to at least 80x12";
        frame.render_widget(Paragraph::new(text), size);
        return;
    }

    // Main vertical layout: header | content | plan
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content (log + info)
            Constraint::Length(5), // Plan
        ])
        .split(size);

    // Header
    render_header(frame, main_chunks[0], state);

    // Content: horizontal split (log | info)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(main_chunks[1]);

    render_log(frame, content_chunks[0], state);
    render_info(frame, content_chunks[1], state);

    // Plan bottom
    render_plan(frame, main_chunks[2], state);
}

fn render_header(frame: &mut Frame, area: Rect, state: &TuiState) {
    let paused = state.paused;
    let title = format!("  ◉ OPENLOOP  v{}   ", env!("CARGO_PKG_VERSION"));
    let iteration_info = if state.iteration > 0 {
        format!(" Iteration {}/{} ", state.iteration, state.max_iterations)
    } else {
        " Ready ".to_string()
    };

    let mins = state.elapsed.as_secs() / 60;
    let secs = state.elapsed.as_secs() % 60;
    let time = format!(" {:02}:{:02} ", mins, secs);

    let phase = &state.phase;
    let phase_tag = if phase.is_empty() {
        " Waiting".to_string()
    } else {
        format!(" {}", phase)
    };

    // Use spans for colored header
    let spans = vec![
        Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(
            iteration_info,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(
            phase_tag,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(time, Style::default().fg(Color::Green)),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(" {} ", state.status),
            Style::default().fg(if paused { Color::Red } else { Color::White }),
        ),
    ];

    let header = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    frame.render_widget(header, area);
}

fn render_log(frame: &mut Frame, area: Rect, state: &TuiState) {
    let lines: Vec<Line> = state
        .log_lines
        .iter()
        .map(|(text, style)| {
            let color = match style {
                LogStyle::Normal => Color::White,
                LogStyle::Info => Color::Cyan,
                LogStyle::Success => Color::Green,
                LogStyle::Error => Color::Red,
                LogStyle::Warning => Color::Yellow,
                LogStyle::Dim => Color::DarkGray,
            };
            Line::from(Span::styled(text, Style::default().fg(color)))
        })
        .collect();

    // Show latest lines that fit
    let available = area.height as usize;
    let start = lines.len().saturating_sub(available);
    let visible: Vec<Line> = lines.into_iter().skip(start).collect();

    let log = Paragraph::new(visible)
        .block(
            Block::default()
                .title(" Agent Output ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Rgb(10, 10, 20)))
        .wrap(Wrap { trim: false });

    frame.render_widget(log, area);
}

fn render_info(frame: &mut Frame, area: Rect, state: &TuiState) {
    let info_lines = vec![
        Line::from(Span::styled(
            format!(" Project: {}", truncate_str(&state.project_dir, 25)),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(
            format!(" Agent: {}", state.agent_name),
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            format!(" Model: {}", truncate_str(&state.model_name, 25)),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(
            format!(" Tokens: {}/{}", state.tokens_used, state.tokens_max),
            Style::default().fg(Color::Yellow),
        )),
    ];

    let token_pct = if state.tokens_max > 0 {
        (state.tokens_used as f64 / state.tokens_max as f64).min(1.0)
    } else {
        0.0
    };
    let gauge_color = if token_pct < 0.5 {
        Color::Green
    } else if token_pct < 0.8 {
        Color::Yellow
    } else {
        Color::Red
    };

    let cost = format!(" Cost: ${:.4}", state.cost_dollars);
    let elapsed_m = state.elapsed.as_secs() / 60;
    let elapsed_s = state.elapsed.as_secs() % 60;
    let elapsed_str = format!(" Elapsed: {:02}:{:02}", elapsed_m, elapsed_s);

    let mut controls = vec![
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(&cost, Style::default().fg(Color::Green))),
        Line::from(Span::styled(
            &elapsed_str,
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(
            format!(" Iteration: {}/{}", state.iteration, state.max_iterations),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled(
            if state.paused {
                " ⏸ PAUSED"
            } else {
                " [p] pause"
            },
            Style::default().fg(if state.paused {
                Color::Red
            } else {
                Color::DarkGray
            }),
        )),
        Line::from(Span::styled(
            " [q] quit",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let mut all_lines = info_lines;
    all_lines.push(Line::from(Span::styled("", Style::default())));
    all_lines.push(Line::from(Span::styled(
        format!(" {:>6.1}%", token_pct * 100.0),
        Style::default().fg(gauge_color),
    )));
    all_lines.push(Line::from(Span::styled("", Style::default())));
    all_lines.append(&mut controls);

    // Layout: text on top, gauge below
    let info_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let info = Paragraph::new(all_lines)
        .block(
            Block::default()
                .title(" Info ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().bg(Color::Rgb(10, 10, 20)));

    frame.render_widget(info, info_chunks[0]);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Context ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .gauge_style(Style::default().fg(gauge_color).bg(Color::Rgb(30, 30, 40)))
        .percent((token_pct * 100.0) as u16);

    frame.render_widget(gauge, info_chunks[1]);
}

fn render_plan(frame: &mut Frame, area: Rect, state: &TuiState) {
    if state.plan_summary.is_empty() && state.plan_tasks.is_empty() {
        let empty = Paragraph::new("Waiting for next plan...")
            .block(
                Block::default()
                    .title(" Current Plan ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().bg(Color::Rgb(10, 10, 20)));
        frame.render_widget(empty, area);
        return;
    }

    let mut plan_lines = vec![Line::from(Span::styled(
        &state.plan_summary,
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ))];

    for task in &state.plan_tasks {
        plan_lines.push(Line::from(Span::styled(
            format!(" ▸ {}", task),
            Style::default().fg(Color::Cyan),
        )));
    }

    let plan = Paragraph::new(plan_lines)
        .block(
            Block::default()
                .title(" Current Plan ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .style(Style::default().bg(Color::Rgb(10, 10, 20)));

    frame.render_widget(plan, area);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
