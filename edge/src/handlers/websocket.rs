//! WebSocket Handler for Edge Functions
//!
//! Provides real-time bidirectional communication:
//! - WebSocket upgrade handling
//! - Message routing and broadcasting
//! - Connection state management
//! - Heartbeat/keepalive support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::{
    console_log, Date, Headers, Request, Response, Result, WebSocketPair,
};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    /// Text message
    Text(String),
    /// JSON payload
    Json(serde_json::Value),
    /// Binary data (base64 encoded)
    Binary(String),
    /// Ping/pong for keepalive
    Ping,
    Pong,
    /// Connection events
    Connected { client_id: String },
    Disconnected { client_id: String, reason: String },
    /// Error message
    Error { code: u32, message: String },
}

/// WebSocket connection configuration
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u32,
    /// Enable compression
    pub compression: bool,
    /// Allowed origins (CORS)
    pub allowed_origins: Vec<String>,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            max_message_size: 64 * 1024, // 64KB
            heartbeat_interval: 30,
            connection_timeout: 300,
            compression: true,
            allowed_origins: vec!["*".to_string()],
        }
    }
}

/// WebSocket handler for edge functions
pub struct WsHandler {
    config: WsConfig,
}

impl WsHandler {
    /// Create a new WebSocket handler
    pub fn new(config: WsConfig) -> Self {
        Self { config }
    }

    /// Create with default config
    pub fn default() -> Self {
        Self::new(WsConfig::default())
    }

    /// Check if request is a WebSocket upgrade
    pub fn is_upgrade_request(req: &Request) -> bool {
        if let Ok(Some(upgrade)) = req.headers().get("Upgrade") {
            upgrade.to_lowercase() == "websocket"
        } else {
            false
        }
    }

    /// Handle WebSocket upgrade request
    pub fn handle_upgrade(&self, req: &Request) -> Result<Response> {
        // Validate origin if configured
        if !self.validate_origin(req)? {
            return Response::error("Origin not allowed", 403);
        }

        // Create WebSocket pair
        let WebSocketPair { client, server } = WebSocketPair::new()?;

        // Accept the WebSocket connection on the server side
        server.accept()?;

        // Generate client ID
        let client_id = generate_client_id();

        // Send connected message
        let connected_msg = WsMessage::Connected {
            client_id: client_id.clone(),
        };
        if let Ok(msg) = serde_json::to_string(&connected_msg) {
            let _ = server.send_with_str(&msg);
        }

        console_log!("WebSocket connected: {}", client_id);

        // Build upgrade response with the client WebSocket
        let mut headers = Headers::new();
        headers.set("Content-Type", "application/json")?;

        Response::from_websocket(client)
    }

    /// Validate request origin
    fn validate_origin(&self, req: &Request) -> Result<bool> {
        if self.config.allowed_origins.contains(&"*".to_string()) {
            return Ok(true);
        }

        if let Ok(Some(origin)) = req.headers().get("Origin") {
            Ok(self.config.allowed_origins.contains(&origin))
        } else {
            Ok(false)
        }
    }

    /// Create a WebSocket message response
    pub fn create_message(msg_type: &str, data: serde_json::Value) -> String {
        let msg = serde_json::json!({
            "type": msg_type,
            "data": data,
            "timestamp": current_timestamp()
        });
        serde_json::to_string(&msg).unwrap_or_default()
    }

    /// Parse incoming WebSocket message
    pub fn parse_message(raw: &str) -> Option<WsMessage> {
        serde_json::from_str(raw).ok()
    }
}

/// WebSocket room for pub/sub messaging
pub struct WsRoom {
    /// Room identifier
    pub id: String,
    /// Connected client IDs
    pub clients: Vec<String>,
    /// Room metadata
    pub metadata: HashMap<String, String>,
    /// Created timestamp
    pub created_at: u64,
}

impl WsRoom {
    /// Create a new room
    pub fn new(id: String) -> Self {
        Self {
            id,
            clients: Vec::new(),
            metadata: HashMap::new(),
            created_at: current_timestamp(),
        }
    }

    /// Add client to room
    pub fn add_client(&mut self, client_id: String) {
        if !self.clients.contains(&client_id) {
            self.clients.push(client_id);
        }
    }

    /// Remove client from room
    pub fn remove_client(&mut self, client_id: &str) {
        self.clients.retain(|c| c != client_id);
    }

    /// Check if client is in room
    pub fn has_client(&self, client_id: &str) -> bool {
        self.clients.iter().any(|c| c == client_id)
    }

    /// Get client count
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

/// Channel-based pub/sub system
pub struct PubSub {
    /// Active channels
    channels: HashMap<String, Vec<String>>,
}

impl PubSub {
    /// Create new pub/sub system
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    /// Subscribe client to channel
    pub fn subscribe(&mut self, channel: &str, client_id: String) {
        self.channels
            .entry(channel.to_string())
            .or_insert_with(Vec::new)
            .push(client_id);
    }

