use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct CancellableRun {
    cancel: Arc<AtomicBool>,
    running: AtomicBool,
}

pub struct ClaimedRun<'a> {
    run: &'a CancellableRun,
}

impl Drop for ClaimedRun<'_> {
    fn drop(&mut self) {
        self.run.running.store(false, Ordering::Release);
    }
}

impl CancellableRun {
    pub fn claim(&self, already_running: &str) -> Result<ClaimedRun<'_>, String> {
        self.running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| already_running.to_string())?;
        self.cancel.store(false, Ordering::Release);
        Ok(ClaimedRun { run: self })
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }

    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }
}
