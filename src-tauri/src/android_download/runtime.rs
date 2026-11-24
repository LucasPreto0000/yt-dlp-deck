use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Running,
    Paused,
    Cancelled,
}

struct Job {
    id: String,
    phase: Phase,
}

#[derive(Default)]
pub(super) struct DownloadRuntime {
    current: Mutex<Option<Job>>,
    changed: Condvar,
}

impl DownloadRuntime {
    pub(super) const fn new() -> Self {
        Self {
            current: Mutex::new(None),
            changed: Condvar::new(),
        }
    }

    fn current(&self) -> MutexGuard<'_, Option<Job>> {
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn begin(&self, id: String) -> bool {
        let mut current = self.current();
        if current.is_some() {
            return false;
        }
        *current = Some(Job {
            id,
            phase: Phase::Running,
        });
        true
    }

    pub(super) fn active_id(&self) -> Option<String> {
        self.current().as_ref().map(|job| job.id.clone())
    }

    fn change_phase(&self, phase: Phase) -> bool {
        let mut current = self.current();
        let Some(job) = current.as_mut() else {
            return false;
        };
        if job.phase == Phase::Cancelled && phase != Phase::Cancelled {
            return false;
        }
        job.phase = phase;
        self.changed.notify_all();
        true
    }

    pub(super) fn pause(&self) -> bool {
        self.change_phase(Phase::Paused)
    }

    pub(super) fn resume(&self) -> bool {
        self.change_phase(Phase::Running)
    }

    pub(super) fn cancel(&self) -> bool {
        self.change_phase(Phase::Cancelled)
    }

    pub(super) fn is_paused(&self) -> bool {
        self.current()
            .as_ref()
            .is_some_and(|job| job.phase == Phase::Paused)
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.current()
            .as_ref()
            .is_some_and(|job| job.phase == Phase::Cancelled)
    }

    pub(super) fn checkpoint(&self) -> bool {
        let current = self
            .changed
            .wait_while(self.current(), |job| {
                job.as_ref().is_some_and(|job| job.phase == Phase::Paused)
            })
            .unwrap_or_else(PoisonError::into_inner);
        !current
            .as_ref()
            .is_some_and(|job| job.phase == Phase::Cancelled)
    }

    pub(super) fn finish(&self, id: &str) {
        let mut current = self.current();
        if current.as_ref().is_some_and(|job| job.id == id) {
            *current = None;
            self.changed.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, thread, time::Duration};

    #[test]
    fn rejects_concurrent_downloads_and_preserves_the_active_id() {
        let runtime = DownloadRuntime::new();
        assert!(runtime.begin("first".into()));
        assert!(!runtime.begin("second".into()));
        runtime.finish("second");
        assert_eq!(runtime.active_id().as_deref(), Some("first"));
        runtime.finish("first");
        assert_eq!(runtime.active_id(), None);
        assert!(runtime.begin("second".into()));
    }

    #[test]
    fn paused_worker_resumes_or_observes_cancellation() {
        let runtime = DownloadRuntime::new();
        assert!(runtime.begin("job".into()));
        for should_resume in [true, false] {
            assert!(runtime.pause());
            assert!(runtime.is_paused());
            thread::scope(|scope| {
                let (sent, received) = mpsc::channel();
                let worker_runtime = &runtime;
                scope.spawn(move || sent.send(worker_runtime.checkpoint()).unwrap());
                let paused = received.recv_timeout(Duration::from_millis(25)).is_err();
                if should_resume {
                    assert!(runtime.resume());
                } else {
                    assert!(runtime.cancel());
                }
                assert!(paused);
                assert_eq!(
                    received.recv_timeout(Duration::from_secs(2)).unwrap(),
                    should_resume
                );
            });
        }
        assert!(runtime.is_cancelled());
        assert!(!runtime.is_paused());
        assert!(!runtime.resume());
        runtime.finish("job");
        assert!(!runtime.is_cancelled());
    }
}
