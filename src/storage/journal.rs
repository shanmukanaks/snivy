use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Serialize;

#[derive(Clone)]
pub struct Journal {
    path: PathBuf,
}

impl Journal {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    pub fn append<T: Serialize>(&self, record: &T) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let entry = serde_json::json!({
            "ts": Utc::now(),
            "record": record
        });
        writeln!(file, "{}", serde_json::to_string(&entry).unwrap())?;
        Ok(())
    }
}
