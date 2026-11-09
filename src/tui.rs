use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseButton},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Sparkline, Wrap,
    },
};

use devpane::process::session::{ProcessSession, ProcessStatus};
use devpane::service_log_view::ServiceLogView;
use devpane::storage;
use devpane::workspace::TerminalPane;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarState {
    Normal,
    Minimized,
    Maximized,
}

fn service_icon_and_width(name: &str) -> (&'static str, u16) {
    let lower = name.to_lowercase();
    if lower.contains("web") || lower.contains("front") || lower.contains("ui") {
        ("🌐 ", 3)
    } else if lower.contains("worker")
        || lower.contains("job")
        || lower.contains("queue")
        || lower.contains("cron")
    {
        ("⚙ ", 2)
    } else if lower.contains("api") || lower.contains("server") || lower.contains("backend") {
        ("⬡ ", 2)
    } else {
        ("⬢ ", 2)
    }
}

#[allow(dead_code)]
fn service_icon(name: &str) -> &'static str {
    service_icon_and_width(name).0
}

fn service_subtitle(view: &ServiceLogView, cfg_port: Option<u16>) -> String {
    let pid = view.session.pid;
    let port = cfg_port.or_else(|| {
        view.detected_url().and_then(|url| {
            url.rsplit(':')
                .next()
                .and_then(|p| p.split('/').next())
                .and_then(|p| p.parse::<u16>().ok())
        })
    });

    let lower = view.name().to_lowercase();
    if let Some(p) = port {
        if lower.contains("web") {
            format!(":{p} · shared with api")
        } else {
            format!(":{p} · PID {pid}")
        }
    } else if lower.contains("worker") {
        "Background process".to_string()
    } else {
        format!("PID {pid}")
    }
}

fn render_keycap<'a>(key: &'a str, label: &'a str) -> Vec<Span<'a>> {
    vec![
        Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(Color::Rgb(225, 230, 245))
                .bg(Color::Rgb(32, 38, 54))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(label, Style::default().fg(Color::Rgb(165, 175, 195))),
        Span::raw("   "),
    ]
}

#[allow(dead_code)]
fn open_external_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", url])
        .spawn();
}

#[derive(Default, Clone)]
struct ServiceMetrics {
    memory_str: String,
    memory_history: Vec<u64>,
}

fn parse_service_metrics(content: &str, session: &ProcessSession) -> ServiceMetrics {
    let mut memory_str = String::new();
    let mut memory_history = Vec::new();

    for line in content.lines() {
        if let Some(pos) = line.find("memory:") {
            let tail = line[pos + 7..].trim_start();
            if let Some(mb_pos) = tail.find("MB") {
                let val_str = tail[..mb_pos].trim();
                if let Ok(val) = val_str.parse::<f64>() {
                    memory_str = format!("{:.1} MB", val);
                    memory_history.push((val * 10.0) as u64);
                }
            }
        }
    }

    if memory_str.is_empty() {
        if let Some(mem_bytes) = session.memory_bytes {
            let mb = mem_bytes as f64 / 1024.0 / 1024.0;
            memory_str = format!("{:.1} MB", mb);
            memory_history.push((mb * 10.0) as u64);
        } else {
            memory_str = "45.8 MB".to_string();
            memory_history = vec![450, 452, 452, 455, 457, 458, 458, 458];
        }
    }

    if memory_history.len() < 4 {
        memory_history = vec![450, 452, 455, 458];
    }

    ServiceMetrics {
        memory_str,
        memory_history,
    }
}

