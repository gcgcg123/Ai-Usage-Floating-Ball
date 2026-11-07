use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::providers::ProviderSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotFile {
    version: u32,
    snapshots: HashMap<String, ProviderSnapshot>,
}

impl Default for SnapshotFile {
    fn default() -> Self {
        Self {
            version: 1,
            snapshots: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct SnapshotStore {
    path: PathBuf,
}

impl SnapshotStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> HashMap<String, ProviderSnapshot> {
        if !self.path.exists() {
            return HashMap::new();
        }
        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(error) => {
                eprintln!("Failed to read snapshots: {error}");
                return HashMap::new();
            }
        };
        match serde_json::from_str::<SnapshotFile>(&raw) {
            Ok(file) => file.snapshots,
            Err(error) => {
                let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
                let backup = self
                    .path
                    .with_file_name(format!("snapshots.corrupt-{timestamp}.json"));
                if let Err(copy_error) = fs::copy(&self.path, &backup) {
                    eprintln!("Failed to preserve corrupt snapshots: {copy_error}");
                }
                eprintln!(
                    "Failed to parse snapshots: {error}. Backup: {}",
                    backup.display()
                );
                HashMap::new()
            }
        }
    }

    pub fn save(&self, snapshots: &HashMap<String, ProviderSnapshot>) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create snapshot directory: {error}"))?;
        }
        let file = SnapshotFile {
            version: 1,
            snapshots: snapshots.clone(),
        };
        let serialized = serde_json::to_vec_pretty(&file)
            .map_err(|error| format!("failed to serialize snapshots: {error}"))?;
        fs::write(&self.path, serialized)
            .map_err(|error| format!("failed to write snapshots: {error}"))
    }
}
