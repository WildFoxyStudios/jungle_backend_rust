use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Global connection hub managing all connected WebSocket clients.
/// Thread-safe: uses DashMap for concurrent access.
#[derive(Clone)]
pub struct ConnectionHub {
    /// user_id → broadcast sender for that user's channel
    connections: Arc<DashMap<i64, broadcast::Sender<WsMessage>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WsMessage {
    pub event: String,
    pub data: serde_json::Value,
}

impl ConnectionHub {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    /// Register a user and return a receiver for their channel
    pub fn subscribe(&self, user_id: i64) -> broadcast::Receiver<WsMessage> {
        let entry = self.connections.entry(user_id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(256);
            tx
        });
        entry.subscribe()
    }

    /// Remove a user's connection. If there are no more receivers, clean up.
    pub fn unsubscribe(&self, user_id: i64) {
        if let Some(entry) = self.connections.get(&user_id)
            && entry.receiver_count() <= 1 {
                drop(entry);
                self.connections.remove(&user_id);
            }
    }

    /// Send a message to a specific user
    pub fn send_to_user(&self, user_id: i64, msg: WsMessage) {
        if let Some(sender) = self.connections.get(&user_id) {
            let _ = sender.send(msg);
        }
    }

    /// Send a message to multiple users
    pub fn send_to_users(&self, user_ids: &[i64], msg: WsMessage) {
        for uid in user_ids {
            self.send_to_user(*uid, msg.clone());
        }
    }

    /// Check if a user is online
    pub fn is_online(&self, user_id: i64) -> bool {
        self.connections.contains_key(&user_id)
    }

    /// Get all online user IDs
    pub fn online_users(&self) -> Vec<i64> {
        self.connections.iter().map(|e| *e.key()).collect()
    }

    /// Get count of online users
    pub fn online_count(&self) -> usize {
        self.connections.len()
    }

    /// Broadcast a message to all connected users
    pub fn broadcast(&self, msg: WsMessage) {
        for entry in self.connections.iter() {
            let _ = entry.value().send(msg.clone());
        }
    }
}
