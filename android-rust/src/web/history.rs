use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, PoisonError},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Record {
    pub title: String,
    pub message: String,
    pub status: String,
    pub percent: u8,
    pub date: String,
    pub time: String,
}

pub(super) struct History {
    path: Option<PathBuf>,
    records: Mutex<Vec<Record>>,
}

impl History {
    pub fn load(path: Option<PathBuf>) -> Self {
        let records = path
            .as_ref()
            .and_then(|path| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            records: Mutex::new(records),
        }
    }

    pub fn records(&self) -> Vec<Record> {
        self.records
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn append(&self, record: Record) {
        let mut records = self.records.lock().unwrap_or_else(PoisonError::into_inner);
        records.insert(0, record);
        records.truncate(40);
        self.save(&records);
    }

    pub fn clear(&self) {
        let mut records = self.records.lock().unwrap_or_else(PoisonError::into_inner);
        records.clear();
        self.save(&records);
    }

    fn save(&self, records: &[Record]) {
        let Some(path) = &self.path else {
            return;
        };
        let Ok(json) = serde_json::to_vec(records) else {
            return;
        };
        let temporary = path.with_extension("next");
        if fs::write(&temporary, json).is_ok() {
            let _ = fs::rename(temporary, path);
        }
    }
}
