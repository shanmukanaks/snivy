use std::fs;
use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};
use serde_json;
use tracing::info;

use crate::errors::AppResult;

#[derive(Clone)]
pub struct SnapshotStore {
    path: PathBuf,
}

impl SnapshotStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn save<T: Serialize>(&self, name: &str, payload: &T) -> AppResult<()> {
        fs::create_dir_all(&self.path)?;
        let file = self.path.join(format!("{name}.json"));
        let data = serde_json::to_vec_pretty(payload)?;
        fs::write(&file, data)?;
        info!(target = "snapshot", file = %file.display(), "snapshot saved");
        Ok(())
    }

    pub fn load<T: DeserializeOwned>(&self, name: &str) -> AppResult<Option<T>> {
        let file = self.path.join(format!("{name}.json"));
        if !file.exists() {
            return Ok(None);
        }
        let bytes = fs::read(file)?;
        let payload = serde_json::from_slice(&bytes)?;
        Ok(Some(payload))
    }
}
