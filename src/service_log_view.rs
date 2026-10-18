use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use crate::process::session::{ProcessSession, ProcessStatus};

pub struct ServiceLogView {
    pub session: ProcessSession,
    pub log_path: PathBuf,
    pub log_content: String,
    pub file_position: u64,
}

impl ServiceLogView {
    pub fn new(session: ProcessSession, log_path: PathBuf) -> Result<Self, std::io::Error> {
        let mut view = Self {
            session,
            log_path,
            log_content: String::new(),
            file_position: 0,
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

        // Seek to where we left off
        file.seek(SeekFrom::Start(self.file_position))?;

        let mut buffer = Vec::new();
        let bytes_read = file.read_to_end(&mut buffer)?;

        if bytes_read > 0 {
            self.log_content.push_str(&String::from_utf8_lossy(&buffer));
            self.file_position += bytes_read as u64;

            // Keep only the last 50KB to avoid memory issues
            const MAX_CONTENT_SIZE: usize = 50 * 1024;
            if self.log_content.len() > MAX_CONTENT_SIZE {
                let start = self.log_content.len() - MAX_CONTENT_SIZE;
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

    pub fn detected_url(&self) -> Option<String> {
        for line in self.log_content.lines().rev() {
            if let Some(pos) = line
                .find("http://localhost:")
                .or_else(|| line.find("http://127.0.0.1:"))
                .or_else(|| line.find("https://localhost:"))
            {
                let tail = &line[pos..];
                let end = tail
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ')' || c == ']' || c == '>')
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
