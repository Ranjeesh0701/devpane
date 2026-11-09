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
    widgets::{
        Block, Borders, BorderType, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Tabs, Wrap,
    },
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
            let term_width = frame.area().width;
            let term_height = frame.area().height;
            let footer_height = if term_height <= 12 { 1 } else { 2 };

            let areas = Layout::vertical([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),     // Content
                Constraint::Length(footer_height),  // Footer
            ])
            .split(frame.area());

            let total_tabs = service_views.len() + shell_tabs.len();

            // Build tab titles with colored status indicators & numbers
            let mut tab_titles: Vec<Line> = service_views
                .iter()
                .enumerate()
                .map(|(idx, view)| {
                    let (status_icon, status_color) = match view.status() {
                        ProcessStatus::Running => ("●", Color::Rgb(85, 214, 162)),
                        ProcessStatus::Exited => ("○", Color::Rgb(243, 114, 128)),
                    };
                    let mut spans = vec![
                        Span::styled(format!("{}:", idx + 1), Style::default().fg(Color::DarkGray)),
                        Span::styled(status_icon, Style::default().fg(status_color)),
                        Span::raw(" "),
                        Span::styled(view.name(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ];
                    if let Some(url) = view.detected_url() {
                        if let Some(port_str) = url.rsplit(':').next().and_then(|p| p.split('/').next()) {
                            if port_str.chars().all(|c| c.is_ascii_digit()) {
                                spans.push(Span::styled(format!(" :{}", port_str), Style::default().fg(Color::Rgb(139, 156, 255))));
                            }
                        }
                    }
                    Line::from(spans)
                })
                .collect();

            // Add shell tabs with modern shell icon & number
            for (idx, shell) in shell_tabs.iter().enumerate() {
                let tab_num = service_views.len() + idx + 1;
                tab_titles.push(Line::from(vec![
                    Span::styled(format!("{}:", tab_num), Style::default().fg(Color::DarkGray)),
                    Span::styled("🐚 ", Style::default().fg(Color::Cyan)),
                    Span::styled(shell.name.as_str(), Style::default().fg(Color::White)),
                ]));
            }

            let tabs = Tabs::new(tab_titles)
                .block(
                    Block::default()
                        .title(vec![
                            Span::styled(" ⬢ Devpane ", Style::default().fg(Color::Rgb(139, 156, 255)).add_modifier(Modifier::BOLD)),
                            Span::styled("v0.4.0 ", Style::default().fg(Color::DarkGray)),
                            Span::styled("• ", Style::default().fg(Color::Rgb(55, 62, 82))),
                            Span::styled("● DAEMON ", Style::default().fg(Color::Rgb(85, 214, 162)).add_modifier(Modifier::BOLD)),
                        ])
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::Rgb(55, 62, 82))),
                )
                .select(active_tab)
                .style(Style::default().fg(Color::Rgb(140, 148, 170)))
                .highlight_style(
                    Style::default()
                        .fg(Color::Rgb(139, 156, 255))
                        .bg(Color::Rgb(34, 40, 60))
                        .add_modifier(Modifier::BOLD),
                )
                .divider(Span::styled(" │ ", Style::default().fg(Color::Rgb(55, 62, 82))));

            frame.render_widget(tabs, areas[0]);

            // Render current tab's content
            if total_tabs == 0 {
                // Show helpful modern empty state
                let help_text = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("⬢ ", Style::default().fg(Color::Rgb(139, 156, 255)).add_modifier(Modifier::BOLD)),
                        Span::styled("DEVPANE WORKSPACE", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled("No services are currently running.", Style::default().fg(Color::Rgb(140, 148, 170)))),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Quick Actions:", Style::default().fg(Color::Rgb(139, 156, 255)).add_modifier(Modifier::BOLD)),
                    ]),
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
                        Span::raw("       Quit dashboard (daemon continues in background)"),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Tip: ", Style::default().fg(Color::Rgb(85, 214, 162)).add_modifier(Modifier::BOLD)),
                        Span::styled("Define services in devpane.yaml or run 'devpane init' to auto-detect.", Style::default().fg(Color::DarkGray)),
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
                    .alignment(ratatui::layout::Alignment::Center);

                frame.render_widget(help_pane, areas[1]);
            } else if active_tab < service_views.len() {
                // Service log view
                let view = &mut service_views[active_tab];
                let viewport_height = areas[1].height.saturating_sub(2) as usize;
                let total_lines = view.filtered_content().lines().count();
                let max_scroll = total_lines.saturating_sub(viewport_height);

                // Strictly clamp scroll offset: can NEVER scroll above top or when content fits viewport!
                view.clamp_scroll(max_scroll);

                let content = view.filtered_content();

                let scroll_y = if view.scroll_offset == 0 {
                    max_scroll
                } else {
                    max_scroll.saturating_sub(view.scroll_offset)
                };

                let (status_text, status_color) = match view.status() {
                    ProcessStatus::Running => ("RUNNING", Color::Rgb(85, 214, 162)),
                    ProcessStatus::Exited => {
                        if let Some(code) = view.session.exit_code {
                            if code == 0 {
                                ("EXITED (0)", Color::Yellow)
                            } else {
                                ("FAILED", Color::Rgb(243, 114, 128))
                            }
                        } else {
                            ("EXITED", Color::Rgb(243, 114, 128))
                        }
                    }
                };

                let mut title_spans = vec![
                    Span::styled(" ● ", Style::default().fg(status_color)),
                    Span::styled(view.name(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" [{status_text} • PID {}]", view.session.pid), Style::default().fg(status_color)),
                ];

                if let Some(url) = view.detected_url() {
                    title_spans.push(Span::styled(" • ", Style::default().fg(Color::DarkGray)));
                    title_spans.push(Span::styled(format!("🌐 {url}"), Style::default().fg(Color::Rgb(139, 156, 255))));
                }

                if let Some(count) = view.match_count() {
                    title_spans.push(Span::styled(
                        format!(" • 🔍 \"{}\" ({} matches)", view.search_query.as_deref().unwrap_or(""), count),
                        Style::default().fg(Color::Yellow),
                    ));
                }

                // Scroll indicator: show LIVE TAIL, TOP, or exact offset
                if max_scroll > 0 {
                    title_spans.push(Span::styled(" • ", Style::default().fg(Color::DarkGray)));
                    if view.scroll_offset == 0 {
                        title_spans.push(Span::styled("LIVE TAIL", Style::default().fg(Color::Rgb(85, 214, 162)).add_modifier(Modifier::BOLD)));
                    } else if view.scroll_offset >= max_scroll {
                        title_spans.push(Span::styled("TOP", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                    } else {
                        let pct = ((max_scroll - view.scroll_offset) * 100) / max_scroll;
                        title_spans.push(Span::styled(
                            format!("↑ {} lines ({}%)", view.scroll_offset, pct),
                            Style::default().fg(Color::Cyan),
                        ));
                    }
                }

                let content_pane = Paragraph::new(content)
                    .scroll((scroll_y as u16, 0))
                    .block(
                        Block::default()
                            .title(Line::from(title_spans))
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Rgb(55, 62, 82))),
                    );

                frame.render_widget(content_pane, areas[1]);

                // Render modern scrollbar on right margin when content is scrollable
                if max_scroll > 0 {
                    let mut scrollbar_state = ScrollbarState::new(max_scroll)
                        .position(max_scroll.saturating_sub(view.scroll_offset));
                    frame.render_stateful_widget(
                        Scrollbar::default()
                            .orientation(ScrollbarOrientation::VerticalRight)
                            .style(Style::default().fg(Color::Rgb(139, 156, 255)))
                            .track_style(Style::default().fg(Color::Rgb(35, 40, 55)))
                            .begin_symbol(Some("▲"))
                            .end_symbol(Some("▼")),
                        areas[1],
                        &mut scrollbar_state,
                    );
                }
            } else if active_tab < total_tabs {
                // Shell tab
                let shell_index = active_tab - service_views.len();
                if let Some(shell) = shell_tabs.get(shell_index) {
                    let terminal_text = shell.screen_contents();

                    let mut title_spans = vec![
                        Span::styled(" 🐚 ", Style::default().fg(Color::Cyan)),
                        Span::styled(&shell.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled(" [INTERACTIVE SHELL]", Style::default().fg(Color::Cyan)),
                    ];

                    if shell.scroll_offset() > 0 {
                        title_spans.push(Span::styled(" • ", Style::default().fg(Color::DarkGray)));
                        title_spans.push(Span::styled(format!("↑ {} lines", shell.scroll_offset()), Style::default().fg(Color::Yellow)));
                    }

                    let terminal_pane = Paragraph::new(terminal_text).block(
                        Block::default()
                            .title(Line::from(title_spans))
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(Color::Rgb(55, 62, 82))),
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
                        let running_count = service_views.iter().filter(|v| matches!(v.status(), ProcessStatus::Running)).count();
                        let exited_count = service_views.iter().filter(|v| matches!(v.status(), ProcessStatus::Exited)).count();

                        // Show scroll shortcuts if viewing a shell tab
                        let is_shell_tab = active_tab >= service_views.len();

                        let status_span = Span::styled(
                            format!("● {}/{} running", running_count, service_views.len()),
                            Style::default().fg(Color::Rgb(85, 214, 162)).add_modifier(Modifier::BOLD),
                        );
                        let exited_span = Span::styled(
                            format!("○ {} exited", exited_count),
                            Style::default().fg(if exited_count > 0 { Color::Rgb(243, 114, 128) } else { Color::DarkGray }),
                        );

                        let active_action = action_status.as_ref().and_then(|(msg, time)| {
                            if time.elapsed() < Duration::from_secs(3) {
                                Some(msg.as_str())
                            } else {
                                None
                            }
                        });

                        if footer_height == 1 {
                            // Single-line compact mode
                            let mut spans = if is_shell_tab {
                                vec![
                                    Span::styled("[Tab]", Style::default().fg(Color::Rgb(139, 156, 255)).add_modifier(Modifier::BOLD)),
                                    Span::raw(" Switch  "),
                                    Span::styled("[^T]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                                    Span::raw(" Shell  "),
                                    Span::styled("[^W]", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
                                    Span::raw(" Close  "),
                                    Span::styled("[^Q]", Style::default().fg(Color::Rgb(243, 114, 128)).add_modifier(Modifier::BOLD)),
                                    Span::raw(" Quit  "),
                                    status_span,
                                ]
                            } else {
                                vec![
                                    Span::styled("[Tab]", Style::default().fg(Color::Rgb(139, 156, 255)).add_modifier(Modifier::BOLD)),
                                    Span::raw(" Switch  "),
                                    Span::styled("[r]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                                    Span::raw(" Restart  "),
                                    Span::styled("[s]", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                                    Span::raw(" Stop  "),
                                    Span::styled("[/]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                    Span::raw(" Filter  "),
                                    Span::styled("[q]", Style::default().fg(Color::Rgb(243, 114, 128)).add_modifier(Modifier::BOLD)),
                                    Span::raw(" Quit  "),
                                    status_span,
                                ]
                            };
                            if let Some(msg) = active_action {
                                spans.push(Span::raw("  "));
                                spans.push(Span::styled(format!("⚡ {msg}"), Style::default().fg(Color::Yellow)));
                            }
                            vec![Line::from(spans)]
                        } else {
                            // Two-line responsive mode
                            let row1_spans = if is_shell_tab {
                                if term_width >= 105 {
                                    vec![
                                        Span::styled("[Tab]", Style::default().fg(Color::Rgb(139, 156, 255)).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Switch   "),
                                        Span::styled("[Ctrl+T]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Shell   "),
                                        Span::styled("[Ctrl+W]", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Close   "),
                                        Span::styled("[PgUp/Dn]", Style::default().fg(Color::Cyan)),
                                        Span::raw(" Scroll   "),
                                        Span::styled("[Ctrl+Q]", Style::default().fg(Color::Rgb(243, 114, 128)).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Quit"),
                                    ]
                                } else {
                                    vec![
                                        Span::styled("[Tab]", Style::default().fg(Color::Rgb(139, 156, 255)).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Switch   "),
                                        Span::styled("[^T]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Shell   "),
                                        Span::styled("[^W]", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Close   "),
                                        Span::styled("[^Q]", Style::default().fg(Color::Rgb(243, 114, 128)).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Quit"),
                                    ]
                                }
                            } else {
                                let has_filter = service_views.get(active_tab).and_then(|v| v.search_query.as_ref()).is_some();
                                let has_url = service_views.get(active_tab).and_then(|v| v.detected_url()).is_some();

                                if term_width >= 115 {
                                    let mut s = vec![
                                        Span::styled("[Tab]", Style::default().fg(Color::Rgb(139, 156, 255)).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Switch   "),
                                        Span::styled("[1-9]", Style::default().fg(Color::DarkGray)),
                                        Span::raw(" Jump   "),
                                        Span::styled("[r]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Restart   "),
                                        Span::styled("[b]", Style::default().fg(Color::Yellow)),
                                        Span::raw(" Rebuild   "),
                                        Span::styled("[s]", Style::default().fg(Color::Magenta)),
                                        Span::raw(" Stop   "),
                                        Span::styled("[/]", Style::default().fg(Color::Yellow)),
                                        Span::raw(" Filter   "),
                                    ];
                                    if has_filter {
                                        s.push(Span::styled("[Esc]", Style::default().fg(Color::Yellow)));
                                        s.push(Span::raw(" Clear   "));
                                    }
                                    if has_url {
                                        s.push(Span::styled("[o]", Style::default().fg(Color::Rgb(85, 214, 162))));
                                        s.push(Span::raw(" Open   "));
                                    }
                                    s.extend(vec![
                                        Span::styled("[^T]", Style::default().fg(Color::Green)),
                                        Span::raw(" Shell   "),
                                        Span::styled("[j/k]", Style::default().fg(Color::DarkGray)),
                                        Span::raw(" Scroll   "),
                                        Span::styled("[q]", Style::default().fg(Color::Rgb(243, 114, 128))),
                                        Span::raw(" Quit"),
                                    ]);
                                    s
                                } else if term_width >= 80 {
                                    let mut s = vec![
                                        Span::styled("[Tab]", Style::default().fg(Color::Rgb(139, 156, 255)).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Switch   "),
                                        Span::styled("[r]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Restart   "),
                                        Span::styled("[s]", Style::default().fg(Color::Magenta)),
                                        Span::raw(" Stop   "),
                                        Span::styled("[/]", Style::default().fg(Color::Yellow)),
                                        Span::raw(" Filter   "),
                                    ];
                                    if has_filter {
                                        s.push(Span::styled("[Esc]", Style::default().fg(Color::Yellow)));
                                        s.push(Span::raw(" Clear   "));
                                    }
                                    s.extend(vec![
                                        Span::styled("[^T]", Style::default().fg(Color::Green)),
                                        Span::raw(" Shell   "),
                                        Span::styled("[q]", Style::default().fg(Color::Rgb(243, 114, 128))),
                                        Span::raw(" Quit"),
                                    ]);
                                    s
                                } else {
                                    vec![
                                        Span::styled("[Tab]", Style::default().fg(Color::Rgb(139, 156, 255)).add_modifier(Modifier::BOLD)),
                                        Span::raw(" Next   "),
                                        Span::styled("[r]", Style::default().fg(Color::Cyan)),
                                        Span::raw(" Restart   "),
                                        Span::styled("[q]", Style::default().fg(Color::Rgb(243, 114, 128))),
                                        Span::raw(" Quit"),
                                    ]
                                }
                            };

                            let mut row2_spans = vec![
                                status_span,
                                Span::raw("   "),
                                exited_span,
                                Span::styled("   •   devpane v0.4.0", Style::default().fg(Color::DarkGray)),
                            ];
                            if let Some(msg) = active_action {
                                row2_spans.push(Span::raw("   "));
                                row2_spans.push(Span::styled(
                                    format!("⚡ {msg}"),
                                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                                ));
                            }

                            vec![Line::from(row1_spans), Line::from(row2_spans)]
                        }
                    }
                }
            };

            frame.render_widget(Paragraph::new(footer_lines).wrap(Wrap { trim: true }), areas[2]);
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
                                    if active_tab < service_views.len() {
                                        service_views[active_tab].set_search(None);
                                    }
                                }
                                KeyCode::Enter => {
                                    is_searching = false;
                                }
                                KeyCode::Backspace => {
                                    search_input.pop();
                                    if active_tab < service_views.len() {
                                        let q = if search_input.is_empty() {
                                            None
                                        } else {
                                            Some(search_input.clone())
                                        };
                                        service_views[active_tab].set_search(q);
                                    }
                                }
                                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    search_input.push(c);
                                    if active_tab < service_views.len() {
                                        service_views[active_tab].set_search(Some(search_input.clone()));
                                    }
                                }
                                _ => {}
                            }
                            continue;
                        }

                        let total_tabs = service_views.len() + shell_tabs.len();
                        let (_term_w, term_h) = terminal.size().map(|s| (s.width, s.height)).unwrap_or((80, 24));
                        let viewport_height = term_h.saturating_sub(5) as usize;

                        match (key.code, key.modifiers) {
                        // Search in active service tab
                        (KeyCode::Char('/'), KeyModifiers::NONE) if active_tab < service_views.len() => {
                            is_searching = true;
                            search_input = service_views[active_tab]
                                .search_query
                                .clone()
                                .unwrap_or_default();
                            continue;
                        }

                        // Clear search filter with Esc if active
                        (KeyCode::Esc, KeyModifiers::NONE) if active_tab < service_views.len() => {
                            if service_views[active_tab].search_query.is_some() {
                                service_views[active_tab].set_search(None);
                                search_input.clear();
                                continue;
                            }
                        }
                        // Interactive service controls (r: Restart, b: Rebuild, s: Stop/Start toggle)
                        (KeyCode::Char('r'), KeyModifiers::NONE)
                        | (KeyCode::Char('R'), KeyModifiers::NONE) if active_tab < service_views.len() => {
                            let service_name = service_views[active_tab].name().to_string();
                            action_status = Some((format!("Restarting '{service_name}'..."), Instant::now()));
                            if let Ok(mut stream) = devpane::ipc::connect() {
                                let _ = devpane::protocol::write_message(
                                    &mut stream,
                                    &devpane::protocol::Request::Restart {
                                        name: Some(service_name.clone()),
                                        project_dir: None,
                                        services: None,
                                    },
                                );
                                let _ = devpane::protocol::read_message::<_, devpane::protocol::Response>(&mut stream);
                            }
                            last_refresh = Instant::now() - Duration::from_secs(1);
                            continue;
                        }

                        (KeyCode::Char('b'), KeyModifiers::NONE)
                        | (KeyCode::Char('B'), KeyModifiers::NONE) if active_tab < service_views.len() => {
                            let service_name = service_views[active_tab].name().to_string();
                            action_status = Some((format!("Rebuilding '{service_name}'..."), Instant::now()));
                            if let Ok(mut stream) = devpane::ipc::connect() {
                                let _ = devpane::protocol::write_message(
                                    &mut stream,
                                    &devpane::protocol::Request::Rebuild {
                                        name: service_name.clone(),
                                    },
                                );
                                let _ = devpane::protocol::read_message::<_, devpane::protocol::Response>(&mut stream);
                            }
                            last_refresh = Instant::now() - Duration::from_secs(1);
                            continue;
                        }

                        (KeyCode::Char('s'), KeyModifiers::NONE)
                        | (KeyCode::Char('S'), KeyModifiers::NONE) if active_tab < service_views.len() => {
                            let service_name = service_views[active_tab].name().to_string();
                            let is_running = matches!(service_views[active_tab].status(), ProcessStatus::Running);
                            if is_running {
                                action_status = Some((format!("Stopping '{service_name}'..."), Instant::now()));
                                let session_id = service_views[active_tab].session.id.clone();
                                if let Ok(mut stream) = devpane::ipc::connect() {
                                    let _ = devpane::protocol::write_message(
                                        &mut stream,
                                        &devpane::protocol::Request::Stop { id: session_id },
                                    );
                                    let _ = devpane::protocol::read_message::<_, devpane::protocol::Response>(&mut stream);
                                }
                            } else {
                                action_status = Some((format!("Starting '{service_name}'..."), Instant::now()));
                                if let Ok(mut stream) = devpane::ipc::connect() {
                                    let _ = devpane::protocol::write_message(
                                        &mut stream,
                                        &devpane::protocol::Request::Restart {
                                            name: Some(service_name.clone()),
                                            project_dir: None,
                                            services: None,
                                        },
                                    );
                                    let _ = devpane::protocol::read_message::<_, devpane::protocol::Response>(&mut stream);
                                }
                            }
                            last_refresh = Instant::now() - Duration::from_secs(1);
                            continue;
                        }

                        // Quit: Ctrl+C or Ctrl+Q always quits; 'q' quits on read-only service views
                        (KeyCode::Char('c'), KeyModifiers::CONTROL)
                        | (KeyCode::Char('\x03'), _) => break,
                        (KeyCode::Char('q'), KeyModifiers::CONTROL)
                        | (KeyCode::Char('\x11'), _) => break,
                        (KeyCode::Char('q'), KeyModifiers::NONE) if active_tab < service_views.len() => break,

                        // Quick Tab Switch via 1-9 keys
                        (KeyCode::Char(c @ '1'..='9'), KeyModifiers::NONE) if active_tab < service_views.len() => {
                            let target = (c as usize) - ('1' as usize);
                            if target < total_tabs {
                                active_tab = target;
                            }
                            continue;
                        }

                        // Tab navigation (Tab / Shift+Tab or Left/Right or h/l)
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
                        (KeyCode::Left, KeyModifiers::NONE) | (KeyCode::Char('h'), KeyModifiers::NONE) if active_tab < service_views.len() => {
                            if total_tabs > 0 {
                                active_tab = if active_tab == 0 { total_tabs - 1 } else { active_tab - 1 };
                            }
                            continue;
                        }
                        (KeyCode::Right, KeyModifiers::NONE) | (KeyCode::Char('l'), KeyModifiers::NONE) if active_tab < service_views.len() => {
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

                        // Scroll up (Page Up, Shift+Up, or Up arrow / 'k' on service view)
                        (KeyCode::PageUp, _) | (KeyCode::Up, KeyModifiers::SHIFT) => {
                            if active_tab < service_views.len() {
                                let max_scroll = service_views[active_tab].filtered_content().lines().count().saturating_sub(viewport_height);
                                service_views[active_tab].scroll_up(10, max_scroll);
                            } else {
                                let shell_index = active_tab - service_views.len();
                                if let Some(shell) = shell_tabs.get_mut(shell_index) {
                                    shell.scroll_up(10);
                                }
                            }
                            continue;
                        }
                        (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) if active_tab < service_views.len() => {
                            let max_scroll = service_views[active_tab].filtered_content().lines().count().saturating_sub(viewport_height);
                            service_views[active_tab].scroll_up(1, max_scroll);
                            continue;
                        }

                        // Scroll down (Page Down, Shift+Down, or Down arrow / 'j' on service view)
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
                        (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) if active_tab < service_views.len() => {
                            service_views[active_tab].scroll_down(1);
                            continue;
                        }

                        // Scroll to top (Home or 'g')
                        (KeyCode::Home, _) | (KeyCode::Char('g'), KeyModifiers::NONE) if active_tab < service_views.len() => {
                            let max_scroll = service_views[active_tab].filtered_content().lines().count().saturating_sub(viewport_height);
                            service_views[active_tab].scroll_to_top(max_scroll);
                            continue;
                        }

                        // Scroll to bottom (End or 'G')
                        (KeyCode::End, _) | (KeyCode::Char('G'), KeyModifiers::NONE) | (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
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
                    let viewport_height = terminal.size().map(|s| s.height.saturating_sub(5) as usize).unwrap_or(20);
                    match mouse_event.kind {
                        event::MouseEventKind::ScrollUp => {
                            if active_tab < service_views.len() {
                                let max_scroll = service_views[active_tab]
                                    .filtered_content()
                                    .lines()
                                    .count()
                                    .saturating_sub(viewport_height);
                                service_views[active_tab].scroll_up(3, max_scroll);
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
