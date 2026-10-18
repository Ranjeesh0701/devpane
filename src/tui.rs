use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
};

use devpane::process::session::{ProcessSession, ProcessStatus};
use devpane::service_log_view::ServiceLogView;
use devpane::storage;
use devpane::workspace::TerminalPane;

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
    let mut active_tab: usize = 0;
    let mut error_message: Option<String> = None;
    let mut last_refresh = Instant::now() - Duration::from_secs(1);

    // Initial load of services
    match fetch_sessions() {
        Ok(sessions) => {
            for session in sessions {
                match storage::get_log_file(&session.id) {
                    Ok(log_path) => {
                        match ServiceLogView::new(session.clone(), log_path) {
                            Ok(view) => service_views.push(view),
                            Err(e) => {
                                eprintln!("Failed to create log view for {}: {}", session.name, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get log path for {}: {}", session.name, e);
                    }
                }
            }

            if service_views.is_empty() {
                error_message = Some(
                    "No services running.\n\nTry:\n  1. Check devbox.yaml has enabled services\n  2. Run 'devbox list' to see all sessions\n  3. Press Ctrl+T to create a shell".to_string()
                );
            }
        }
        Err(e) => {
            error_message = Some(format!(
                "Failed to connect to daemon: {}\n\nTry:\n  1. Check if daemon is running: devbox list\n  2. Restart daemon: devbox shutdown && devbox up",
                e
            ));
        }
    }

    loop {
        // Refresh session list and update logs periodically
        if last_refresh.elapsed() >= Duration::from_millis(500) {
            match fetch_sessions() {
                Ok(sessions) => {
                    // Update existing service views
                    for view in &mut service_views {
                        if let Err(e) = view.update() {
                            eprintln!("Failed to update log for {}: {}", view.name(), e);
                        }

                        // Update session status
                        if let Some(updated_session) = sessions.iter().find(|s| s.id == view.session.id) {
                            view.session = updated_session.clone();
                        }
                    }

                    // Remove service views whose session no longer exists in the daemon
                    let prev_services = service_views.len();
                    service_views.retain(|v| sessions.iter().any(|s| s.id == v.session.id));
                    if service_views.len() < prev_services {
                        let total = service_views.len() + shell_tabs.len();
                        if total > 0 && active_tab >= total {
                            active_tab = total - 1;
                        }
                    }

                    // Add new services that appeared (and were not explicitly closed by user)
                    for session in sessions {
                        if !closed_session_ids.contains(&session.id)
                            && !service_views.iter().any(|v| v.session.id == session.id)
                        {
                            match storage::get_log_file(&session.id) {
                                Ok(log_path) => {
                                    match ServiceLogView::new(session.clone(), log_path) {
                                        Ok(view) => service_views.push(view),
                                        Err(e) => {
                                            eprintln!("Failed to create log view for {}: {}", session.name, e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to get log path for {}: {}", session.name, e);
                                }
                            }
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
            let total = service_views.len() + shell_tabs.len();
            if total > 0 && active_tab >= total {
                active_tab = total - 1;
            }
        }

        // Render UI
        terminal.draw(|frame| {
            let areas = Layout::vertical([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),     // Content
                Constraint::Length(2),  // Footer
            ])
            .split(frame.area());

            // Build tab titles with colored status indicators
            let mut tab_titles: Vec<Line> = service_views
                .iter()
                .map(|view| {
                    let (status_icon, status_color) = match view.status() {
                        ProcessStatus::Running => ("●", Color::Green),
                        ProcessStatus::Exited => ("○", Color::Red),
                    };
                    Line::from(vec![
                        Span::styled(status_icon, Style::default().fg(status_color)),
                        Span::raw(" "),
                        Span::raw(view.name()),
                    ])
                })
                .collect();

            // Add shell tabs with different icon
            for shell in shell_tabs.iter() {
                tab_titles.push(Line::from(vec![
                    Span::styled("🐚", Style::default().fg(Color::Cyan)),
                    Span::raw(" "),
                    Span::raw(shell.name.as_str()),
                ]));
            }

            let tabs = Tabs::new(tab_titles)
                .block(Block::default().title(" Devbox Workspace ").borders(Borders::ALL))
                .select(active_tab)
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                        .bg(Color::DarkGray),
                );

            frame.render_widget(tabs, areas[0]);

            // Render current tab's content
            let total_tabs = service_views.len() + shell_tabs.len();

            if total_tabs == 0 {
                // Show helpful empty state
                let help_text = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Welcome to Devbox!",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from("No services are currently running."),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("To get started:"),
                    ]),
                    Line::from(vec![
                        Span::raw("  1. Press "),
                        Span::styled("Ctrl+T", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::raw(" to create an interactive shell"),
                    ]),
                    Line::from(vec![
                        Span::raw("  2. Edit "),
                        Span::styled("devbox.yaml", Style::default().fg(Color::Green)),
                        Span::raw(" to define your services"),
                    ]),
                    Line::from(vec![
                        Span::raw("  3. Run "),
                        Span::styled("devbox up", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::raw(" to start services"),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("Press "),
                        Span::styled("q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        Span::raw(" to quit"),
                    ]),
                ];

                let help_pane = Paragraph::new(help_text)
                    .block(Block::default().title(" Getting Started ").borders(Borders::ALL))
                    .alignment(ratatui::layout::Alignment::Center);

                frame.render_widget(help_pane, areas[1]);
            } else if active_tab < service_views.len() {
                // Service log view
                let view = &service_views[active_tab];
                let content = view.content();
                let total_lines = content.lines().count();
                let viewport_height = areas[1].height.saturating_sub(2) as usize;
                let max_scroll = total_lines.saturating_sub(viewport_height);

                let scroll_y = if view.scroll_offset == 0 {
                    max_scroll
                } else {
                    max_scroll.saturating_sub(view.scroll_offset)
                };

                let (status_text, status_color) = match view.status() {
                    ProcessStatus::Running => ("Running", Color::Green),
                    ProcessStatus::Exited => {
                        if let Some(code) = view.session.exit_code {
                            if code == 0 {
                                ("Exited (success)", Color::Yellow)
                            } else {
                                ("Exited (failed)", Color::Red)
                            }
                        } else {
                            ("Exited", Color::Red)
                        }
                    }
                };

                let mut title = match view.detected_url() {
                    Some(url) => format!(
                        " {} - {} (PID: {}) | 🌐 {} ",
                        view.name(),
                        status_text,
                        view.session.pid,
                        url
                    ),
                    None => format!(
                        " {} - {} (PID: {}) ",
                        view.name(),
                        status_text,
                        view.session.pid
                    ),
                };

                if view.scroll_offset > 0 {
                    title.push_str(&format!("[↑ {} lines] ", view.scroll_offset));
                }

                let content_pane = Paragraph::new(content)
                    .scroll((scroll_y as u16, 0))
                    .block(
                        Block::default()
                            .title(Span::styled(title, Style::default().fg(status_color)))
                            .borders(Borders::ALL),
                    );

                frame.render_widget(content_pane, areas[1]);
            } else if active_tab < total_tabs {
                // Shell tab
                let shell_index = active_tab - service_views.len();
                if let Some(shell) = shell_tabs.get(shell_index) {
                    let terminal_text = shell.screen_contents();

                    let mut title = format!(" {} ", shell.name);
                    if shell.scroll_offset() > 0 {
                        title = format!(" {} [↑ {} lines] ", shell.name, shell.scroll_offset());
                    }

                    let terminal_pane = Paragraph::new(terminal_text).block(
                        Block::default()
                            .title(Span::styled(
                                title,
                                if shell.scroll_offset() > 0 {
                                    Style::default().fg(Color::Yellow)
                                } else {
                                    Style::default()
                                }
                            ))
                            .borders(Borders::ALL),
                    );

                    frame.render_widget(terminal_pane, areas[1]);
                }
            }

            // Render footer with help text and status summary
            let footer_lines = match &error_message {
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
                    let running_count = service_views.iter().filter(|v| matches!(v.status(), ProcessStatus::Running)).count();
                    let exited_count = service_views.iter().filter(|v| matches!(v.status(), ProcessStatus::Exited)).count();

                    // Show scroll shortcuts if viewing a shell tab
                    let is_shell_tab = active_tab >= service_views.len();

                    if is_shell_tab {
                        vec![Line::from(vec![
                            Span::raw("Tab: Switch • "),
                            Span::raw("Ctrl+T: New shell • "),
                            Span::raw("Ctrl+W: Close tab • "),
                            Span::styled("PgUp/PgDn: Scroll • ", Style::default().fg(Color::Cyan)),
                            Span::styled("End: Bottom • ", Style::default().fg(Color::Cyan)),
                            Span::raw("Ctrl+Q: Quit • "),
                            Span::styled(
                                format!("{} running", running_count),
                                Style::default().fg(Color::Green),
                            ),
                            Span::raw(" • "),
                            Span::styled(
                                format!("{} exited", exited_count),
                                Style::default().fg(Color::Red),
                            ),
                        ])]
                    } else {
                        vec![Line::from(vec![
                            Span::raw("Tab/Shift+Tab: Switch • "),
                            Span::raw("Ctrl+T: New shell • "),
                            Span::raw("Ctrl+W: Close tab • "),
                            Span::raw("Ctrl+R: Refresh • "),
                            Span::raw("q / Ctrl+C: Stop & Quit • "),
                            Span::styled(
                                format!("{} running", running_count),
                                Style::default().fg(Color::Green),
                            ),
                            Span::raw(" • "),
                            Span::styled(
                                format!("{} exited", exited_count),
                                Style::default().fg(Color::Red),
                            ),
                        ])]
                    }
                }
            };

            frame.render_widget(Paragraph::new(footer_lines), areas[2]);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Resize(cols, rows) => {
                    // Resize all shell tabs to match new terminal size
                    for shell in &mut shell_tabs {
                        if let Err(e) = shell.resize(rows.saturating_sub(6), cols) {
                            eprintln!("Failed to resize shell {}: {}", shell.name, e);
                        }
                    }
                }
                Event::Key(key) => {
                    if key.kind == event::KeyEventKind::Press {
                        let total_tabs = service_views.len() + shell_tabs.len();

                        match (key.code, key.modifiers) {
                        // Quit: Ctrl+C or Ctrl+Q always quits; 'q' quits on read-only service views
                        (KeyCode::Char('c'), KeyModifiers::CONTROL)
                        | (KeyCode::Char('\x03'), _) => break,
                        (KeyCode::Char('q'), KeyModifiers::CONTROL)
                        | (KeyCode::Char('\x11'), _) => break,
                        (KeyCode::Char('q'), KeyModifiers::NONE) if active_tab < service_views.len() => break,

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

                        // New shell tab (Ctrl+T)
                        (KeyCode::Char('t'), KeyModifiers::CONTROL)
                        | (KeyCode::Char('\x14'), _) => {
                            let (program, args) = devpane::terminal::default_shell();
                            let shell_number = shell_tabs.len() + 1;

                            // Get current terminal size
                            let size = terminal.size()?;
                            let rows = size.height.saturating_sub(6); // Account for tab bar and footer
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
                                    active_tab = service_views.len() + shell_tabs.len() - 1;
                                }
                                Err(e) => {
                                    error_message = Some(format!("Failed to create shell: {}", e));
                                }
                            }
                            continue;
                        }

                        // Close tab (Ctrl+W) - closes service tab (stops service) or closes shell tab
                        (KeyCode::Char('w'), KeyModifiers::CONTROL)
                        | (KeyCode::Char('W'), KeyModifiers::CONTROL)
                        | (KeyCode::Char('\x17'), _) => {
                            let total_tabs = service_views.len() + shell_tabs.len();
                            if total_tabs > 0 {
                                if active_tab < service_views.len() {
                                    let service_view = service_views.remove(active_tab);
                                    let session_id = service_view.session.id.clone();
                                    closed_session_ids.insert(session_id.clone());
                                    if let Ok(mut stop_stream) = devpane::ipc::connect() {
                                        let _ = devpane::protocol::write_message(
                                            &mut stop_stream,
                                            &devpane::protocol::Request::Stop {
                                                id: session_id,
                                            },
                                        );
                                        let _ = devpane::protocol::read_message::<_, devpane::protocol::Response>(
                                            &mut stop_stream,
                                        );
                                    }
                                } else {
                                    let shell_index = active_tab - service_views.len();
                                    if shell_index < shell_tabs.len() {
                                        shell_tabs.remove(shell_index);
                                    }
                                }

                                let new_total = service_views.len() + shell_tabs.len();
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

                        // Scroll up (Page Up, Shift+Up, or Up arrow on service view)
                        (KeyCode::PageUp, _) | (KeyCode::Up, KeyModifiers::SHIFT) => {
                            if active_tab < service_views.len() {
                                service_views[active_tab].scroll_up(10);
                            } else {
                                let shell_index = active_tab - service_views.len();
                                if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                    shell.scroll_up(10);
                                }
                            }
                            continue;
                        }
                        (KeyCode::Up, KeyModifiers::NONE) if active_tab < service_views.len() => {
                            service_views[active_tab].scroll_up(1);
                            continue;
                        }

                        // Scroll down (Page Down, Shift+Down, or Down arrow on service view)
                        (KeyCode::PageDown, _) | (KeyCode::Down, KeyModifiers::SHIFT) => {
                            if active_tab < service_views.len() {
                                service_views[active_tab].scroll_down(10);
                            } else {
                                let shell_index = active_tab - service_views.len();
                                if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                    shell.scroll_down(10);
                                }
                            }
                            continue;
                        }
                        (KeyCode::Down, KeyModifiers::NONE) if active_tab < service_views.len() => {
                            service_views[active_tab].scroll_down(1);
                            continue;
                        }

                        // Scroll to bottom (End)
                        (KeyCode::End, _) => {
                            if active_tab < service_views.len() {
                                service_views[active_tab].scroll_to_bottom();
                            } else {
                                let shell_index = active_tab - service_views.len();
                                if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                    shell.scroll_to_bottom();
                                }
                            }
                            continue;
                        }

                        _ => {}
                    }

                    // Send input to shell tabs only
                    if active_tab >= service_views.len() {
                        let shell_index = active_tab - service_views.len();
                        if let Some(shell) = shell_tabs.get_mut(shell_index) {
                            let input: Option<Vec<u8>> = match key.code {
                                KeyCode::Char(character) => {
                                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                                        None // Already handled above
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
                    match mouse_event.kind {
                        event::MouseEventKind::ScrollUp => {
                            if active_tab < service_views.len() {
                                service_views[active_tab].scroll_up(3);
                            } else {
                                let shell_index = active_tab - service_views.len();
                                if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                    shell.scroll_up(3);
                                }
                            }
                        }
                        event::MouseEventKind::ScrollDown => {
                            if active_tab < service_views.len() {
                                service_views[active_tab].scroll_down(3);
                            } else {
                                let shell_index = active_tab - service_views.len();
                                if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                    shell.scroll_down(3);
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