    /// Unsubscribe client from channel
    pub fn unsubscribe(&mut self, channel: &str, client_id: &str) {
        if let Some(clients) = self.channels.get_mut(channel) {
            clients.retain(|c| c != client_id);
        }
    }

    /// Get subscribers for a channel
    pub fn get_subscribers(&self, channel: &str) -> Vec<String> {
        self.channels.get(channel).cloned().unwrap_or_default()
    }

    /// Create broadcast message for channel
    pub fn create_broadcast(channel: &str, data: serde_json::Value) -> String {
        let msg = serde_json::json!({
            "type": "broadcast",
            "channel": channel,
            "data": data,
            "timestamp": current_timestamp()
        });
        serde_json::to_string(&msg).unwrap_or_default()
    }
}

impl Default for PubSub {
    fn default() -> Self {
        Self::new()
    }
}

/// Real-time event stream
pub struct EventStream {
    /// Stream ID
    pub id: String,
    /// Event buffer
    events: Vec<StreamEvent>,
    /// Max buffer size
    max_buffer: usize,
}

/// Stream event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Event ID
    pub id: String,
    /// Event type
    pub event_type: String,
    /// Event data
    pub data: serde_json::Value,
    /// Timestamp
    pub timestamp: u64,
}

impl EventStream {
    /// Create new event stream
    pub fn new(id: String) -> Self {
        Self {
            id,
            events: Vec::new(),
            max_buffer: 1000,
        }
    }

    /// Push event to stream
    pub fn push(&mut self, event_type: String, data: serde_json::Value) -> StreamEvent {
        let event = StreamEvent {
            id: generate_event_id(),
            event_type,
            data,
            timestamp: current_timestamp(),
        };

        self.events.push(event.clone());

        // Evict old events if buffer full
        if self.events.len() > self.max_buffer {
            self.events.remove(0);
        }

        event
    }

    /// Get events since ID
    pub fn since(&self, last_id: Option<&str>) -> Vec<&StreamEvent> {
        match last_id {
            Some(id) => {
                let mut found = false;
                self.events
                    .iter()
                    .filter(|e| {
                        if found {
                            return true;
                        }
                        if e.id == id {
                            found = true;
                        }
                        false
                    })
                    .collect()
            }
            None => self.events.iter().collect(),
        }
    }
}

/// Cursor-based pagination for real-time data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    /// Position identifier
    pub position: String,
    /// Timestamp
    pub timestamp: u64,
    /// Direction (forward/backward)
    pub direction: CursorDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CursorDirection {
    Forward,
    Backward,
}

impl Cursor {
    /// Create new forward cursor
    pub fn forward(position: String) -> Self {
        Self {
            position,
            timestamp: current_timestamp(),
            direction: CursorDirection::Forward,
        }
    }

    /// Encode cursor to string
    pub fn encode(&self) -> String {
        // Simple base64 encoding
        let json = serde_json::to_string(self).unwrap_or_default();
        base64_encode(&json)
    }

    /// Decode cursor from string
    pub fn decode(encoded: &str) -> Option<Self> {
        let json = base64_decode(encoded)?;
        serde_json::from_str(&json).ok()
    }
}

// Helper functions

fn generate_client_id() -> String {
    format!("client_{}", current_timestamp())
}

fn generate_event_id() -> String {
    format!("evt_{}", current_timestamp())
}

fn current_timestamp() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        Date::now().as_millis()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

fn base64_encode(input: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.encode(input.as_bytes())
}

fn base64_decode(input: &str) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD
        .decode(input)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_config_default() {
        let config = WsConfig::default();
        assert_eq!(config.max_message_size, 64 * 1024);
        assert_eq!(config.heartbeat_interval, 30);
    }

    #[test]
    fn test_ws_room() {
        let mut room = WsRoom::new("test-room".to_string());
        room.add_client("client1".to_string());
        room.add_client("client2".to_string());

        assert_eq!(room.client_count(), 2);
        assert!(room.has_client("client1"));

        room.remove_client("client1");
        assert_eq!(room.client_count(), 1);
        assert!(!room.has_client("client1"));
    }

    #[test]
    fn test_pubsub() {
        let mut pubsub = PubSub::new();
        pubsub.subscribe("news", "client1".to_string());
        pubsub.subscribe("news", "client2".to_string());

        let subs = pubsub.get_subscribers("news");
        assert_eq!(subs.len(), 2);

        pubsub.unsubscribe("news", "client1");
        let subs = pubsub.get_subscribers("news");
        assert_eq!(subs.len(), 1);
    }

    #[test]
    fn test_cursor_encoding() {
        let cursor = Cursor::forward("pos_123".to_string());
        let encoded = cursor.encode();
        let decoded = Cursor::decode(&encoded).unwrap();
        assert_eq!(decoded.position, "pos_123");
    }
}
