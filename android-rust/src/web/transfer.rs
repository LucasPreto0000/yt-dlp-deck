use std::{
    io,
    sync::{Condvar, Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Running,
    Paused,
    Cancelled,
    Completed,
    Failed,
}

impl Phase {
    pub fn key(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "Baixando",
            Self::Paused => "Pausado",
            Self::Cancelled => "Cancelado",
            Self::Completed => "Concluído",
            Self::Failed => "Falhou",
        }
    }

    pub fn active(self) -> bool {
        matches!(self, Self::Running | Self::Paused)
    }
}

#[derive(Clone)]
pub struct Snapshot {
    pub phase: Phase,
    pub received: u64,
    pub total: Option<u64>,
    pub elapsed: Duration,
    pub lines: Vec<String>,
    pub message: String,
}

impl Snapshot {
    pub fn percent(&self) -> u8 {
        if self.phase == Phase::Completed {
            return 100;
        }
        self.total.filter(|total| *total > 0).map_or(0, |total| {
            ((u128::from(self.received) * 100 / u128::from(total)).min(99)) as u8
        })
    }
}

pub struct Transfer {
    state: Mutex<Snapshot>,
    changed: Condvar,
    started: Instant,
}

impl Transfer {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(Snapshot {
                phase: Phase::Running,
                received: 0,
                total: None,
                elapsed: Duration::ZERO,
                lines: vec![
                    "[sistema] Abrindo console Android…".into(),
                    "[sistema] Preparando transferência Rust…".into(),
                ],
                message: String::new(),
            }),
            changed: Condvar::new(),
            started: Instant::now(),
        }
    }

    fn locked(&self) -> MutexGuard<'_, Snapshot> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn snapshot(&self) -> Snapshot {
        self.locked().clone()
    }

    pub(super) fn control(&self, action: &str) {
        let mut state = self.locked();
        state.phase = match (state.phase, action) {
            (Phase::Running, "pause") => Phase::Paused,
            (Phase::Paused, "resume") => Phase::Running,
            (phase, "cancel") if phase.active() => Phase::Cancelled,
            _ => return,
        };
        if state.phase == Phase::Cancelled {
            state.message = "Download cancelado pelo usuário.".into();
        }
        self.changed.notify_all();
    }

    pub fn checkpoint(&self) -> io::Result<()> {
        let state = self
            .changed
            .wait_while(self.locked(), |state| state.phase == Phase::Paused)
            .unwrap_or_else(PoisonError::into_inner);
        if state.phase == Phase::Cancelled {
            return Err(io::Error::other("Download cancelado pelo usuário."));
        }
        Ok(())
    }

    pub fn total(&self, total: Option<u64>) {
        self.locked().total = total;
    }

    pub fn received(&self, count: usize) {
        let mut state = self.locked();
        state.received += count as u64;
        state.elapsed = self.started.elapsed();
    }

    pub(super) fn finish(&self, result: Result<String, String>) {
        let mut state = self.locked();
        if state.phase != Phase::Cancelled {
            (state.phase, state.message) = match result {
                Ok(message) => (Phase::Completed, message),
                Err(error) => (Phase::Failed, error),
            };
        }
        let prefix = if state.phase == Phase::Failed {
            "ERRO:"
        } else {
            "[sistema]"
        };
        let line = format!("{prefix} {}", state.message);
        state.lines.push(line);
    }
}
