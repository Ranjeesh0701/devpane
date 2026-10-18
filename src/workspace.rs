use std::path::PathBuf;
use vt100::Parser;

use crate::terminal::TerminalSession;

pub struct TerminalPane {
    pub session: TerminalSession,
    pub parser: Parser,
    pub name: String,
    pub pid: u32,
    pub cwd: PathBuf,
    pub scroll_offset: usize,
}

impl TerminalPane {
    pub fn new(
        name: String,
        program: &str,
        args: &[String],
        cwd: PathBuf,
        rows: u16,
        cols: u16,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let session = TerminalSession::spawn(program, args, &cwd, rows, cols)?;
        let parser = Parser::new(rows, cols, 1000); // 1000 lines of scrollback

        Ok(Self {
            session,
            parser,
            name,
            pid: 0, // TODO: get actual PID from session
            cwd,
            scroll_offset: 0,
        })
    }

    pub fn update(&mut self) {
        loop {
            match self.session.try_read_output() {
                Ok(bytes) => {
                    self.parser.process(&bytes);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
    }

    pub fn screen_contents(&self) -> String {
        self.parser.screen().contents()
    }

    pub fn send_input(&mut self, input: &[u8]) -> std::io::Result<()> {
        self.session.send_input(input)
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.session.resize(rows, cols)?;
        self.parser.set_size(rows, cols);
        Ok(())
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
        self.parser.set_scrollback(self.scroll_offset);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.parser.set_scrollback(self.scroll_offset);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.parser.set_scrollback(0);
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
}

pub struct Tab {
    pub name: String,
    pub panes: Vec<TerminalPane>,
    pub focused_pane: usize,
}

impl Tab {
    pub fn single_pane(name: String, pane: TerminalPane) -> Self {
        Self {
            name,
            panes: vec![pane],
            focused_pane: 0,
        }
    }

    pub fn update(&mut self) {
        for pane in &mut self.panes {
            pane.update();
        }
    }

    pub fn focused_pane_mut(&mut self) -> Option<&mut TerminalPane> {
        self.panes.get_mut(self.focused_pane)
    }
}

pub struct Workspace {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
        }
    }

    pub fn add_tab(&mut self, name: String, pane: TerminalPane) {
        self.tabs.push(Tab::single_pane(name, pane));
    }

    pub fn current_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    pub fn current_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
        }
    }

    pub fn close_current_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.tabs.remove(self.active_tab);

            if self.active_tab >= self.tabs.len() && self.active_tab > 0 {
                self.active_tab -= 1;
            }
        }
    }

    pub fn update_all(&mut self) {
        for tab in &mut self.tabs {
            tab.update();
        }
    }
}
