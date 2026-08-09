//! Domain events emitted on the app [`EventBus`](sova_core::EventBus).

use sova_core::Event;

/// Fired after a task is successfully enqueued.
#[derive(Debug, Clone)]
pub struct TaskDispatched {
    pub id: String,
    pub name: String,
    pub queue: String,
}

impl Event for TaskDispatched {
    fn name(&self) -> &'static str {
        "tasks.dispatched"
    }
}

/// Fired when a worker marks a task as terminal failure (no more retries).
#[derive(Debug, Clone)]
pub struct TaskFailed {
    pub id: String,
    pub name: String,
    pub attempts: u32,
}

impl Event for TaskFailed {
    fn name(&self) -> &'static str {
        "tasks.failed"
    }
}
