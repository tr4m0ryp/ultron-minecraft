//! Reads the lines appended to the control file since the last tick.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub struct Inbox {
    path: PathBuf,
    offset: u64,
    partial: String,
}

impl Inbox {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            offset: 0,
            partial: String::new(),
        }
    }

    /// Complete lines appended since the last call; a missing file reads as empty.
    pub fn read_new(&mut self) -> Vec<String> {
        let Ok(mut f) = File::open(&self.path) else {
            return Vec::new();
        };
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.offset {
            self.offset = 0;
            self.partial.clear();
        }
        if len == self.offset || f.seek(SeekFrom::Start(self.offset)).is_err() {
            return Vec::new();
        }
        let mut buf = String::new();
        let Ok(n) = f.read_to_string(&mut buf) else {
            return Vec::new();
        };
        self.offset += n as u64;
        self.partial.push_str(&buf);
        let mut lines: Vec<String> = self.partial.split('\n').map(str::to_string).collect();
        self.partial = lines.pop().unwrap_or_default();
        lines
    }
}
