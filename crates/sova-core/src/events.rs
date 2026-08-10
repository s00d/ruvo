//! In-process application events (`EventBus`).

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Typed application event.
pub trait Event: Send + Sync + 'static {
    fn name(&self) -> &'static str;
}

type DynListener = Arc<dyn Fn(&dyn Any) + Send + Sync>;

/// Sync event bus: listeners run in the dispatching task (order of registration).
#[derive(Clone, Default)]
pub struct EventBus {
    inner: Arc<Mutex<HashMap<TypeId, Vec<DynListener>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a typed listener. Multiple listeners per event type are allowed.
    pub fn listen<E, F>(&self, f: F)
    where
        E: Event,
        F: Fn(&E) + Send + Sync + 'static,
    {
        let mut map = self.inner.lock().expect("EventBus");
        map.entry(TypeId::of::<E>())
            .or_default()
            .push(Arc::new(move |any| {
                if let Some(e) = any.downcast_ref::<E>() {
                    f(e);
                }
            }));
    }

    /// Dispatch `event` to all listeners for `E` (registration order).
    pub fn dispatch<E: Event>(&self, event: E) {
        let listeners = {
            let map = self.inner.lock().expect("EventBus");
            map.get(&TypeId::of::<E>()).cloned().unwrap_or_default()
        };
        for listener in &listeners {
            listener(&event);
        }
    }

    /// Number of listeners registered for `E` (tests / diagnostics).
    pub fn listener_count<E: Event>(&self) -> usize {
        self.inner
            .lock()
            .expect("EventBus")
            .get(&TypeId::of::<E>())
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Ping;
    impl Event for Ping {
        fn name(&self) -> &'static str {
            "ping"
        }
    }

    #[test]
    fn order_and_multiple() {
        let bus = EventBus::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        let a = Arc::clone(&log);
        let b = Arc::clone(&log);
        bus.listen::<Ping, _>(move |_| a.lock().unwrap().push(1));
        bus.listen::<Ping, _>(move |_| b.lock().unwrap().push(2));
        assert_eq!(bus.listener_count::<Ping>(), 2);
        bus.dispatch(Ping);
        assert_eq!(*log.lock().unwrap(), vec![1, 2]);
    }

    #[test]
    fn no_listeners_ok() {
        let bus = EventBus::new();
        let n = AtomicUsize::new(0);
        bus.dispatch(Ping);
        assert_eq!(n.load(Ordering::SeqCst), 0);
    }
}
