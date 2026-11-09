use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use crate::process::session::{ProcessSession, ProcessStatus};

pub struct ServiceLogView {
    pub session: ProcessSession,
    pub log_path: PathBuf,
    pub log_content: String,
    pub file_position: u64,
    pub scroll_offset: usize,
    pub search_query: Option<String>,
}

impl ServiceLogView {
    pub fn new(session: ProcessSession, log_path: PathBuf) -> Result<Self, std::io::Error> {
        let mut view = Self {
            session,
            log_path,
            log_content: String::new(),
            file_position: 0,
            scroll_offset: 0,
            search_query: None,
        };

        view.update()?;
        Ok(view)
    }

    pub fn update(&mut self) -> Result<(), std::io::Error> {
        let mut file = match File::open(&self.log_path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Log file doesn't exist yet
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        // Detect truncation or log rotation
        if let Ok(metadata) = file.metadata() {
            if metadata.len() < self.file_position {
                self.file_position = 0;
                self.log_content.clear();
            }
        }

        // Seek to where we left off
        file.seek(SeekFrom::Start(self.file_position))?;

        let mut buffer = Vec::new();
        let bytes_read = file.read_to_end(&mut buffer)?;

        if bytes_read > 0 {
            self.log_content.push_str(&String::from_utf8_lossy(&buffer));
            self.file_position += bytes_read as u64;

            // Keep up to 500KB (~10,000 lines) of scrollback memory buffer
            const MAX_CONTENT_SIZE: usize = 500 * 1024;
            if self.log_content.len() > MAX_CONTENT_SIZE {
                let raw_start = self.log_content.len() - MAX_CONTENT_SIZE;
                let mut start = raw_start;
                while start < self.log_content.len() && !self.log_content.is_char_boundary(start) {
                    start += 1;
                }
                if let Some(newline_offset) = self.log_content[start..].find('\n') {
                    start += newline_offset + 1;
                }
                self.log_content = self.log_content[start..].to_string();
            }
        }

        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.session.name
    }

    pub fn status(&self) -> &ProcessStatus {
        &self.session.status
    }

    pub fn content(&self) -> &str {
        &self.log_content
    }

    pub fn filtered_content(&self) -> Cow<'_, str> {
        match &self.search_query {
            Some(query) if !query.is_empty() => {
                let q_lower = query.to_lowercase();
                let matching_lines: Vec<&str> = self
                    .log_content
                    .lines()
                    .filter(|line| line.to_lowercase().contains(&q_lower))
                    .collect();
                if matching_lines.is_empty() {
                    Cow::Owned(format!("[No lines match filter: \"{}\"]", query))
                } else {
                    Cow::Owned(matching_lines.join("\n"))
                }
            }
            _ => Cow::Borrowed(&self.log_content),
        }
    }

    pub fn match_count(&self) -> Option<usize> {
        let query = self.search_query.as_deref()?;
        if query.is_empty() {
            return None;
        }
        let q_lower = query.to_lowercase();
        Some(
            self.log_content
                .lines()
                .filter(|line| line.to_lowercase().contains(&q_lower))
                .count(),
        )
    }

    pub fn set_search(&mut self, query: Option<String>) {
        self.search_query = query;
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self, lines: usize, max_scroll: usize) {
        if max_scroll == 0 {
            self.scroll_offset = 0;
            return;
        }
        self.scroll_offset = (self.scroll_offset.saturating_add(lines)).min(max_scroll);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_to_top(&mut self, max_scroll: usize) {
        self.scroll_offset = max_scroll;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn clamp_scroll(&mut self, max_scroll: usize) {
        self.scroll_offset = self.scroll_offset.min(max_scroll);
    }

    pub fn detected_url(&self) -> Option<String> {
        for line in self.log_content.lines().rev() {
            if let Some(pos) = line
                .find("http://localhost:")
                .or_else(|| line.find("http://127.0.0.1:"))
                .or_else(|| line.find("https://localhost:"))
            {
                let tail = &line[pos..];
                let end = tail
                    .find(|c: char| {
                        c.is_whitespace()
                            || c == '"'
                            || c == '\''
                            || c == ')'
                            || c == ']'
                            || c == '>'
                    })
                    .unwrap_or(tail.len());
                let url = &tail[..end];
                let clean = url.trim_end_matches(['.', ';', ',']);
                if clean.len() >= 16 {
                    return Some(clean.to_string());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn mock_session() -> ProcessSession {
        ProcessSession {
            id: "test-id".to_string(),
            pid: 1234,
            name: "test-service".to_string(),
            command: "echo test".to_string(),
            start_time_seconds: 0,
            start_time_useconds: 0,
            cwd: PathBuf::from("."),
            status: ProcessStatus::Running,
            exit_code: None,
            project_dir: None,
            cpu_percent: None,
            memory_bytes: None,
        }
    }

    #[test]
    fn test_scroll_cannot_exceed_top_when_no_content_on_top() {
        let mut view = ServiceLogView {
            session: mock_session(),
            log_path: PathBuf::from("/dev/null"),
            log_content: "line 1\nline 2\nline 3".to_string(),
            file_position: 0,
            scroll_offset: 0,
            search_query: None,
        };

        // If viewport height is 10, total_lines is 3, max_scroll is 0
        let max_scroll = 0;
        view.scroll_up(5, max_scroll);
        assert_eq!(view.scroll_offset, 0, "Should not scroll up when max_scroll is 0");

        view.scroll_to_top(max_scroll);
        assert_eq!(view.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_clamped_to_max_scroll() {
        let mut view = ServiceLogView {
            session: mock_session(),
            log_path: PathBuf::from("/dev/null"),
            log_content: "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7".to_string(),
            file_position: 0,
            scroll_offset: 0,
            search_query: None,
        };

        // Total 7 lines, viewport 4 lines => max_scroll is 3
        let max_scroll = 3;
        view.scroll_up(2, max_scroll);
        assert_eq!(view.scroll_offset, 2);

        // Try to scroll up beyond top
        view.scroll_up(10, max_scroll);
        assert_eq!(view.scroll_offset, 3, "Scroll offset must be clamped to max_scroll");

        // Scroll back down
        view.scroll_down(1);
        assert_eq!(view.scroll_offset, 2);

        view.scroll_to_bottom();
        assert_eq!(view.scroll_offset, 0);
    }
}
