//! In-memory pub/sub rooms for WebSocket sessions.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

type ClientId = u64;

type Sender = mpsc::UnboundedSender<Message>;
type RoomClients = HashMap<ClientId, Sender>;
type Rooms = HashMap<String, RoomClients>;

#[derive(Clone)]
struct HubInner {
    rooms: Arc<Mutex<Rooms>>,
    next_id: Arc<AtomicU64>,
}

/// Shared room hub installed by [`crate::Ws`].
#[derive(Clone)]
pub struct Hub {
    inner: HubInner,
}

impl Hub {
    pub(crate) fn new() -> Self {
        Self {
            inner: HubInner {
                rooms: Arc::new(Mutex::new(HashMap::new())),
                next_id: Arc::new(AtomicU64::new(1)),
            },
        }
    }

    pub(crate) fn register(
        &self,
        room: &str,
        tx: mpsc::UnboundedSender<Message>,
    ) -> (ClientId, RoomHandle) {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        self.inner
            .rooms
            .lock()
            .unwrap()
            .entry(room.to_string())
            .or_default()
            .insert(id, tx);
        (
            id,
            RoomHandle {
                inner: Arc::new(RoomHandleInner {
                    hub: self.inner.clone(),
                    room: room.to_string(),
                    id,
                }),
            },
        )
    }

    fn unregister(&self, room: &str, id: ClientId) {
        let mut rooms = self.inner.rooms.lock().unwrap();
        if let Some(clients) = rooms.get_mut(room) {
            clients.remove(&id);
            if clients.is_empty() {
                rooms.remove(room);
            }
        }
    }

    /// Send a message to every client in `room`.
    pub async fn broadcast(&self, room: &str, msg: Message) {
        let senders: Vec<mpsc::UnboundedSender<Message>> = {
            let rooms = self.inner.rooms.lock().unwrap();
            rooms
                .get(room)
                .map(|clients| clients.values().cloned().collect())
                .unwrap_or_default()
        };
        for tx in senders {
            let _ = tx.send(msg.clone());
        }
    }
}

struct RoomHandleInner {
    hub: HubInner,
    room: String,
    id: ClientId,
}

/// Handle for a room membership; dropped automatically on leave / session end.
#[derive(Clone)]
pub struct RoomHandle {
    inner: Arc<RoomHandleInner>,
}

impl RoomHandle {
    pub fn room(&self) -> &str {
        &self.inner.room
    }
}

impl Drop for RoomHandleInner {
    fn drop(&mut self) {
        Hub {
            inner: self.hub.clone(),
        }
        .unregister(&self.room, self.id);
    }
}
