//! In-memory sent-mail recorder for tests (`Mail::fake()`).

use crate::email::EmailSnapshot;
use std::sync::{Arc, Mutex};

/// Shared inbox of emails sent through a fake transport.
#[derive(Clone, Default)]
pub struct FakeMail {
    inner: Arc<Mutex<Vec<EmailSnapshot>>>,
}

impl FakeMail {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn record(&self, snap: EmailSnapshot) {
        self.inner.lock().unwrap().push(snap);
    }

    /// Snapshot of all messages sent so far.
    pub fn sent(&self) -> Vec<EmailSnapshot> {
        self.inner.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