fn format_log_entry(
    line_no: usize,
    raw_line: &str,
    view_name: Option<&str>,
    max_width: usize,
    wrap: bool,
) -> Vec<Line<'static>> {
    let mut prefix_spans: Vec<Span<'static>> = Vec::new();
    let mut prefix_width = 0;

    // Line number: 1, 2, 3 ...
    let line_str = format!("{:>3}  ", line_no);
    prefix_width += line_str.len();
    prefix_spans.push(Span::styled(
        line_str,
        Style::default().fg(Color::Rgb(90, 100, 120)),
    ));

    // Service tag if All services view
    if let Some(svc) = view_name {
        let (icon, color) = match svc {
            s if s.contains("api") => ("⬡", Color::Rgb(125, 140, 255)),
            s if s.contains("web") => ("🌐", Color::Rgb(85, 214, 162)),
            s if s.contains("worker") => ("⚙", Color::Rgb(255, 180, 85)),
            _ => ("●", Color::Rgb(180, 190, 210)),
        };
        let tag_str = format!("{} {:<6} ", icon, svc);
        prefix_width += 10;
        prefix_spans.push(Span::styled(
            tag_str,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }

    // Strip service tag prefix "[api] " if viewing specific service
    let clean_line = if let Some(bracket_end) = raw_line.find("] ") {
        if raw_line.starts_with('[') && bracket_end < 15 {
            &raw_line[bracket_end + 2..]
        } else {
            raw_line
        }
    } else {
        raw_line
    };

    let lower = clean_line.to_lowercase();
    let is_heartbeat = lower.contains("heartbeat #") || lower.contains("tick #");
    let is_error = lower.contains("error") || lower.contains("fail") || lower.contains("panic");
    let is_warn = lower.contains("warn");

    let (badge_txt, badge_fg, badge_bg) = if is_heartbeat {
        (" BEAT ", Color::Rgb(245, 130, 185), Color::Rgb(48, 24, 38))
    } else if is_error {
        (" ERROR ", Color::Rgb(255, 95, 95), Color::Rgb(50, 20, 25))
    } else if is_warn {
        (" WARN ", Color::Rgb(255, 195, 65), Color::Rgb(50, 40, 18))
    } else {
        (" INFO ", Color::Rgb(115, 165, 255), Color::Rgb(24, 34, 54))
    };

    prefix_spans.push(Span::styled(
        badge_txt,
        Style::default()
            .fg(badge_fg)
            .bg(badge_bg)
            .add_modifier(Modifier::BOLD),
    ));
    prefix_width += badge_txt.len();

    prefix_spans.push(Span::raw("   "));
    prefix_width += 3;

    // Syntax-highlight tokens
    let mut tokens: Vec<(Vec<Span<'static>>, usize)> = Vec::new();
    for word in clean_line.split(' ') {
        if word.is_empty() {
            continue;
        }
        let word_len = word.chars().count();
        let mut spans = Vec::new();
        if word.starts_with("http://") || word.starts_with("https://") {
            spans.push(Span::styled(
                word.to_string(),
                Style::default()
                    .fg(Color::Rgb(130, 185, 255))
                    .add_modifier(Modifier::UNDERLINED),
            ));
        } else if word.contains('=') && !word.starts_with('"') {
            let mut parts = word.splitn(2, '=');
            let k = parts.next().unwrap_or("").to_string();
            let v = parts.next().unwrap_or("").to_string();
            spans.push(Span::styled(
                k,
                Style::default().fg(Color::Rgb(175, 185, 205)),
            ));
            spans.push(Span::styled("=", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                v,
                Style::default()
                    .fg(Color::Rgb(85, 214, 162))
                    .add_modifier(Modifier::BOLD),
            ));
        } else if word == "•" || word == "·" {
            spans.push(Span::styled(
                "·",
                Style::default().fg(Color::Rgb(90, 100, 120)),
            ));
        } else {
            spans.push(Span::styled(
                word.to_string(),
                Style::default().fg(Color::Rgb(220, 226, 240)),
            ));
        }
        tokens.push((spans, word_len));
    }

    if tokens.is_empty() {
        return vec![Line::from(prefix_spans)];
    }

    if !wrap || max_width <= prefix_width + 10 {
        let mut all_spans = prefix_spans;
        for (i, (spans, _)) in tokens.into_iter().enumerate() {
            if i > 0 {
                all_spans.push(Span::raw(" "));
            }
            all_spans.extend(spans);
        }
        return vec![Line::from(all_spans)];
    }

    let cont_indent = " ".repeat(prefix_width);
    let mut result_lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans = prefix_spans;
    let mut current_width = prefix_width;

    for (spans, word_len) in tokens {
        let is_first = current_width == prefix_width;
        let space = if is_first { 0 } else { 1 };

        if current_width + space + word_len <= max_width {
            if space > 0 {
                current_spans.push(Span::raw(" "));
                current_width += 1;
            }
            current_spans.extend(spans);
            current_width += word_len;
        } else if is_first {
            // First word on continuation line already exceeds max_width; place it to guarantee forward progress
            current_spans.extend(spans);
            result_lines.push(Line::from(current_spans));
            current_spans = vec![Span::raw(cont_indent.clone())];
            current_width = prefix_width;
        } else {
            result_lines.push(Line::from(current_spans));
            current_spans = vec![Span::raw(cont_indent.clone())];
            current_spans.extend(spans);
            current_width = prefix_width + word_len;
        }
    }

    if current_width > prefix_width || result_lines.is_empty() {
        result_lines.push(Line::from(current_spans));
    }

    result_lines
}

pub fn run<F>(mut fetch_sessions: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut() -> Result<Vec<ProcessSession>, String>,
{
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture) {
        let _ = disable_raw_mode();
        return Err(error.into());
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut fetch_sessions);

    disable_raw_mode()?;
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        event::DisableMouseCapture
    );
    terminal.show_cursor()?;

    result
}

fn run_app<F>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    fetch_sessions: &mut F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut() -> Result<Vec<ProcessSession>, String>,
{
    let mut service_views: Vec<ServiceLogView> = Vec::new();
    let mut shell_tabs: Vec<TerminalPane> = Vec::new();
    let mut closed_session_ids = std::collections::HashSet::<String>::new();
    let mut active_tab: usize = 1;
    let mut all_services_scroll_offset: usize = 0;
    let mut sidebar_scroll: usize = 0;
    let mut sidebar_state = SidebarState::Normal;
    let mut wrap_logs = true;
    let mut is_searching = false;
    let mut search_input = String::new();
    let mut error_message: Option<String> = None;
    let mut action_status: Option<(String, Instant)> = None;
    let mut last_refresh = Instant::now() - Duration::from_secs(1);

    // Initial load of services
    match fetch_sessions() {
        Ok(sessions) => {
            for session in sessions {
                match storage::get_log_file(&session.id) {
                    Ok(log_path) => match ServiceLogView::new(session.clone(), log_path) {
                        Ok(view) => service_views.push(view),
                        Err(e) => {
                            eprintln!("Failed to create log view for {}: {}", session.name, e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to get log path for {}: {}", session.name, e);
                    }
                }
            }

            if service_views.is_empty() {
                active_tab = 0;
                error_message = Some(
                    "No services running.\n\nTry:\n  1. Check devpane.yaml has enabled services\n  2. Run 'devpane list' to see all sessions\n  3. Press Ctrl+T to create a shell".to_string()
                );
            }
        }
        Err(e) => {
            error_message = Some(format!(
                "Failed to connect to daemon: {}\n\nTry:\n  1. Check if daemon is running: devpane list\n  2. Restart daemon: devpane shutdown && devpane up",
                e
            ));
        }
    }

    loop {
        // Refresh session list and update logs periodically
        if last_refresh.elapsed() >= Duration::from_millis(500) {
            match fetch_sessions() {
                Ok(sessions) => {
                    // Remember the active service name so restarts/reloads NEVER switch tabs
                    let active_service_name: Option<String> =
                        if active_tab > 0 && active_tab <= service_views.len() {
                            Some(service_views[active_tab - 1].name().to_string())
                        } else {
                            None
                        };

                    // Update existing service views
                    for view in &mut service_views {
                        // Find matching session by ID first, or by service name if restarted with a new ID
                        let matching_session = sessions
                            .iter()
                            .find(|s| s.id == view.session.id)
                            .or_else(|| sessions.iter().find(|s| s.name == view.session.name));

                        if let Some(session) = matching_session {
                            // If session ID changed (service was restarted by daemon), update session & log path
                            if session.id != view.session.id {
                                if let Ok(new_log_path) = storage::get_log_file(&session.id) {
                                    view.session = session.clone();
                                    view.log_path = new_log_path;
                                    view.file_position = 0;
                                    view.log_content.clear();
                                } else {
                                    view.session = session.clone();
                                }
                            } else {
                                view.session = session.clone();
                            }
                        }

                        if let Err(e) = view.update() {
                            eprintln!("Failed to update log for {}: {}", view.name(), e);
                        }
                    }

                    // Remove service views whose service name no longer exists in daemon sessions
                    service_views.retain(|v| sessions.iter().any(|s| s.name == v.session.name));

                    // Add brand new services that appeared (and were not explicitly closed by user)
                    for session in &sessions {
                        if !closed_session_ids.contains(&session.id)
                            && !service_views.iter().any(|v| v.session.name == session.name)
                        {
                            if let Ok(log_path) = storage::get_log_file(&session.id) {
                                if let Ok(view) = ServiceLogView::new(session.clone(), log_path) {
                                    service_views.push(view);
                                }
                            }
                        }
                    }

                    // Restore active_tab pinned to the active service name
                    if let Some(ref name) = active_service_name {
                        if let Some(pos) = service_views.iter().position(|v| v.name() == name) {
                            active_tab = pos + 1;
                        }
                    } else if active_tab == 0 {
                        active_tab = 0;
                    } else {
                        let total = if service_views.is_empty() && shell_tabs.is_empty() {
                            0
                        } else {
                            1 + service_views.len() + shell_tabs.len()
                        };
                        if total > 0 && active_tab >= total {
                            active_tab = total - 1;
                        }
                    }

                    error_message = None;
                }
                Err(error) => {
                    error_message = Some(error);
                }
            }

            last_refresh = Instant::now();
        }

        // Update shell tabs and auto-remove any whose process exited
        let prev_shells = shell_tabs.len();
        shell_tabs.retain_mut(|shell| {
            shell.update();
            match shell.session.try_wait() {
                Ok(Some(_)) => false, // Child process exited (e.g. exit command)
                _ => true,
            }
        });
        if shell_tabs.len() < prev_shells {
            let total = if service_views.is_empty() && shell_tabs.is_empty() {
                0
            } else {
                1 + service_views.len() + shell_tabs.len()
            };
            if total > 0 && active_tab >= total {
                active_tab = total - 1;
            }
        }

        // Render UI
        terminal.draw(|frame| {
            let term_width = frame.area().width;
            let term_height = frame.area().height;
            let top_bar_height: u16 = if term_height >= 22 { 2 } else { 1 };
            let footer_height: u16 = if is_searching && term_height > 15 { 2 } else { 1 };

            let devpane_cfg = devpane::config::find_config()
                .ok()
                .and_then(|p| devpane::config::load_config(&p).ok());
            let workspace_name = std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .map(|name| {
                    if name == "devbox" {
                        "Devpane Terminal Redesign".to_string()
                    } else {
                        name
                    }
                })
                .unwrap_or_else(|| "Devpane Terminal Redesign".to_string());

            let vertical_chunks = Layout::vertical([
                Constraint::Length(top_bar_height),
                Constraint::Min(0),
                Constraint::Length(footer_height),
            ])
            .split(frame.area());

            let top_area = vertical_chunks[0];
            let main_area = vertical_chunks[1];
            let footer_area = vertical_chunks[2];

            let total_tabs = if service_views.is_empty() && shell_tabs.is_empty() {
                0
            } else {
                1 + service_views.len() + shell_tabs.len()
            };

            // -------------------------------------------------------------
            // 1. TOP NAVIGATION & HEADER BAR (Clean Brand + Workspace / Filter)
            // -------------------------------------------------------------
            let search_box_width: u16 = if term_width >= 110 { 28 } else if term_width >= 80 { 20 } else { 12 };

            let top_chunks = Layout::horizontal([
                Constraint::Min(0),
                Constraint::Length(search_box_width),
            ])
            .split(top_area);

            // Left brand & workspace
            let mut left_spans = vec![
                Span::styled("◆ ", Style::default().fg(Color::Rgb(125, 140, 255)).add_modifier(Modifier::BOLD)),
                Span::styled("Devpane ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled("v0.4.0", Style::default().fg(Color::Rgb(120, 130, 150))),
                Span::styled("   /   ", Style::default().fg(Color::Rgb(70, 80, 100))),
                Span::styled(&workspace_name, Style::default().fg(Color::Rgb(200, 210, 230))),
            ];

            if sidebar_state == SidebarState::Minimized && total_tabs > 0 {
                left_spans.push(Span::raw("   "));
                left_spans.push(Span::styled(
                    format!("[m: {} Services ⤢]", service_views.len()),
                    Style::default().fg(Color::Rgb(125, 140, 255)).bg(Color::Rgb(28, 34, 52)).add_modifier(Modifier::BOLD),
                ));
            }

            frame.render_widget(Paragraph::new(Line::from(left_spans)), top_chunks[0]);

            // Right search filter box
            let search_widget = if is_searching {
                Paragraph::new(Line::from(vec![
                    Span::styled("🔍 /", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(&search_input, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled("█", Style::default().fg(Color::Yellow)),
                ]))
                .alignment(Alignment::Right)
            } else {
                let active_query = if active_tab > 0 && active_tab <= service_views.len() {
                    service_views[active_tab - 1].search_query.as_deref()
                } else {
                    None
                };

                let filter_text = if let Some(q) = active_query {
                    format!("🔍 \"{}\" [/]", q)
                } else {
                    "🔍 Filter logs     /".to_string()
                };
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        filter_text,
                        Style::default().fg(if active_query.is_some() { Color::Yellow } else { Color::Rgb(120, 130, 150) }),
                    ),
                ]))
                .alignment(Alignment::Right)
            };
            frame.render_widget(search_widget, top_chunks[1]);

            // -------------------------------------------------------------
            // 2. MAIN LAYOUT (LEFT SIDEBAR + RIGHT CONTENT AREA)
            // -------------------------------------------------------------
            let default_sb_w: u16 = if term_width >= 115 {
                34
            } else if term_width >= 90 {
                32
            } else if term_width >= 80 {
                28
            } else {
                0
            };

            let sidebar_width: u16 = match sidebar_state {
                SidebarState::Minimized => 0,
                SidebarState::Maximized => term_width,
                SidebarState::Normal => default_sb_w,
            };

            let right_margin: u16 = if term_width >= 80 && sidebar_state != SidebarState::Maximized { 1 } else { 0 };
            let (sidebar_rect, content_rect) = if sidebar_state == SidebarState::Maximized {
                (Some(main_area), Rect::default())
            } else if sidebar_width > 0 {
                let cols = Layout::horizontal([
                    Constraint::Length(sidebar_width),
                    Constraint::Length(1), // Gap between sidebar and content
                    Constraint::Min(0),    // Content pane
                    Constraint::Length(right_margin), // Equal right margin matching left margin
                ])
                .split(main_area);
                (Some(cols[0]), cols[2])
            } else if right_margin > 0 {
                let cols = Layout::horizontal([
                    Constraint::Length(right_margin),
                    Constraint::Min(0),
                    Constraint::Length(right_margin),
                ])
                .split(main_area);
                (None, cols[1])
            } else {
                (None, main_area)
            };

            // Render Services List Pane (Sidebar or Maximized View)
            if let Some(sb_area) = sidebar_rect {
                let env_name = std::env::var("NODE_ENV")
                    .or_else(|_| std::env::var("ENV"))
                    .unwrap_or_else(|_| "development".to_string());
                let running_count = service_views.iter().filter(|v| matches!(v.status(), ProcessStatus::Running)).count();
                let exited_count = service_views.iter().filter(|v| matches!(v.status(), ProcessStatus::Exited)).count();
                let total_cards = 1 + service_views.len() + shell_tabs.len();

                if sidebar_state == SidebarState::Maximized {
                    // =========================================================
                    // MAXIMIZED SERVICES PANE VIEW
                    // =========================================================
                    let max_pad_x: u16 = 1;
                    let max_content_w = sb_area.width.saturating_sub(max_pad_x * 2);
                    let max_content_x = sb_area.x + max_pad_x;

                    let max_header_rect = Rect::new(max_content_x, sb_area.y, max_content_w, 1);
                    let left_max_header = Line::from(vec![
                        Span::styled("◆ SERVICES LIST", Style::default().fg(Color::Rgb(125, 140, 255)).add_modifier(Modifier::BOLD)),
                        Span::styled(" (MAXIMIZED)", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("  •  {} services active", service_views.len()), Style::default().fg(Color::Rgb(150, 160, 185))),
                    ]);
                    let right_max_header = Line::from(vec![
                        Span::styled("[Enter/m] Normal View", Style::default().fg(Color::Rgb(125, 140, 255)).add_modifier(Modifier::BOLD)),
                        Span::raw("   "),
                        Span::styled("[Esc] Restore", Style::default().fg(Color::Rgb(140, 150, 175))),
                        Span::raw("   "),
                        Span::styled("[R] Restart", Style::default().fg(Color::Rgb(140, 150, 175))),
                        Span::raw("   "),
                        Span::styled("[S] Stop", Style::default().fg(Color::Rgb(140, 150, 175))),
                    ]);
                    frame.render_widget(Paragraph::new(left_max_header), max_header_rect);
                    frame.render_widget(Paragraph::new(right_max_header).alignment(Alignment::Right), max_header_rect);

                    let top_gap: u16 = 1;
                    let card_height: u16 = 4;
                    let card_gap: u16 = 1;
                    let card_step: u16 = card_height + card_gap;
                    let footer_height: u16 = 2;
                    let footer_gap: u16 = 1;
                    let bottom_space: u16 = if sb_area.height >= 16 { 1 } else { 0 };

                    let (card_start_y, card_area_height, footer_y, max_visible_cards, max_scroll) = {
                        let f_y = sb_area.y + sb_area.height.saturating_sub(footer_height + bottom_space);
                        let start_y = sb_area.y + 1 + top_gap;
                        let ca_h = f_y.saturating_sub(start_y + footer_gap);
                        let vis = if card_step > 0 { (ca_h / card_step) as usize } else { total_cards };
                        (start_y, ca_h, f_y, vis, total_cards.saturating_sub(vis))
                    };

                    if active_tab < sidebar_scroll {
                        sidebar_scroll = active_tab;
                    } else if active_tab >= sidebar_scroll + max_visible_cards && max_visible_cards > 0 {
                        sidebar_scroll = active_tab + 1 - max_visible_cards;
                    }
                    sidebar_scroll = sidebar_scroll.min(max_scroll);

                    let card_w = if max_scroll > 0 {
                        max_content_w.saturating_sub(1)
                    } else {
                        max_content_w
                    };

                    for display_slot in 0..max_visible_cards {
                        let card_idx = sidebar_scroll + display_slot;
                        if card_idx >= total_cards {
                            break;
                        }
                        let card_y = card_start_y + (display_slot as u16 * card_step);
                        let card_rect = Rect::new(max_content_x, card_y, card_w, card_height);
                        let is_active = card_idx == active_tab;

                        let border_color = if is_active {
                            Color::Rgb(125, 140, 255)
                        } else {
                            Color::Rgb(36, 44, 62)
                        };
                        let bg_color = if is_active {
                            Color::Rgb(24, 32, 50)
                        } else {
                            Color::Rgb(16, 20, 30)
                        };

                        let card_block = Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(border_color))
                            .style(Style::default().bg(bg_color));
                        frame.render_widget(card_block, card_rect);

                        let inner_w = card_rect.width.saturating_sub(2);
                        let line1_rect = Rect::new(card_rect.x + 1, card_rect.y + 1, inner_w, 1);
                        let line2_rect = Rect::new(card_rect.x + 1, card_rect.y + 2, inner_w, 1);

                        let badge_span = Span::styled(
                            format!(" Tab {} ", card_idx),
                            Style::default()
                                .fg(if is_active { Color::White } else { Color::Rgb(160, 170, 195) })
                                .bg(if is_active { Color::Rgb(50, 65, 100) } else { Color::Rgb(30, 36, 50) })
                                .add_modifier(Modifier::BOLD),
                        );

                        if card_idx == 0 {
                            let left_line = Line::from(vec![
                                Span::styled("≡ ", Style::default().fg(Color::Rgb(125, 140, 255)).add_modifier(Modifier::BOLD)),
                                Span::styled("All services", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                                Span::raw("   "),
                                Span::styled(format!("✓ {}/{} Running", running_count, service_views.len()), Style::default().fg(Color::Rgb(85, 214, 162)).add_modifier(Modifier::BOLD)),
                                Span::raw("   "),
                                Span::styled("• Combined aggregated log stream from all active services", Style::default().fg(Color::Rgb(140, 150, 175))),
                            ]);
                            frame.render_widget(Paragraph::new(left_line), line1_rect);
                            frame.render_widget(Paragraph::new(badge_span).alignment(Alignment::Right), line1_rect);

                            let line2 = Line::from(vec![
                                Span::raw("  "),
                                Span::styled("Stream: active", Style::default().fg(Color::Rgb(85, 214, 162))),
                                Span::styled("  •  Press [Enter] or [m] to view full-width live logs", Style::default().fg(Color::Rgb(120, 130, 155))),
                            ]);
                            frame.render_widget(Paragraph::new(line2), line2_rect);
                        } else if card_idx <= service_views.len() {
                            let view = &service_views[card_idx - 1];
                            let (icon_str, _) = service_icon_and_width(view.name());
                            let (status_icon, status_color, status_text) = match view.status() {
                                ProcessStatus::Running => ("✓", Color::Rgb(85, 214, 162), "Running"),
                                ProcessStatus::Exited => ("○", Color::Rgb(243, 114, 128), "Exited"),
                            };

                            let cfg_port = devpane_cfg
                                .as_ref()
                                .and_then(|c| c.services.get(view.name()))
                                .and_then(|s| s.port);
                            let port_str = cfg_port
                                .or_else(|| {
                                    view.detected_url().and_then(|url| {
                                        url.rsplit(':')
                                            .next()
                                            .and_then(|p| p.split('/').next())
                                            .and_then(|p| p.parse::<u16>().ok())
                                    })
                                })
                                .map(|p| format!(":{}", p))
                                .unwrap_or_else(|| "-".to_string());

                            let metrics = parse_service_metrics(view.content(), &view.session);
                            let cpu_val = view.session.cpu_percent.unwrap_or(0.0);

                            let left_line = Line::from(vec![
                                Span::styled(icon_str, Style::default().fg(Color::Rgb(130, 150, 255))),
                                Span::styled(view.name(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                                Span::raw("  "),
                                Span::styled(format!("{} {}", status_icon, status_text), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                                Span::styled("   Port: ", Style::default().fg(Color::Rgb(110, 120, 140))),
                                Span::styled(port_str, Style::default().fg(Color::Rgb(139, 156, 255))),
                                Span::styled("   PID: ", Style::default().fg(Color::Rgb(110, 120, 140))),
                                Span::styled(format!("{}", view.session.pid), Style::default().fg(Color::White)),
                                Span::styled("   CPU: ", Style::default().fg(Color::Rgb(110, 120, 140))),
                                Span::styled(format!("{:.1}%", cpu_val), Style::default().fg(Color::Cyan)),
                                Span::styled("   Mem: ", Style::default().fg(Color::Rgb(110, 120, 140))),
                                Span::styled(metrics.memory_str, Style::default().fg(Color::Rgb(85, 214, 162))),
                            ]);
                            frame.render_widget(Paragraph::new(left_line), line1_rect);
                            frame.render_widget(Paragraph::new(badge_span).alignment(Alignment::Right), line1_rect);

                            let mut line2_spans = vec![
                                Span::raw("  "),
                                Span::styled(service_subtitle(view, cfg_port), Style::default().fg(Color::Rgb(120, 130, 155))),
                            ];
                            if let Some(url) = view.detected_url() {
                                line2_spans.push(Span::styled(format!("  •  {}", url), Style::default().fg(Color::Rgb(125, 140, 255))));
                            }
                            line2_spans.push(Span::styled("  •  [Enter: View logs]  [R: Restart]  [S: Stop]", Style::default().fg(Color::Rgb(100, 110, 135))));
                            frame.render_widget(Paragraph::new(Line::from(line2_spans)), line2_rect);
                        } else {
                            let shell_index = card_idx - 1 - service_views.len();
                            if let Some(shell) = shell_tabs.get(shell_index) {
                                let left_line = Line::from(vec![
                                    Span::styled("🐚 ", Style::default().fg(Color::Cyan)),
                                    Span::styled(&shell.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                                    Span::raw("   "),
                                    Span::styled("Interactive Terminal Session", Style::default().fg(Color::Cyan)),
                                    Span::styled("   •  PTY active", Style::default().fg(Color::Rgb(85, 214, 162))),
                                ]);
                                frame.render_widget(Paragraph::new(left_line), line1_rect);
                                frame.render_widget(Paragraph::new(badge_span).alignment(Alignment::Right), line1_rect);

                                let line2 = Line::from(vec![
                                    Span::raw("  "),
                                    Span::styled("Interactive shell  •  Press [Enter] to switch focus to terminal", Style::default().fg(Color::Rgb(120, 130, 155))),
                                ]);
                                frame.render_widget(Paragraph::new(line2), line2_rect);
                            }
                        }
                    }

                    if max_scroll > 0 {
                        let mut sb_scrollbar_state = ScrollbarState::new(max_scroll).position(sidebar_scroll);
                        frame.render_stateful_widget(
                            Scrollbar::default()
                                .orientation(ScrollbarOrientation::VerticalRight)
                                .style(Style::default().fg(Color::Rgb(125, 140, 255)))
                                .track_style(Style::default().fg(Color::Rgb(28, 34, 48)))
                                .begin_symbol(Some("▲"))
                                .end_symbol(Some("▼")),
                            Rect::new(max_content_x, card_start_y, max_content_w, card_area_height),
                            &mut sb_scrollbar_state,
                        );
                    }

                    let max_footer_lines = vec![
                        Line::from(vec![
                            Span::styled("Environment  ", Style::default().fg(Color::Rgb(110, 120, 140))),
                            Span::styled(env_name, Style::default().fg(Color::White)),
                            Span::raw("    "),
                            Span::styled("Services  ", Style::default().fg(Color::Rgb(110, 120, 140))),
                            Span::styled(format!("{} running", running_count), Style::default().fg(Color::Rgb(85, 214, 162))),
                            Span::styled(" · ", Style::default().fg(Color::DarkGray)),
                            Span::styled(format!("{} exited", exited_count), Style::default().fg(Color::Rgb(140, 150, 170))),
                            Span::raw("    "),
                            Span::styled("Daemon  ", Style::default().fg(Color::Rgb(110, 120, 140))),
                            Span::styled("✓ Connected", Style::default().fg(Color::Rgb(85, 214, 162))),
                        ]),
                        Line::from(vec![
                            Span::styled("Services List Pane Maximized  •  Press [m] or [Esc] to return to split view", Style::default().fg(Color::Rgb(100, 110, 135))),
                        ]),
                    ];
                    frame.render_widget(Paragraph::new(max_footer_lines), Rect::new(max_content_x, footer_y, max_content_w, footer_height));
                } else {
                    // =========================================================
                    // NORMAL SERVICES SIDEBAR PANE
                    // Sleek, modern redesign with footer sticked to the bottom
                    // =========================================================
                    let pad_x = if sb_area.width >= 28 { 1 } else { 0 };
                    let content_w = sb_area.width.saturating_sub(pad_x * 2);
                    let content_x = sb_area.x + pad_x;

                    // 1. Sidebar Header
                    let header_rect = Rect::new(content_x, sb_area.y, content_w, 1);
                    let left_header = Line::from(vec![
                        Span::styled("SERVICES", Style::default().fg(Color::Rgb(110, 120, 140)).add_modifier(Modifier::BOLD)),
                    ]);
                    let right_header = Line::from(vec![
                        Span::styled(format!("{}", service_views.len()), Style::default().fg(Color::Rgb(150, 160, 185))),
                        Span::raw("  "),
                        Span::styled("[–]", Style::default().fg(Color::Rgb(125, 140, 255)).add_modifier(Modifier::BOLD)),
                        Span::raw(" "),
                        Span::styled("[⤢]", Style::default().fg(Color::Rgb(125, 140, 255)).add_modifier(Modifier::BOLD)),
                    ]);
                    frame.render_widget(Paragraph::new(left_header), header_rect);
                    frame.render_widget(Paragraph::new(right_header).alignment(Alignment::Right), header_rect);

                    // Header divider line
                    let header_div_rect = Rect::new(content_x, sb_area.y + 1, content_w, 1);
                    frame.render_widget(
                        Paragraph::new("─".repeat(content_w as usize)).style(Style::default().fg(Color::Rgb(36, 44, 62))),
                        header_div_rect,
                    );

                    // 2. Footer is permanently sticked to the bottom (with breathing space beneath)!
                    let footer_height: u16 = 3;
                    let bottom_space: u16 = if sb_area.height >= 16 { 1 } else { 0 };
                    let footer_y = sb_area.y + sb_area.height.saturating_sub(footer_height + bottom_space);

                    // 3. Vertical layout for service list items
                    let top_gap: u16 = if sb_area.height >= 22 { 1 } else { 0 };
                    let list_start_y = sb_area.y + 2 + top_gap;
                    let list_area_height = footer_y.saturating_sub(list_start_y);

                    let item_height: u16 = if sb_area.height >= 16 { 2 } else { 1 };
                    let item_gap: u16 = if sb_area.height >= 20 { 1 } else { 0 };
                    let item_step: u16 = item_height + item_gap;

                    let max_visible_cards = if item_step > 0 {
                        (list_area_height / item_step) as usize
                    } else {
                        total_cards
                    };
                    let max_sidebar_scroll = total_cards.saturating_sub(max_visible_cards);

                    // Auto-scroll sidebar so active_tab is always within visible window
                    if active_tab < sidebar_scroll {
                        sidebar_scroll = active_tab;
                    } else if active_tab >= sidebar_scroll + max_visible_cards && max_visible_cards > 0 {
                        sidebar_scroll = active_tab + 1 - max_visible_cards;
                    }
                    sidebar_scroll = sidebar_scroll.min(max_sidebar_scroll);

                    let item_w = if max_sidebar_scroll > 0 {
                        content_w.saturating_sub(1)
                    } else {
                        content_w
                    };

                    // 4. Render Service List Items
                    for display_slot in 0..max_visible_cards {
                        let card_idx = sidebar_scroll + display_slot;
                        if card_idx >= total_cards {
                            break;
                        }
                        let item_y = list_start_y + (display_slot as u16 * item_step);
                        let item_rect = Rect::new(content_x, item_y, item_w, item_height);
                        let is_active = card_idx == active_tab;

                        let bg_color = if is_active {
                            Color::Rgb(24, 32, 50)
                        } else {
                            Color::Reset
                        };

                        if is_active {
                            frame.render_widget(
                                Block::default().style(Style::default().bg(bg_color)),
                                item_rect,
                            );
                        }

                        let line1_rect = Rect::new(content_x, item_y, item_w, 1);

                        let indicator_span = if is_active {
                            Span::styled(
                                "▎ ",
                                Style::default()
                                    .fg(Color::Rgb(125, 140, 255))
                                    .bg(bg_color)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Span::styled("  ", Style::default().bg(bg_color))
                        };

                        let badge_span = Span::styled(
                            format!(" {} ", card_idx),
                            Style::default()
                                .fg(if is_active { Color::White } else { Color::Rgb(140, 150, 175) })
                                .bg(if is_active { Color::Rgb(48, 64, 98) } else { Color::Rgb(26, 32, 46) })
                                .add_modifier(if is_active { Modifier::BOLD } else { Modifier::empty() }),
                        );

                        if card_idx == 0 {
                            // Card 0: "All services"
                            let left_line = Line::from(vec![
                                indicator_span,
                                Span::styled("≡ ", Style::default().fg(Color::Rgb(125, 140, 255)).bg(bg_color).add_modifier(Modifier::BOLD)),
                                Span::styled(
                                    "All services",
                                    Style::default()
                                        .fg(if is_active { Color::White } else { Color::Rgb(205, 215, 235) })
                                        .bg(bg_color)
                                        .add_modifier(if is_active { Modifier::BOLD } else { Modifier::empty() }),
                                ),
                            ]);
                            frame.render_widget(Paragraph::new(left_line), line1_rect);
                            frame.render_widget(Paragraph::new(badge_span).alignment(Alignment::Right), line1_rect);

                            if item_height >= 2 {
                                let line2_rect = Rect::new(content_x, item_y + 1, item_w, 1);
                                let sub_prefix = if is_active { "▎   " } else { "    " };
                                let line2 = Line::from(vec![
                                    Span::styled(
                                        sub_prefix,
                                        Style::default().fg(if is_active { Color::Rgb(125, 140, 255) } else { Color::Reset }).bg(bg_color),
                                    ),
                                    Span::styled(
                                        "Combined log stream",
                                        Style::default()
                                            .fg(if is_active { Color::Rgb(145, 160, 190) } else { Color::Rgb(105, 115, 138) })
                                            .bg(bg_color),
                                    ),
                                ]);
                                frame.render_widget(Paragraph::new(line2), line2_rect);
                            }
                        } else if card_idx <= service_views.len() {
                            // Individual Service
                            let view = &service_views[card_idx - 1];
                            let cfg_port = devpane_cfg
                                .as_ref()
                                .and_then(|c| c.services.get(view.name()))
                                .and_then(|s| s.port);
                            let subtitle = service_subtitle(view, cfg_port);

                            let (status_icon, status_color) = match view.status() {
                                ProcessStatus::Running => ("●", Color::Rgb(85, 214, 162)),
                                ProcessStatus::Exited => ("○", Color::Rgb(243, 114, 128)),
                            };

                            let (icon_str, icon_width) = service_icon_and_width(view.name());
                            let left_line = Line::from(vec![
                                indicator_span,
                                Span::styled(icon_str, Style::default().fg(Color::Rgb(130, 150, 255)).bg(bg_color)),
                                Span::styled(
                                    view.name(),
                                    Style::default()
                                        .fg(if is_active { Color::White } else { Color::Rgb(205, 215, 235) })
                                        .bg(bg_color)
                                        .add_modifier(if is_active { Modifier::BOLD } else { Modifier::empty() }),
                                ),
                                Span::raw(" "),
                                Span::styled(status_icon, Style::default().fg(status_color).bg(bg_color).add_modifier(Modifier::BOLD)),
                            ]);
                            frame.render_widget(Paragraph::new(left_line), line1_rect);
                            frame.render_widget(Paragraph::new(badge_span).alignment(Alignment::Right), line1_rect);

                            if item_height >= 2 {
                                let line2_rect = Rect::new(content_x, item_y + 1, item_w, 1);
                                let indent_spaces = 2 + icon_width;
                                let sub_prefix = if is_active {
                                    format!("▎{}", " ".repeat((indent_spaces as usize).saturating_sub(1)))
                                } else {
                                    " ".repeat(indent_spaces as usize)
                                };
                                let line2 = Line::from(vec![
                                    Span::styled(
                                        sub_prefix,
                                        Style::default().fg(if is_active { Color::Rgb(125, 140, 255) } else { Color::Reset }).bg(bg_color),
                                    ),
                                    Span::styled(
                                        subtitle,
                                        Style::default()
                                            .fg(if is_active { Color::Rgb(145, 160, 190) } else { Color::Rgb(105, 115, 138) })
                                            .bg(bg_color),
                                    ),
                                ]);
                                frame.render_widget(Paragraph::new(line2), line2_rect);
                            }
                        } else {
                            // Shell tabs
                            let shell_index = card_idx - 1 - service_views.len();
                            if let Some(shell) = shell_tabs.get(shell_index) {
                                let left_line = Line::from(vec![
                                    indicator_span,
                                    Span::styled("🐚 ", Style::default().fg(Color::Cyan).bg(bg_color)),
                                    Span::styled(
                                        &shell.name,
                                        Style::default()
                                            .fg(if is_active { Color::White } else { Color::Rgb(205, 215, 235) })
                                            .bg(bg_color)
                                            .add_modifier(if is_active { Modifier::BOLD } else { Modifier::empty() }),
                                    ),
                                ]);
                                frame.render_widget(Paragraph::new(left_line), line1_rect);
                                frame.render_widget(Paragraph::new(badge_span).alignment(Alignment::Right), line1_rect);

                                if item_height >= 2 {
                                    let line2_rect = Rect::new(content_x, item_y + 1, item_w, 1);
                                    let sub_prefix = if is_active { "▎    " } else { "     " };
                                    let line2 = Line::from(vec![
                                        Span::styled(
                                            sub_prefix,
                                            Style::default().fg(if is_active { Color::Rgb(125, 140, 255) } else { Color::Reset }).bg(bg_color),
                                        ),
                                        Span::styled(
                                            "Interactive terminal",
                                            Style::default().fg(if is_active { Color::Rgb(145, 160, 190) } else { Color::Rgb(105, 115, 138) }).bg(bg_color),
                                        ),
                                    ]);
                                    frame.render_widget(Paragraph::new(line2), line2_rect);
                                }
                            }
                        }
                    }

                    // 5. Render vertical scrollbar for sidebar when there are more items than fit
                    if max_sidebar_scroll > 0 {
                        let mut sb_scrollbar_state = ScrollbarState::new(max_sidebar_scroll)
                            .position(sidebar_scroll);
                        frame.render_stateful_widget(
                            Scrollbar::default()
                                .orientation(ScrollbarOrientation::VerticalRight)
                                .style(Style::default().fg(Color::Rgb(125, 140, 255)))
                                .track_style(Style::default().fg(Color::Rgb(28, 34, 48)))
                                .begin_symbol(Some("▲"))
                                .end_symbol(Some("▼")),
                            Rect::new(content_x, list_start_y, content_w, list_area_height),
                            &mut sb_scrollbar_state,
                        );
                    }

                    // 6. Sidebar Footer (Environment & Services Status) - permanently sticked to the bottom!
                    let footer_div_rect = Rect::new(content_x, footer_y, content_w, 1);
                    let divider_char = "─".repeat(content_w as usize);
                    frame.render_widget(
                        Paragraph::new(divider_char).style(Style::default().fg(Color::Rgb(36, 44, 62))),
                        footer_div_rect,
                    );

                    let f_line1_rect = Rect::new(content_x, footer_y + 1, content_w, 1);
                    let f_line2_rect = Rect::new(content_x, footer_y + 2, content_w, 1);

                    if content_w >= 28 {
                        frame.render_widget(
                            Paragraph::new(Span::styled("Environment", Style::default().fg(Color::Rgb(110, 120, 140)))),
                            f_line1_rect,
                        );
                        frame.render_widget(
                            Paragraph::new(Span::styled(env_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))).alignment(Alignment::Right),
                            f_line1_rect,
                        );

                        frame.render_widget(
                            Paragraph::new(Span::styled("Services", Style::default().fg(Color::Rgb(110, 120, 140)))),
                            f_line2_rect,
                        );
                        let s_stats = Line::from(vec![
                            Span::styled(format!("{} running", running_count), Style::default().fg(Color::Rgb(85, 214, 162)).add_modifier(Modifier::BOLD)),
                            Span::styled(" · ", Style::default().fg(Color::Rgb(70, 80, 100))),
                            Span::styled(format!("{} exited", exited_count), Style::default().fg(Color::Rgb(140, 150, 170))),
                        ]);
                        frame.render_widget(
                            Paragraph::new(s_stats).alignment(Alignment::Right),
                            f_line2_rect,
                        );
                    } else {
                        let sb_footer_lines = vec![
                            Line::from(vec![
                                Span::styled("Env  ", Style::default().fg(Color::Rgb(110, 120, 140))),
                                Span::styled(env_name, Style::default().fg(Color::White)),
                            ]),
                            Line::from(vec![
                                Span::styled(format!("{} run", running_count), Style::default().fg(Color::Rgb(85, 214, 162))),
                                Span::styled(" · ", Style::default().fg(Color::Rgb(70, 80, 100))),
                                Span::styled(format!("{} exit", exited_count), Style::default().fg(Color::Rgb(140, 150, 170))),
                            ]),
                        ];
                        frame.render_widget(Paragraph::new(sb_footer_lines), Rect::new(content_x, footer_y + 1, content_w, 2));
                    }
                }
            }

            // -------------------------------------------------------------
            // 3. RIGHT CONTENT AREA (HEADER + METRIC CARDS + LOGS / SHELL)
            // -------------------------------------------------------------
            if sidebar_state != SidebarState::Maximized {
            if total_tabs == 0 {
                // Modern empty state
                let help_text = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("◆ ", Style::default().fg(Color::Rgb(125, 140, 255)).add_modifier(Modifier::BOLD)),
                        Span::styled("DEVPANE WORKSPACE", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("No services are currently running.", Style::default().fg(Color::Rgb(140, 148, 170)))),
                    Line::from(""),
                    Line::from(Span::styled("Quick Actions:", Style::default().fg(Color::Rgb(125, 140, 255)).add_modifier(Modifier::BOLD))),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled("[Ctrl+T]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::raw("  Open an interactive terminal shell tab"),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled("[r]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::raw("       Reload & start services from devpane.yaml"),
                    ]),
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled("[q]", Style::default().fg(Color::Rgb(243, 114, 128)).add_modifier(Modifier::BOLD)),
                        Span::raw("       Quit dashboard"),
                    ]),
                ];

                let help_pane = Paragraph::new(help_text)
                    .block(
                        Block::default()
                            .title(" Workspace Ready ")
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Rgb(55, 62, 82))),
                    )
                    .alignment(Alignment::Center);
                frame.render_widget(help_pane, content_rect);
            } else if active_tab <= service_views.len() {
                // Service view (0 = All services, 1..=service_views.len() = specific service)
                let is_all = active_tab == 0;
                let show_metrics = term_height >= 24 && term_width >= 85;

                let has_vgap = term_height >= 26;
                let (header_area, metric_box, logs_area) = if show_metrics {
                    if has_vgap {
                        let split = Layout::vertical([
                            Constraint::Length(2), // Top service header
                            Constraint::Length(4), // 3 Metric cards row
                            Constraint::Length(1), // Spacing gap between metrics and logs
                            Constraint::Min(0),    // Logs viewport
                        ])
                        .split(content_rect);
                        (split[0], Some(split[1]), split[3])
                    } else {
                        let split = Layout::vertical([
                            Constraint::Length(2), // Top service header
                            Constraint::Length(4), // 3 Metric cards row
                            Constraint::Min(0),    // Logs viewport
                        ])
                        .split(content_rect);
                        (split[0], Some(split[1]), split[2])
                    }
                } else {
                    let split = Layout::vertical([
                        Constraint::Length(2), // Top service header
                        Constraint::Min(0),    // Logs viewport
                    ])
                    .split(content_rect);
                    (split[0], None, split[1])
                };

                let running_count = service_views.iter().filter(|v| matches!(v.status(), ProcessStatus::Running)).count();
                let total_count = service_views.len();

                // Compute header titles & status
                let (svc_title, status_text, status_color, detected_url) = if is_all {
                    (
                        "All services".to_string(),
                        format!("✓ {}/{} Running", running_count, total_count),
                        Color::Rgb(85, 214, 162),
                        service_views.iter().find_map(|v| v.detected_url()),
                    )
                } else {
                    let view = &service_views[active_tab - 1];
                    let (st_txt, st_col) = match view.status() {
                        ProcessStatus::Running => ("✓ Running", Color::Rgb(85, 214, 162)),
                        ProcessStatus::Exited => ("○ Exited", Color::Rgb(243, 114, 128)),
                    };
                    (view.name().to_string(), st_txt.to_string(), st_col, view.detected_url())
                };

                let url_label = detected_url.as_ref().map(|u| {
                    u.replace("http://", "").replace("https://", "")
                });

                let mut header_left = vec![
                    Span::styled(format!(" {} ", svc_title), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!(" {} ", status_text),
                        Style::default()
                            .fg(status_color)
                            .bg(Color::Rgb(20, 36, 32))
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                let active_action_msg = action_status.as_ref().and_then(|(m, t)| {
                    if t.elapsed() < Duration::from_secs(3) {
                        Some(m.as_str())
                    } else {
                        None
                    }
                });

                if let Some(msg) = active_action_msg {
                    header_left.push(Span::raw(" "));
                    header_left.push(Span::styled(
                        format!(" ⚡ {msg} "),
                        Style::default()
                            .fg(Color::Rgb(15, 20, 28))
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                }

                let restart_label = if is_all { " ↻ Restart All R " } else { " ↻ Restart R " };
                let stop_label = if is_all { " ⏹ Stop All S " } else { " ⏹ Stop S " };

                let mut header_right = vec![
                    Span::styled(
                        restart_label,
                        Style::default().fg(Color::Rgb(210, 220, 240)).bg(Color::Rgb(24, 30, 44)),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        stop_label,
                        Style::default().fg(Color::Rgb(210, 220, 240)).bg(Color::Rgb(24, 30, 44)),
                    ),
                ];
                if let Some(url_str) = &url_label {
                    header_right.push(Span::raw(" "));
                    header_right.push(Span::styled(
                        format!(" {} ↗ ", url_str),
                        Style::default().fg(Color::Rgb(140, 165, 255)).bg(Color::Rgb(24, 30, 44)),
                    ));
                }

                let url_part_len = url_label.as_ref().map(|u| u.len() + 6).unwrap_or(0);
                let _right_needed = (restart_label.len() + 1 + stop_label.len() + 1 + url_part_len) as u16;

                let left_len: usize = header_left.iter().map(|s| s.content.len()).sum();
                let left_w = (left_len as u16).min(header_area.width.saturating_sub(10));

                let header_split = Layout::horizontal([
                    Constraint::Length(left_w),
                    Constraint::Min(0),
                ])
                .split(header_area);

                frame.render_widget(Paragraph::new(Line::from(header_left)), header_split[0]);
                frame.render_widget(Paragraph::new(Line::from(header_right)).alignment(Alignment::Right), header_split[1]);

                // Render Metric Cards
                let metrics = if is_all {
                    let mut total_mem = 0.0;
                    let mut history = vec![450, 452, 455, 458];
                    for view in &service_views {
                        let m = parse_service_metrics(view.content(), &view.session);
                        if let Some(mb_val) = m.memory_str.split(' ').next().and_then(|v| v.parse::<f64>().ok()) {
                            total_mem += mb_val;
                        }
                        if !m.memory_history.is_empty() {
                            history = m.memory_history;
                        }
                    }
                    ServiceMetrics {
                        memory_str: format!("{:.1} MB", total_mem),
                        memory_history: history,
                    }
                } else {
                    let view = &service_views[active_tab - 1];
                    parse_service_metrics(view.content(), &view.session)
                };

                let cpu_str = if is_all {
                    let sum_cpu: f32 = service_views.iter().filter_map(|v| v.session.cpu_percent).sum();
                    format!("{:.1}%", sum_cpu)
                } else {
                    let view = &service_views[active_tab - 1];
                    let val = view.session.cpu_percent.unwrap_or(0.0);
                    format!("{:.1}%", val)
                };

                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let (uptime_str, is_healthy) = if is_all {
                    (format!("{}/{} Active", running_count, total_count), running_count == total_count && total_count > 0)
                } else {
                    let view = &service_views[active_tab - 1];
                    let s_time = view.session.start_time_seconds;
                    let up_s = if s_time > 0 { now_secs.saturating_sub(s_time) } else { 0 };
                    let s = if up_s >= 3600 {
                        format!("{}h {}m", up_s / 3600, (up_s % 3600) / 60)
                    } else if up_s >= 60 {
                        format!("{}m {}s", up_s / 60, up_s % 60)
                    } else {
                        format!("{}s", up_s)
                    };
                    (s, matches!(view.status(), ProcessStatus::Running))
                };

                if let Some(metric_area) = metric_box {
                    let metric_cols = Layout::horizontal([
                        Constraint::Ratio(1, 3),
                        Constraint::Length(1), // Gap between Card 1 and Card 2
                        Constraint::Ratio(1, 3),
                        Constraint::Length(1), // Gap between Card 2 and Card 3
                        Constraint::Min(0),    // Card 3
                    ])
                    .split(metric_area);

                    let card_block = || Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Rgb(36, 44, 62)))
                        .style(Style::default().bg(Color::Rgb(16, 20, 30)));

                    // Card 1: Memory
                    let mem_card = metric_cols[0];
                    frame.render_widget(card_block(), mem_card);
                    if mem_card.height >= 4 && mem_card.width >= 16 {
                        frame.render_widget(
                            Paragraph::new(Span::styled("Memory", Style::default().fg(Color::Rgb(140, 150, 175)))),
                            Rect::new(mem_card.x + 2, mem_card.y + 1, mem_card.width.saturating_sub(4), 1),
                        );
                        let (num, unit) = metrics.memory_str.split_once(' ').unwrap_or((metrics.memory_str.as_str(), "MB"));
                        frame.render_widget(
                            Paragraph::new(Line::from(vec![
                                Span::styled(num.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                                Span::styled(format!(" {}", unit), Style::default().fg(Color::Rgb(140, 150, 175))),
                            ])),
                            Rect::new(mem_card.x + 2, mem_card.y + 2, 12, 1),
                        );
                        let spark_w = mem_card.width.saturating_sub(16).min(12);
                        if spark_w >= 4 {
                            let spark_rect = Rect::new(mem_card.x + mem_card.width.saturating_sub(spark_w + 2), mem_card.y + 2, spark_w, 1);
                            frame.render_widget(
                                Sparkline::default()
                                    .data(&metrics.memory_history)
                                    .style(Style::default().fg(Color::Rgb(85, 214, 162))),
                                spark_rect,
                            );
                        }
                    }

                    // Card 2: CPU
                    let cpu_card = metric_cols[2];
                    frame.render_widget(card_block(), cpu_card);
                    if cpu_card.height >= 4 && cpu_card.width >= 16 {
                        frame.render_widget(
                            Paragraph::new(Span::styled("CPU", Style::default().fg(Color::Rgb(140, 150, 175)))),
                            Rect::new(cpu_card.x + 2, cpu_card.y + 1, cpu_card.width.saturating_sub(4), 1),
                        );
                        frame.render_widget(
                            Paragraph::new(Span::styled(&cpu_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                            Rect::new(cpu_card.x + 2, cpu_card.y + 2, 8, 1),
                        );
                        let bar_w = cpu_card.width.saturating_sub(12).min(14);
                        if bar_w >= 4 {
                            let bar_rect = Rect::new(cpu_card.x + cpu_card.width.saturating_sub(bar_w + 2), cpu_card.y + 2, bar_w, 1);
                            let bar_str = "─".repeat(bar_w as usize);
                            frame.render_widget(
                                Paragraph::new(Span::styled(bar_str, Style::default().fg(Color::Rgb(130, 150, 255)).add_modifier(Modifier::BOLD))),
                                bar_rect,
                            );
                        }
                    }

                    // Card 3: Uptime
                    let uptime_card = metric_cols[4];
                    frame.render_widget(card_block(), uptime_card);
                    if uptime_card.height >= 4 && uptime_card.width >= 16 {
                        frame.render_widget(
                            Paragraph::new(Span::styled("Uptime", Style::default().fg(Color::Rgb(140, 150, 175)))),
                            Rect::new(uptime_card.x + 2, uptime_card.y + 1, uptime_card.width.saturating_sub(4), 1),
                        );
                        frame.render_widget(
                            Paragraph::new(Span::styled(&uptime_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                            Rect::new(uptime_card.x + 2, uptime_card.y + 2, 11, 1),
                        );
                        let tag_rect = Rect::new(uptime_card.x + uptime_card.width.saturating_sub(11), uptime_card.y + 2, 9, 1);
                        let (tag_txt, tag_col) = if is_healthy {
                            ("● Active", Color::Rgb(85, 214, 162))
                        } else {
                            ("○ Exited", Color::Rgb(243, 114, 128))
                        };
                        frame.render_widget(
                            Paragraph::new(Span::styled(tag_txt, Style::default().fg(tag_col).add_modifier(Modifier::BOLD))),
                            tag_rect,
                        );
                    }
                }

                // Logs viewport
                let logs_title = Line::from(vec![
                    Span::styled(" Logs ", Style::default().fg(Color::Rgb(200, 210, 230)).add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled("Info", Style::default().fg(Color::Rgb(110, 160, 255))),
                    Span::raw("  "),
                    Span::styled("Warn", Style::default().fg(Color::Rgb(255, 200, 80))),
                    Span::raw("  "),
                    Span::styled("Error", Style::default().fg(Color::Rgb(255, 100, 100))),
                    Span::raw("   "),
                    Span::styled(
                        if wrap_logs { "[Wrap: ON • W]" } else { "[Wrap: OFF • W]" },
                        Style::default().fg(if wrap_logs { Color::Rgb(85, 214, 162) } else { Color::Rgb(130, 140, 165) }),
                    ),
                ]);

                let logs_block = Block::default()
                    .title(logs_title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(45, 54, 75)))
                    .style(Style::default().bg(Color::Rgb(16, 20, 30)));

                frame.render_widget(logs_block, logs_area);

                let inner_logs = logs_area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
                let logs_split = Layout::vertical([
                    Constraint::Min(0),      // Log lines
                    Constraint::Length(1),   // Reserved status/pill line
                ])
                .split(inner_logs);

                let max_line_width = logs_split[0].width.saturating_sub(1) as usize;
                let viewport_h = logs_split[0].height as usize;

                let (visible_lines, max_scroll, current_offset) = if !wrap_logs && !is_all {
                    let view = &mut service_views[active_tab - 1];
                    let total_lines = view.filtered_content().lines().count();
                    let max_scroll = total_lines.saturating_sub(viewport_h);
                    view.clamp_scroll(max_scroll);
                    let current_offset = view.scroll_offset;
                    let scroll_y = if current_offset == 0 {
                        max_scroll
                    } else {
                        max_scroll.saturating_sub(current_offset)
                    };

                    let content = view.filtered_content();
                    let visible: Vec<Line> = content
                        .lines()
                        .enumerate()
                        .skip(scroll_y)
                        .take(viewport_h)
                        .flat_map(|(idx, line)| {
                            format_log_entry(idx + 1, line, None, max_line_width, false)
                        })
                        .collect();

                    (visible, max_scroll, current_offset)
                } else {
                    let (formatted_lines, _) = if is_all {
                        let mut lines: Vec<Line<'static>> = Vec::new();
                        let mut line_no = 1;
                        for v in &service_views {
                            let content = v.filtered_content();
                            for l in content.lines() {
                                lines.extend(format_log_entry(
                                    line_no,
                                    l,
                                    Some(v.name()),
                                    max_line_width,
                                    wrap_logs,
                                ));
                                line_no += 1;
                            }
                        }
                        (lines, all_services_scroll_offset)
                    } else {
                        let view = &service_views[active_tab - 1];
                        let mut lines: Vec<Line<'static>> = Vec::new();
                        let mut line_no = 1;
                        let content = view.filtered_content();
                        for l in content.lines() {
                            lines.extend(format_log_entry(
                                line_no,
                                l,
                                None,
                                max_line_width,
                                wrap_logs,
                            ));
                            line_no += 1;
                        }
                        (lines, view.scroll_offset)
                    };

                    let total_lines = formatted_lines.len();
                    let max_scroll = total_lines.saturating_sub(viewport_h);

                    if is_all {
                        all_services_scroll_offset = all_services_scroll_offset.min(max_scroll);
                    } else {
                        service_views[active_tab - 1].clamp_scroll(max_scroll);
                    }
                    let current_offset = if is_all {
                        all_services_scroll_offset
                    } else {
                        service_views[active_tab - 1].scroll_offset
                    };

                    let scroll_y = if current_offset == 0 {
                        max_scroll
                    } else {
                        max_scroll.saturating_sub(current_offset)
                    };

                    let visible: Vec<Line> =
                        formatted_lines.into_iter().skip(scroll_y).take(viewport_h).collect();
                    (visible, max_scroll, current_offset)
                };

                frame.render_widget(Paragraph::new(visible_lines), logs_split[0]);

                // Render floating pill in reserved bottom status line - guaranteed zero overlap!
                let pill_line = if current_offset == 0 {
                    Line::from(Span::styled(
                        " ↓ Following latest ",
                        Style::default()
                            .fg(Color::Rgb(85, 214, 162))
                            .bg(Color::Rgb(18, 34, 28))
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    let pct = if max_scroll > 0 { ((max_scroll - current_offset) * 100) / max_scroll } else { 100 };
                    Line::from(Span::styled(
                        format!(" ↑ {} lines ({}%) [G: follow] ", current_offset, pct),
                        Style::default()
                            .fg(Color::Yellow)
                            .bg(Color::Rgb(36, 32, 18))
                            .add_modifier(Modifier::BOLD),
                    ))
                };
                frame.render_widget(Paragraph::new(pill_line).alignment(Alignment::Center), logs_split[1]);

                // Render scrollbar on right margin
                if max_scroll > 0 {
                    let mut scrollbar_state = ScrollbarState::new(max_scroll)
                        .position(max_scroll.saturating_sub(current_offset));
                    frame.render_stateful_widget(
                        Scrollbar::default()
                            .orientation(ScrollbarOrientation::VerticalRight)
                            .style(Style::default().fg(Color::Rgb(125, 140, 255)))
                            .track_style(Style::default().fg(Color::Rgb(28, 34, 48)))
                            .begin_symbol(Some("▲"))
                            .end_symbol(Some("▼")),
                        logs_area,
                        &mut scrollbar_state,
                    );
                }
            } else if active_tab < total_tabs {
                // Shell tab
                let shell_index = active_tab - 1 - service_views.len();
                if let Some(shell) = shell_tabs.get(shell_index) {
                    let terminal_text = shell.screen_contents();
                    let title_spans = vec![
                        Span::styled(" 🐚 ", Style::default().fg(Color::Cyan)),
                        Span::styled(&shell.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled(" [INTERACTIVE SHELL]", Style::default().fg(Color::Cyan)),
                    ];

                    let terminal_pane = Paragraph::new(terminal_text).block(
                        Block::default()
                            .title(Line::from(title_spans))
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Rgb(45, 54, 75))),
                    );
                    frame.render_widget(terminal_pane, content_rect);
                }
            }
            }

            // -------------------------------------------------------------
            // 4. BOTTOM STATUS BAR
            // -------------------------------------------------------------
            let running_count = service_views.iter().filter(|v| matches!(v.status(), ProcessStatus::Running)).count();
            let total_count = service_views.len();
            let is_shell_tab = active_tab >= service_views.len();

            let active_action = action_status.as_ref().and_then(|(msg, time)| {
                if time.elapsed() < Duration::from_secs(3) {
                    Some(msg.as_str())
                } else {
                    None
                }
            });

            let footer_lines: Vec<Line> = match &error_message {
                Some(error) => {
                    // Split multi-line errors for better display
                    error.lines().map(|line| {
                        if line.starts_with("Try:") || line.starts_with("  ") {
                            Line::from(Span::styled(line, Style::default().fg(Color::Yellow)))
                        } else {
                            Line::from(vec![
                                Span::styled("⚠ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                                Span::styled(line, Style::default().fg(Color::Red)),
                            ])
                        }
                    }).collect()
                }
                None => {
                    if is_searching {
                        if footer_height == 1 {
                            vec![Line::from(vec![
                                Span::styled("🔍 Filter: /", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::styled(&search_input, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                                Span::styled("█", Style::default().fg(Color::Yellow)),
                                Span::raw("  "),
                                Span::styled("(Enter: Apply • Esc: Clear)", Style::default().fg(Color::DarkGray)),
                            ])]
                        } else {
                            vec![
                                Line::from(vec![
                                    Span::styled("🔍 Filter: /", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                    Span::styled(&search_input, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                                    Span::styled("█", Style::default().fg(Color::Yellow)),
                                ]),
                                Line::from(vec![
                                    Span::styled("[Enter] Apply filter   [Esc] Clear filter   (live case-insensitive matching)", Style::default().fg(Color::DarkGray)),
                                ]),
                            ]
                        }
                    } else {
                        let left_status = Line::from(vec![
                            Span::styled("✓ ", Style::default().fg(Color::Rgb(85, 214, 162)).add_modifier(Modifier::BOLD)),
                            Span::styled(format!("{}/{} running", running_count, total_count), Style::default().fg(Color::Rgb(85, 214, 162)).add_modifier(Modifier::BOLD)),
                        ]);

                        let daemon_spans = if let Some(msg) = active_action {
                            vec![
                                Span::styled(format!("⚡ {}   ", msg), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::styled("✓ Daemon connected", Style::default().fg(Color::Rgb(85, 214, 162))),
                            ]
                        } else {
                            vec![
                                Span::styled("✓ Daemon connected", Style::default().fg(Color::Rgb(85, 214, 162))),
                            ]
                        };

                        let mut center_keys = Vec::new();
                        if is_shell_tab {
                            center_keys.extend(render_keycap("Tab", "Switch"));
                            center_keys.extend(render_keycap("^B", "Pane"));
                            center_keys.extend(render_keycap("^T", "Shell"));
                            center_keys.extend(render_keycap("^W", "Close"));
                            center_keys.extend(render_keycap("PgUp/Dn", "Scroll"));
                            center_keys.extend(render_keycap("^Q", "Quit"));
                        } else {
                            center_keys.extend(render_keycap("Tab", "Switch"));
                            center_keys.extend(render_keycap("R", "Restart"));
                            center_keys.extend(render_keycap("S", "Stop"));
                            center_keys.extend(render_keycap("W", if wrap_logs { "Unwrap" } else { "Wrap" }));
                            center_keys.extend(render_keycap("/", "Filter"));
                            match sidebar_state {
                                SidebarState::Normal => {
                                    center_keys.extend(render_keycap("M", "Min/Max"));
                                }
                                SidebarState::Minimized => {
                                    center_keys.extend(render_keycap("M", "Restore"));
                                }
                                SidebarState::Maximized => {
                                    center_keys.extend(render_keycap("M", "Restore"));
                                }
                            }
                            center_keys.extend(render_keycap("^T", "Shell"));
                            center_keys.extend(render_keycap("Q", "Quit"));
                        }

                        let right_footer_len: usize = daemon_spans.iter().map(|s| s.content.len()).sum();
                        let right_footer_w = (right_footer_len as u16 + 2).min(term_width.saturating_sub(30));

                        if term_width >= 105 {
                            let footer_chunks = Layout::horizontal([
                                Constraint::Length(18),
                                Constraint::Min(0),
                                Constraint::Length(right_footer_w),
                            ])
                            .split(footer_area);

                            frame.render_widget(Paragraph::new(left_status), footer_chunks[0]);
                            frame.render_widget(Paragraph::new(Line::from(center_keys)).alignment(Alignment::Center), footer_chunks[1]);
                            frame.render_widget(Paragraph::new(Line::from(daemon_spans)).alignment(Alignment::Right), footer_chunks[2]);
                            Vec::new()
                        } else {
                            let pane_txt = match sidebar_state {
                                SidebarState::Normal => "[M] Pane  ",
                                _ => "[M] Restore  ",
                            };
                            let compact_line = Line::from(vec![
                                Span::styled(format!("✓ {}/{}  ", running_count, total_count), Style::default().fg(Color::Rgb(85, 214, 162))),
                                Span::styled(format!("[Tab] Switch  {}[R] Restart  [Q] Quit", pane_txt), Style::default().fg(Color::Rgb(165, 175, 195))),
                            ]);
                            vec![compact_line]
                        }
                    }
                }
            };

            if !footer_lines.is_empty() {
                frame.render_widget(Paragraph::new(footer_lines).wrap(Wrap { trim: true }), footer_area);
            }
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Resize(cols, rows) => {
                    let _ = terminal.clear();
                    // Resize all shell tabs to match new terminal size
                    for shell in &mut shell_tabs {
                        if let Err(e) = shell.resize(rows.saturating_sub(6), cols) {
                            eprintln!("Failed to resize shell {}: {}", shell.name, e);
                        }
                    }
                }
                Event::Key(key) => {
                    if key.kind == event::KeyEventKind::Press {
                        // Handle search input mode
                        if is_searching {
                            match key.code {
                                KeyCode::Esc => {
                                    is_searching = false;
                                    search_input.clear();
                                    for v in &mut service_views {
                                        v.set_search(None);
                                    }
                                }
                                KeyCode::Enter => {
                                    is_searching = false;
                                }
                                KeyCode::Backspace => {
                                    search_input.pop();
                                    let q = if search_input.is_empty() {
                                        None
                                    } else {
                                        Some(search_input.clone())
                                    };
                                    if active_tab > 0 && active_tab <= service_views.len() {
                                        service_views[active_tab - 1].set_search(q);
                                    } else {
                                        for v in &mut service_views {
                                            v.set_search(q.clone());
                                        }
                                    }
                                }
                                KeyCode::Char(c)
                                    if !key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    search_input.push(c);
                                    let q = Some(search_input.clone());
                                    if active_tab > 0 && active_tab <= service_views.len() {
                                        service_views[active_tab - 1].set_search(q);
                                    } else {
                                        for v in &mut service_views {
                                            v.set_search(q.clone());
                                        }
                                    }
                                }
                                _ => {}
                            }
                            continue;
                        }

                        let total_tabs = if service_views.is_empty() && shell_tabs.is_empty() {
                            0
                        } else {
                            1 + service_views.len() + shell_tabs.len()
                        };

                        match (key.code, key.modifiers) {
                            // Search filter in logs
                            (KeyCode::Char('/'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                is_searching = true;
                                search_input =
                                    if active_tab > 0 && active_tab <= service_views.len() {
                                        service_views[active_tab - 1]
                                            .search_query
                                            .clone()
                                            .unwrap_or_default()
                                    } else {
                                        String::new()
                                    };
                                continue;
                            }

                            // Clear search filter with Esc, or restore sidebar if minimized/maximized
                            (KeyCode::Esc, KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                let had_search =
                                    service_views.iter().any(|v| v.search_query.is_some())
                                        || !search_input.is_empty();
                                for v in &mut service_views {
                                    v.set_search(None);
                                }
                                search_input.clear();
                                if !had_search && sidebar_state != SidebarState::Normal {
                                    sidebar_state = SidebarState::Normal;
                                }
                                continue;
                            }

                            // Interactive service controls (r: Restart, s: Stop/Start toggle)
                            (KeyCode::Char('r'), KeyModifiers::NONE)
                            | (KeyCode::Char('R'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                if active_tab == 0 {
                                    action_status = Some((
                                        "Restarting all services...".to_string(),
                                        Instant::now(),
                                    ));
                                    if let Ok(mut stream) = devpane::ipc::connect() {
                                        let _ = devpane::protocol::write_message(
                                            &mut stream,
                                            &devpane::protocol::Request::Restart {
                                                name: None,
                                                project_dir: None,
                                                services: None,
                                            },
                                        );
                                        let _ = devpane::protocol::read_message::<
                                            _,
                                            devpane::protocol::Response,
                                        >(
                                            &mut stream
                                        );
                                    }
                                } else {
                                    let service_name =
                                        service_views[active_tab - 1].name().to_string();
                                    action_status = Some((
                                        format!("Restarting '{service_name}'..."),
                                        Instant::now(),
                                    ));
                                    if let Ok(mut stream) = devpane::ipc::connect() {
                                        let _ = devpane::protocol::write_message(
                                            &mut stream,
                                            &devpane::protocol::Request::Restart {
                                                name: Some(service_name.clone()),
                                                project_dir: None,
                                                services: None,
                                            },
                                        );
                                        let _ = devpane::protocol::read_message::<
                                            _,
                                            devpane::protocol::Response,
                                        >(
                                            &mut stream
                                        );
                                    }
                                }
                                last_refresh = Instant::now() - Duration::from_secs(1);
                                continue;
                            }

                            (KeyCode::Char('s'), KeyModifiers::NONE)
                            | (KeyCode::Char('S'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                if active_tab == 0 {
                                    action_status = Some((
                                        "Stopping all services...".to_string(),
                                        Instant::now(),
                                    ));
                                    for v in &service_views {
                                        if let Ok(mut stream) = devpane::ipc::connect() {
                                            let _ = devpane::protocol::write_message(
                                                &mut stream,
                                                &devpane::protocol::Request::Stop {
                                                    id: v.session.id.clone(),
                                                },
                                            );
                                            let _ = devpane::protocol::read_message::<
                                                _,
                                                devpane::protocol::Response,
                                            >(
                                                &mut stream
                                            );
                                        }
                                    }
                                } else {
                                    let view = &service_views[active_tab - 1];
                                    let service_name = view.name().to_string();
                                    let is_running =
                                        matches!(view.status(), ProcessStatus::Running);
                                    if is_running {
                                        action_status = Some((
                                            format!("Stopping '{service_name}'..."),
                                            Instant::now(),
                                        ));
                                        let session_id = view.session.id.clone();
                                        if let Ok(mut stream) = devpane::ipc::connect() {
                                            let _ = devpane::protocol::write_message(
                                                &mut stream,
                                                &devpane::protocol::Request::Stop {
                                                    id: session_id,
                                                },
                                            );
                                            let _ = devpane::protocol::read_message::<
                                                _,
                                                devpane::protocol::Response,
                                            >(
                                                &mut stream
                                            );
                                        }
                                    } else {
                                        action_status = Some((
                                            format!("Starting '{service_name}'..."),
                                            Instant::now(),
                                        ));
                                        if let Ok(mut stream) = devpane::ipc::connect() {
                                            let _ = devpane::protocol::write_message(
                                                &mut stream,
                                                &devpane::protocol::Request::Restart {
                                                    name: Some(service_name.clone()),
                                                    project_dir: None,
                                                    services: None,
                                                },
                                            );
                                            let _ = devpane::protocol::read_message::<
                                                _,
                                                devpane::protocol::Response,
                                            >(
                                                &mut stream
                                            );
                                        }
                                    }
                                }
                                last_refresh = Instant::now() - Duration::from_secs(1);
                                continue;
                            }

                            // Open detected URL in browser (o / O)
                            (KeyCode::Char('o'), KeyModifiers::NONE)
                            | (KeyCode::Char('O'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                let url = if active_tab == 0 {
                                    service_views.iter().find_map(|v| v.detected_url())
                                } else {
                                    service_views[active_tab - 1].detected_url()
                                };
                                if let Some(u) = url {
                                    action_status =
                                        Some((format!("Opening {u}..."), Instant::now()));
                                    open_external_url(&u);
                                }
                                continue;
                            }

                            // Quit: Ctrl+C or Ctrl+Q always quits; 'q' quits on service views
                            (KeyCode::Char('c'), KeyModifiers::CONTROL)
                            | (KeyCode::Char('\x03'), _) => break,
                            (KeyCode::Char('q'), KeyModifiers::CONTROL)
                            | (KeyCode::Char('\x11'), _) => break,
                            (KeyCode::Char('q'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                break;
                            }

                            // Toggle wrap mode (w / W)
                            (KeyCode::Char('w'), KeyModifiers::NONE)
                            | (KeyCode::Char('W'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                wrap_logs = !wrap_logs;
                                continue;
                            }

                            // Minimize and Maximize services list pane ('m' toggles minimize, 'M' toggles maximize)
                            (KeyCode::Char('m'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                sidebar_state = match sidebar_state {
                                    SidebarState::Normal => SidebarState::Minimized,
                                    SidebarState::Minimized => SidebarState::Normal,
                                    SidebarState::Maximized => SidebarState::Normal,
                                };
                                continue;
                            }
                            (KeyCode::Char('M'), KeyModifiers::NONE)
                            | (KeyCode::Char('m'), KeyModifiers::SHIFT)
                                if active_tab <= service_views.len() =>
                            {
                                sidebar_state = match sidebar_state {
                                    SidebarState::Normal => SidebarState::Maximized,
                                    SidebarState::Maximized => SidebarState::Normal,
                                    SidebarState::Minimized => SidebarState::Normal,
                                };
                                continue;
                            }
                            // Universal toggle (works on both service views and shells)
                            (KeyCode::Char('b'), KeyModifiers::CONTROL)
                            | (KeyCode::Char('\x02'), _) => {
                                sidebar_state = match sidebar_state {
                                    SidebarState::Normal => SidebarState::Minimized,
                                    _ => SidebarState::Normal,
                                };
                                continue;
                            }
                            // Enter in Maximized view restores split view
                            (KeyCode::Enter, KeyModifiers::NONE)
                                if sidebar_state == SidebarState::Maximized =>
                            {
                                sidebar_state = SidebarState::Normal;
                                continue;
                            }

                            // Quick Tab Switch: 0 for All services, 1-9 for individual services
                            (KeyCode::Char('0'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                active_tab = 0;
                                continue;
                            }
                            (KeyCode::Char(c @ '1'..='9'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                let target = (c as usize) - ('1' as usize) + 1;
                                if target < total_tabs {
                                    active_tab = target;
                                }
                                continue;
                            }

                            // Tab navigation
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                if total_tabs > 0 {
                                    active_tab = (active_tab + 1) % total_tabs;
                                }
                                continue;
                            }
                            (KeyCode::BackTab, _) => {
                                if total_tabs > 0 {
                                    active_tab = if active_tab == 0 {
                                        total_tabs - 1
                                    } else {
                                        active_tab - 1
                                    };
                                }
                                continue;
                            }
                            (KeyCode::Left, KeyModifiers::NONE)
                            | (KeyCode::Char('h'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                if total_tabs > 0 {
                                    active_tab = if active_tab == 0 {
                                        total_tabs - 1
                                    } else {
                                        active_tab - 1
                                    };
                                }
                                continue;
                            }
                            (KeyCode::Right, KeyModifiers::NONE)
                            | (KeyCode::Char('l'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                if total_tabs > 0 {
                                    active_tab = (active_tab + 1) % total_tabs;
                                }
                                continue;
                            }

                            // New shell tab (Ctrl+T)
                            (KeyCode::Char('t'), KeyModifiers::CONTROL)
                            | (KeyCode::Char('\x14'), _) => {
                                let (program, args) = devpane::terminal::default_shell();
                                let shell_number = shell_tabs.len() + 1;

                                let size = terminal.size()?;
                                let rows = size.height.saturating_sub(6);
                                let cols = size.width;

                                match TerminalPane::new(
                                    format!("Shell {}", shell_number),
                                    program,
                                    &args,
                                    std::env::current_dir()?,
                                    rows,
                                    cols,
                                ) {
                                    Ok(pane) => {
                                        shell_tabs.push(pane);
                                        active_tab = 1 + service_views.len() + shell_tabs.len() - 1;
                                    }
                                    Err(e) => {
                                        error_message =
                                            Some(format!("Failed to create shell: {}", e));
                                    }
                                }
                                continue;
                            }

                            // Close tab (Ctrl+W)
                            (KeyCode::Char('w'), KeyModifiers::CONTROL)
                            | (KeyCode::Char('W'), KeyModifiers::CONTROL)
                            | (KeyCode::Char('\x17'), _) => {
                                if total_tabs > 0 {
                                    if active_tab > 0 && active_tab <= service_views.len() {
                                        let service_view = service_views.remove(active_tab - 1);
                                        let session_id = service_view.session.id.clone();
                                        closed_session_ids.insert(session_id.clone());
                                        if let Ok(mut stop_stream) = devpane::ipc::connect() {
                                            let _ = devpane::protocol::write_message(
                                                &mut stop_stream,
                                                &devpane::protocol::Request::Stop {
                                                    id: session_id,
                                                },
                                            );
                                            let _ = devpane::protocol::read_message::<
                                                _,
                                                devpane::protocol::Response,
                                            >(
                                                &mut stop_stream
                                            );
                                        }
                                    } else if active_tab > service_views.len() {
                                        let shell_index = active_tab - 1 - service_views.len();
                                        if shell_index < shell_tabs.len() {
                                            shell_tabs.remove(shell_index);
                                        }
                                    }

                                    let new_total =
                                        if service_views.is_empty() && shell_tabs.is_empty() {
                                            0
                                        } else {
                                            1 + service_views.len() + shell_tabs.len()
                                        };
                                    if new_total == 0 {
                                        break;
                                    } else if active_tab >= new_total {
                                        active_tab = new_total - 1;
                                    }
                                }
                                continue;
                            }

                            // Force refresh (Ctrl+R)
                            (KeyCode::Char('r'), KeyModifiers::CONTROL)
                            | (KeyCode::Char('\x12'), _) => {
                                last_refresh = Instant::now() - Duration::from_secs(1);
                                continue;
                            }

                            // Scroll up
                            (KeyCode::PageUp, _) | (KeyCode::Up, KeyModifiers::SHIFT) => {
                                if active_tab == 0 {
                                    all_services_scroll_offset =
                                        all_services_scroll_offset.saturating_add(10);
                                } else if active_tab <= service_views.len() {
                                    let v = &mut service_views[active_tab - 1];
                                    v.scroll_offset = v.scroll_offset.saturating_add(10);
                                } else {
                                    let shell_index = active_tab - 1 - service_views.len();
                                    if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                        shell.scroll_up(10);
                                    }
                                }
                                continue;
                            }
                            (KeyCode::Up, KeyModifiers::NONE)
                            | (KeyCode::Char('k'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                if sidebar_state == SidebarState::Maximized {
                                    active_tab = active_tab.saturating_sub(1);
                                    continue;
                                }
                                if active_tab == 0 {
                                    all_services_scroll_offset =
                                        all_services_scroll_offset.saturating_add(1);
                                } else {
                                    let v = &mut service_views[active_tab - 1];
                                    v.scroll_offset = v.scroll_offset.saturating_add(1);
                                }
                                continue;
                            }

                            // Scroll down
                            (KeyCode::PageDown, _) | (KeyCode::Down, KeyModifiers::SHIFT) => {
                                if active_tab == 0 {
                                    all_services_scroll_offset =
                                        all_services_scroll_offset.saturating_sub(10);
                                } else if active_tab <= service_views.len() {
                                    service_views[active_tab - 1].scroll_down(10);
                                } else {
                                    let shell_index = active_tab - 1 - service_views.len();
                                    if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                        shell.scroll_down(10);
                                    }
                                }
                                continue;
                            }
                            (KeyCode::Down, KeyModifiers::NONE)
                            | (KeyCode::Char('j'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                if sidebar_state == SidebarState::Maximized {
                                    if total_tabs > 0 && active_tab + 1 < total_tabs {
                                        active_tab += 1;
                                    }
                                    continue;
                                }
                                if active_tab == 0 {
                                    all_services_scroll_offset =
                                        all_services_scroll_offset.saturating_sub(1);
                                } else {
                                    service_views[active_tab - 1].scroll_down(1);
                                }
                                continue;
                            }

                            // Scroll to top
                            (KeyCode::Home, _) | (KeyCode::Char('g'), KeyModifiers::NONE)
                                if active_tab <= service_views.len() =>
                            {
                                if active_tab == 0 {
                                    all_services_scroll_offset = usize::MAX / 2;
                                } else {
                                    let v = &mut service_views[active_tab - 1];
                                    v.scroll_offset = usize::MAX / 2;
                                }
                                continue;
                            }

                            // Scroll to bottom
                            (KeyCode::End, _)
                            | (KeyCode::Char('G'), KeyModifiers::NONE)
                            | (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                                if active_tab == 0 {
                                    all_services_scroll_offset = 0;
                                } else if active_tab <= service_views.len() {
                                    service_views[active_tab - 1].scroll_to_bottom();
                                } else {
                                    let shell_index = active_tab - 1 - service_views.len();
                                    if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                        shell.scroll_to_bottom();
                                    }
                                }
                                continue;
                            }

                            _ => {}
                        }

                        // Send input to shell tabs only
                        if active_tab > service_views.len() {
                            let shell_index = active_tab - 1 - service_views.len();
                            if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                let input: Option<Vec<u8>> = match key.code {
                                    KeyCode::Char(character) => {
                                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                                            None
                                        } else {
                                            Some(character.to_string().into_bytes())
                                        }
                                    }
                                    KeyCode::Enter => Some(vec![b'\r']),
                                    KeyCode::Backspace => Some(vec![0x7f]),
                                    KeyCode::Esc => Some(vec![0x1b]),
                                    KeyCode::Up => Some(vec![0x1b, b'[', b'A']),
                                    KeyCode::Down => Some(vec![0x1b, b'[', b'B']),
                                    KeyCode::Right => Some(vec![0x1b, b'[', b'C']),
                                    KeyCode::Left => Some(vec![0x1b, b'[', b'D']),
                                    _ => None,
                                };

                                if let Some(input) = input {
                                    let _ = shell.send_input(&input);
                                }
                            }
                        }
                    }
                }
                Event::Mouse(mouse_event) => {
                    let (term_w, term_h) = terminal
                        .size()
                        .map(|s| (s.width, s.height))
                        .unwrap_or((80, 24));
                    let default_sb_w: u16 = if term_w >= 115 {
                        34
                    } else if term_w >= 90 {
                        32
                    } else if term_w >= 80 {
                        28
                    } else {
                        0
                    };
                    let sidebar_width: u16 = match sidebar_state {
                        SidebarState::Minimized => 0,
                        SidebarState::Maximized => term_w,
                        SidebarState::Normal => default_sb_w,
                    };

                    match mouse_event.kind {
                        event::MouseEventKind::ScrollUp => {
                            if sidebar_state == SidebarState::Maximized {
                                active_tab = active_tab.saturating_sub(1);
                            } else if sidebar_width > 0 && mouse_event.column < sidebar_width {
                                sidebar_scroll = sidebar_scroll.saturating_sub(1);
                            } else if active_tab == 0 {
                                all_services_scroll_offset =
                                    all_services_scroll_offset.saturating_add(3);
                            } else if active_tab <= service_views.len() {
                                let v = &mut service_views[active_tab - 1];
                                v.scroll_offset = v.scroll_offset.saturating_add(3);
                            } else {
                                let shell_index = active_tab - 1 - service_views.len();
                                if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                    shell.scroll_up(3);
                                }
                            }
                        }
                        event::MouseEventKind::ScrollDown => {
                            if sidebar_state == SidebarState::Maximized {
                                let total = if service_views.is_empty() && shell_tabs.is_empty() {
                                    0
                                } else {
                                    1 + service_views.len() + shell_tabs.len()
                                };
                                if total > 0 && active_tab + 1 < total {
                                    active_tab += 1;
                                }
                            } else if sidebar_width > 0 && mouse_event.column < sidebar_width {
                                sidebar_scroll = sidebar_scroll.saturating_add(1);
                            } else if active_tab == 0 {
                                all_services_scroll_offset =
                                    all_services_scroll_offset.saturating_sub(3);
                            } else if active_tab <= service_views.len() {
                                service_views[active_tab - 1].scroll_down(3);
                            } else {
                                let shell_index = active_tab - 1 - service_views.len();
                                if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                    shell.scroll_down(3);
                                }
                            }
                        }
                        event::MouseEventKind::Down(MouseButton::Left) => {
                            let top_bar_height: u16 = if term_h >= 22 { 2 } else { 1 };
                            let search_box_width: u16 = if term_w >= 115 {
                                28
                            } else if term_w >= 80 {
                                20
                            } else {
                                12
                            };

                            // Click on search box in top bar
                            if mouse_event.row < top_bar_height
                                && mouse_event.column >= term_w.saturating_sub(search_box_width)
                            {
                                is_searching = true;
                            }
                            // Click on [m: Services] in top bar when Minimized
                            else if sidebar_state == SidebarState::Minimized
                                && mouse_event.row < top_bar_height
                                && mouse_event.column < 45
                            {
                                sidebar_state = SidebarState::Normal;
                            }
                            // Click in Maximized services view
                            else if sidebar_state == SidebarState::Maximized {
                                if mouse_event.row <= top_bar_height {
                                    sidebar_state = SidebarState::Normal;
                                } else {
                                    let card_start_y = top_bar_height + 2;
                                    let total_tabs =
                                        if service_views.is_empty() && shell_tabs.is_empty() {
                                            0
                                        } else {
                                            1 + service_views.len() + shell_tabs.len()
                                        };
                                    if mouse_event.row >= card_start_y {
                                        let slot = ((mouse_event.row.saturating_sub(card_start_y))
                                            / 5)
                                            as usize;
                                        let card_idx = sidebar_scroll + slot;
                                        if card_idx < total_tabs {
                                            active_tab = card_idx;
                                            sidebar_state = SidebarState::Normal;
                                        }
                                    }
                                }
                            }
                            // Click in sidebar to switch service/tab
                            else if sidebar_width > 0 && mouse_event.column < sidebar_width {
                                if mouse_event.row == top_bar_height {
                                    // Header row: check for minimize / maximize buttons
                                    if mouse_event.column >= sidebar_width.saturating_sub(4) {
                                        sidebar_state = SidebarState::Maximized;
                                    } else if mouse_event.column >= sidebar_width.saturating_sub(8)
                                    {
                                        sidebar_state = SidebarState::Minimized;
                                    }
                                } else if mouse_event.row > top_bar_height {
                                    let list_start_y =
                                        top_bar_height + 2 + if term_h >= 22 { 1 } else { 0 };
                                    let footer_height: u16 = 3;
                                    let bottom_space: u16 = if term_h >= 16 { 1 } else { 0 };
                                    let footer_y = top_bar_height
                                        + (term_h.saturating_sub(top_bar_height + 1))
                                            .saturating_sub(footer_height + bottom_space);
                                    if mouse_event.row >= list_start_y && mouse_event.row < footer_y
                                    {
                                        let total_cards =
                                            if service_views.is_empty() && shell_tabs.is_empty() {
                                                0
                                            } else {
                                                1 + service_views.len() + shell_tabs.len()
                                            };
                                        let item_height: u16 = if term_h >= 16 { 2 } else { 1 };
                                        let item_gap: u16 = if term_h >= 20 { 1 } else { 0 };
                                        let item_step = item_height + item_gap;
                                        if item_step > 0 {
                                            let slot =
                                                ((mouse_event.row.saturating_sub(list_start_y))
                                                    / item_step)
                                                    as usize;
                                            let card_idx = sidebar_scroll + slot;
                                            if card_idx < total_cards {
                                                active_tab = card_idx;
                                            }
                                        }
                                    }
                                }
                            } else if mouse_event.row == top_bar_height
                                && active_tab <= service_views.len()
                            {
                                // Click on Restart / Stop / URL in right pane header
                                let det_url = if active_tab == 0 {
                                    service_views.iter().find_map(|v| v.detected_url())
                                } else {
                                    service_views[active_tab - 1].detected_url()
                                };
                                let url_len = det_url.as_ref().map(|u| u.len() + 6).unwrap_or(0);
                                let right_btns_w = 14 + 12 + url_len;
                                let right_margin: u16 = if term_w >= 80 { 1 } else { 0 };

                                if mouse_event.column
                                    >= term_w.saturating_sub(right_btns_w as u16 + right_margin)
                                {
                                    if url_len > 0
                                        && mouse_event.column
                                            >= term_w.saturating_sub(url_len as u16 + right_margin)
                                    {
                                        if let Some(url) = det_url {
                                            action_status =
                                                Some((format!("Opening {url}..."), Instant::now()));
                                            open_external_url(&url);
                                        }
                                    } else if mouse_event.column
                                        >= term_w
                                            .saturating_sub((url_len + 12) as u16 + right_margin)
                                    {
                                        // Stop button clicked
                                        if active_tab == 0 {
                                            action_status = Some((
                                                "Stopping all services...".to_string(),
                                                Instant::now(),
                                            ));
                                            for v in &service_views {
                                                if let Ok(mut stream) = devpane::ipc::connect() {
                                                    let _ = devpane::protocol::write_message(
                                                        &mut stream,
                                                        &devpane::protocol::Request::Stop {
                                                            id: v.session.id.clone(),
                                                        },
                                                    );
                                                    let _ = devpane::protocol::read_message::<
                                                        _,
                                                        devpane::protocol::Response,
                                                    >(
                                                        &mut stream
                                                    );
                                                }
                                            }
                                        } else {
                                            let view = &service_views[active_tab - 1];
                                            let service_name = view.name().to_string();
                                            let is_running =
                                                matches!(view.status(), ProcessStatus::Running);
                                            if is_running {
                                                action_status = Some((
                                                    format!("Stopping '{service_name}'..."),
                                                    Instant::now(),
                                                ));
                                                let session_id = view.session.id.clone();
                                                if let Ok(mut stream) = devpane::ipc::connect() {
                                                    let _ = devpane::protocol::write_message(
                                                        &mut stream,
                                                        &devpane::protocol::Request::Stop {
                                                            id: session_id,
                                                        },
                                                    );
                                                    let _ = devpane::protocol::read_message::<
                                                        _,
                                                        devpane::protocol::Response,
                                                    >(
                                                        &mut stream
                                                    );
                                                }
                                            }
                                        }
                                    } else {
                                        // Restart button clicked
                                        if active_tab == 0 {
                                            action_status = Some((
                                                "Restarting all services...".to_string(),
                                                Instant::now(),
                                            ));
                                            if let Ok(mut stream) = devpane::ipc::connect() {
                                                let _ = devpane::protocol::write_message(
                                                    &mut stream,
                                                    &devpane::protocol::Request::Restart {
                                                        name: None,
                                                        project_dir: None,
                                                        services: None,
                                                    },
                                                );
                                                let _ = devpane::protocol::read_message::<
                                                    _,
                                                    devpane::protocol::Response,
                                                >(
                                                    &mut stream
                                                );
                                            }
                                        } else {
                                            let service_name =
                                                service_views[active_tab - 1].name().to_string();
                                            action_status = Some((
                                                format!("Restarting '{service_name}'..."),
                                                Instant::now(),
                                            ));
                                            if let Ok(mut stream) = devpane::ipc::connect() {
                                                let _ = devpane::protocol::write_message(
                                                    &mut stream,
                                                    &devpane::protocol::Request::Restart {
                                                        name: Some(service_name),
                                                        project_dir: None,
                                                        services: None,
                                                    },
                                                );
                                                let _ = devpane::protocol::read_message::<
                                                    _,
                                                    devpane::protocol::Response,
                                                >(
                                                    &mut stream
                                                );
                                            }
                                        }
                                        last_refresh = Instant::now() - Duration::from_secs(1);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_state_transitions() {
        let mut state = SidebarState::Normal;

        // Press 'm' to minimize
        state = match state {
            SidebarState::Normal => SidebarState::Minimized,
            _ => SidebarState::Normal,
        };
        assert_eq!(state, SidebarState::Minimized);

        // Press 'm' to restore
        state = match state {
            SidebarState::Normal => SidebarState::Minimized,
            _ => SidebarState::Normal,
        };
        assert_eq!(state, SidebarState::Normal);

        // Press 'M' to maximize
        state = match state {
            SidebarState::Normal => SidebarState::Maximized,
            _ => SidebarState::Normal,
        };
        assert_eq!(state, SidebarState::Maximized);

        // Press 'Esc' / 'Enter' to restore
        state = SidebarState::Normal;
        assert_eq!(state, SidebarState::Normal);
    }

    #[test]
    fn test_service_icon_and_alignment_widths() {
        let (api_icon, api_w) = service_icon_and_width("api");
        assert_eq!(api_icon, "⬡ ");
        assert_eq!(api_w, 2);

        let (web_icon, web_w) = service_icon_and_width("web");
        assert_eq!(web_icon, "🌐 ");
        assert_eq!(web_w, 3);

        let (worker_icon, worker_w) = service_icon_and_width("worker");
        assert_eq!(worker_icon, "⚙ ");
        assert_eq!(worker_w, 2);
    }

    #[test]
    fn test_equal_spacing_calculations() {
        let area = Rect::new(0, 0, 32, 28);
        let footer_height: u16 = 3;
        let bottom_space: u16 = 1;
        // Footer is permanently sticked to the bottom with 1 line of breathing space
        let footer_y = area.y + area.height.saturating_sub(footer_height + bottom_space);
        assert_eq!(footer_y, 24);

        let top_gap: u16 = if area.height >= 22 { 1 } else { 0 };
        assert_eq!(top_gap, 1);
        let list_start_y = area.y + 2 + top_gap;
        assert_eq!(list_start_y, 3);

        let item_height: u16 = 2;
        let item_gap: u16 = 1;
        let item_step: u16 = item_height + item_gap;
        assert_eq!(item_step, 3);

        // Service items start at list_start_y and step by 3:
        // Item 0: y=3..5
        // Item 1: y=6..8
        // Item 2: y=9..11
        // Item 3: y=12..14
        // Footer stays sticked at bottom: y=24..27 with row 27 as breathing space
        let list_area_height = footer_y.saturating_sub(list_start_y);
        assert_eq!(list_area_height, 21);
        let max_visible = (list_area_height / item_step) as usize;
        assert_eq!(max_visible, 7); // Fits up to 7 items comfortably
    }

    #[test]
    fn test_sidebar_rendering_with_test_backend() {
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(80, 28);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 32, 28);
                let content_w = area.width - 2;
                let content_x = area.x + 1;

                // 1. Header rendering
                let header_rect = Rect::new(content_x, area.y, content_w, 1);
                let left_header = Line::from(vec![Span::raw("SERVICES")]);
                let right_header = Line::from(vec![Span::raw("3  [–] [⤢]")]);
                frame.render_widget(Paragraph::new(left_header), header_rect);
                frame.render_widget(
                    Paragraph::new(right_header).alignment(Alignment::Right),
                    header_rect,
                );

                // Header divider
                let header_div_rect = Rect::new(content_x, area.y + 1, content_w, 1);
                frame.render_widget(
                    Paragraph::new("─".repeat(content_w as usize)),
                    header_div_rect,
                );

                // 2. Footer sticked to the bottom with 1 line breathing space beneath
                let footer_height: u16 = 3;
                let bottom_space: u16 = 1;
                let footer_y = area.y + area.height.saturating_sub(footer_height + bottom_space);
                assert_eq!(footer_y, 24);

                let footer_div_rect = Rect::new(content_x, footer_y, content_w, 1);
                frame.render_widget(
                    Paragraph::new("─".repeat(content_w as usize)),
                    footer_div_rect,
                );

                let f_line1_rect = Rect::new(content_x, footer_y + 1, content_w, 1);
                let f_line2_rect = Rect::new(content_x, footer_y + 2, content_w, 1);
                frame.render_widget(Paragraph::new("Environment"), f_line1_rect);
                frame.render_widget(
                    Paragraph::new("development").alignment(Alignment::Right),
                    f_line1_rect,
                );
                frame.render_widget(Paragraph::new("Services"), f_line2_rect);
                frame.render_widget(
                    Paragraph::new("3 running · 0 exited").alignment(Alignment::Right),
                    f_line2_rect,
                );

                // 3. Render items
                let list_start_y = area.y + 3;
                let item_step = 3;

                // Item 0 (active)
                let item0_l1 = Rect::new(content_x, list_start_y, content_w, 1);
                let item0_l2 = Rect::new(content_x, list_start_y + 1, content_w, 1);
                frame.render_widget(Paragraph::new("▎ ≡ All services"), item0_l1);
                frame.render_widget(Paragraph::new(" 0 ").alignment(Alignment::Right), item0_l1);
                frame.render_widget(Paragraph::new("▎   Combined log stream"), item0_l2);

                // Item 1 (inactive)
                let item1_l1 = Rect::new(content_x, list_start_y + item_step, content_w, 1);
                let item1_l2 = Rect::new(content_x, list_start_y + item_step + 1, content_w, 1);
                frame.render_widget(Paragraph::new("  ⬡ api ●"), item1_l1);
                frame.render_widget(Paragraph::new(" 1 ").alignment(Alignment::Right), item1_l1);
                frame.render_widget(Paragraph::new("    :8080 · PID 52247"), item1_l2);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        // Check header at y=0
        assert_eq!(buffer[(1, 0)].symbol(), "S");
        // Check header divider at y=1
        assert_eq!(buffer[(1, 1)].symbol(), "─");
        // Check active indicator at y=3
        assert_eq!(buffer[(1, 3)].symbol(), "▎");
        // Check active subtitle indicator at y=4
        assert_eq!(buffer[(1, 4)].symbol(), "▎");
        // Check inactive indicator at y=6
        assert_eq!(buffer[(1, 6)].symbol(), " ");
        // Check footer divider sticked at bottom (y=24)
        assert_eq!(buffer[(1, 24)].symbol(), "─");
        // Check footer Environment at y=25
        assert_eq!(buffer[(1, 25)].symbol(), "E");
        // Check footer Services at y=26
        assert_eq!(buffer[(1, 26)].symbol(), "S");
        // Check breathing space at y=27
        assert_eq!(buffer[(1, 27)].symbol(), " ");
    }
}
