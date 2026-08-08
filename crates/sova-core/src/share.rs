//! Lightweight handles for sharing values across tasks / handlers.
//!
//! Inspired by the idea of thread-share (convenient cross-task handles), but only
//! two primitives — no worker managers or macros.
//!
//! | Type | Use for | Notes |
//! |------|---------|--------|
//! | [`Cell`] | `Clone` data (counters, flags, config) | `get` / `set` / `update` / [`Cell::changed`] |
//! | [`Slot`] | ownership handoff (sockets, streams, …) | `put` / `take` / `try_take` |
//!
//! Both are cheap [`Clone`] handles (`Arc` inside). Typical wiring:
//!
//! ```ignore
//! let inbox = Slot::<TcpStream>::new();
//! let n = Cell::new(0u64);
//! app.state(inbox.clone()).state(n);
//! // BackgroundService: inbox.put(stream);
//! // Handler: let s = req.state::<Slot<TcpStream>>().take().await;
//! ```

use std::sync::{Arc, Mutex};
use tokio::sync::watch;

/// Shared `Clone` value with change notifications (`tokio::sync::watch` under the hood).
///
/// Prefer this for counters and config. For non-[`Clone`] values (sockets), use [`Slot`].
pub struct Cell<T: Clone> {
    tx: watch::Sender<T>,
}

impl<T: Clone> Clone for Cell<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<T: Clone> Cell<T> {
    pub fn new(init: T) -> Self {
        let (tx, _) = watch::channel(init);
        Self { tx }
    }

    pub fn get(&self) -> T {
        self.tx.borrow().clone()
    }

    pub fn set(&self, value: T) {
        self.tx.send_modify(|cur| *cur = value);
    }

    /// Replace the value with `f(&current)`.
    pub fn update(&self, f: impl FnOnce(&T) -> T) {
        self.tx.send_modify(|cur| {
            *cur = f(cur);
        });
    }

    /// Wait until the value changes from the moment this future starts, then return it.
    pub async fn changed(&self) -> T {
        let mut rx = self.tx.subscribe();
        let _ = rx.changed().await;
        let value = rx.borrow().clone();
        value
    }
}

/// One-item ownership handoff between tasks (sockets, streams, anything non-`Clone`).
///
/// [`Slot::put`] stores a value. If the slot was already full, the previous value is
/// **dropped** (no queue). [`Slot::take`] waits until a value is available.
pub struct Slot<T> {
    inner: Arc<SlotInner<T>>,
}

struct SlotInner<T> {
    slot: Mutex<Option<T>>,
    tick: watch::Sender<u64>,
}

impl<T> Clone for Slot<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Slot<T> {
    pub fn new() -> Self {
        let (tick, _) = watch::channel(0u64);
        Self {
            inner: Arc::new(SlotInner {
                slot: Mutex::new(None),
                tick,
            }),
        }
    }

    /// Store `value`. Replaces (and drops) any unread previous value.
    pub fn put(&self, value: T) {
        {
            let mut g = self.inner.slot.lock().unwrap();
            *g = Some(value);
        }
        self.inner.tick.send_modify(|n| *n = n.wrapping_add(1));
    }

    pub fn try_take(&self) -> Option<T> {
        self.inner.slot.lock().unwrap().take()
    }

    /// Wait until a value is available, then take it.
    pub async fn take(&self) -> T {
        let mut rx = self.inner.tick.subscribe();
        loop {
            if let Some(v) = self.try_take() {
                return v;
            }
            if rx.changed().await.is_err() {
                if let Some(v) = self.try_take() {
                    return v;
                }
                std::future::pending::<()>().await;
            }
        }
    }
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn cell_get_set_update() {
        let c = Cell::new(1u32);
        assert_eq!(c.get(), 1);
        c.set(2);
        assert_eq!(c.get(), 2);
        c.update(|n| n + 3);
        assert_eq!(c.get(), 5);
    }

    #[tokio::test]
    async fn cell_changed_wakes() {
        let c = Cell::new(0u32);
        let c2 = c.clone();
        let h = tokio::spawn(async move { c2.changed().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        c.set(7);
        assert_eq!(h.await.unwrap(), 7);
    }

    #[tokio::test]
    async fn slot_put_take() {
        let s = Slot::new();
        s.put(String::from("hi"));
        assert_eq!(s.try_take().as_deref(), Some("hi"));
        assert!(s.try_take().is_none());
    }

    #[tokio::test]
    async fn slot_take_waits() {
        let s = Slot::new();
        let s2 = s.clone();
        let h = tokio::spawn(async move { s2.take().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        s.put(42u8);
        assert_eq!(h.await.unwrap(), 42);
    }

    #[tokio::test]
    async fn slot_put_replaces() {
        let s = Slot::new();
        s.put(1u8);
        s.put(2u8);
        assert_eq!(s.try_take(), Some(2));
    }
}
