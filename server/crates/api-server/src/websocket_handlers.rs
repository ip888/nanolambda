//! WebSocket support for real-time function invocation
//!
//! Provides WebSocket connections for streaming function output and real-time updates.

use axum::{
    Json,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{error, info, warn};

use crate::ApiServer;

// ============================================================================
// Types
// ============================================================================

/// WebSocket connection parameters
#[derive(Debug, Deserialize)]
pub struct WsConnectParams {
    pub token: String,
    #[serde(default)]
    pub room: Option<String>,
}

/// WebSocket message from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WsClientMessage {
    #[serde(rename = "invoke")]
    Invoke {
        function_name: String,
        payload: Option<serde_json::Value>,
        #[serde(default)]
        request_id: Option<String>,
    },
    #[serde(rename = "subscribe")]
    Subscribe { room: String },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { room: String },
    #[serde(rename = "ping")]
    Ping,
}

/// WebSocket message to client
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WsServerMessage {
    #[serde(rename = "result")]
    Result {
        request_id: String,
        status_code: u16,
        body: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        execution_time_ms: u64,
    },
    #[serde(rename = "error")]
    Error { message: String, code: String },
    #[serde(rename = "subscribed")]
    Subscribed { room: String },
    #[serde(rename = "unsubscribed")]
    Unsubscribed { room: String },
    #[serde(rename = "broadcast")]
    Broadcast {
        room: String,
        sender: String,
        data: serde_json::Value,
    },
    #[serde(rename = "pong")]
    Pong { timestamp: i64 },
    #[serde(rename = "connected")]
    Connected { user_id: String, session_id: String },
}

/// Room manager for broadcast channels
pub struct RoomManager {
    rooms: RwLock<HashMap<String, broadcast::Sender<WsServerMessage>>>,
    max_rooms: usize,
    #[allow(dead_code)]
    max_room_size: usize,
}

impl RoomManager {
    pub fn new(max_rooms: usize, max_room_size: usize) -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
            max_rooms,
            max_room_size,
        }
    }

    /// Get or create a room's broadcast channel
    pub async fn get_or_create_room(
        &self,
        room: &str,
    ) -> Option<broadcast::Sender<WsServerMessage>> {
        let mut rooms = self.rooms.write().await;

        if let Some(sender) = rooms.get(room) {
            return Some(sender.clone());
        }

        // Check room limit
        if rooms.len() >= self.max_rooms {
            return None;
        }

        // Create new room with buffer
        let (tx, _) = broadcast::channel(100);
        rooms.insert(room.to_string(), tx.clone());
        Some(tx)
    }

    /// Subscribe to a room
    pub async fn subscribe(&self, room: &str) -> Option<broadcast::Receiver<WsServerMessage>> {
        let rooms = self.rooms.read().await;
        rooms.get(room).map(|tx| tx.subscribe())
    }

    /// Broadcast message to a room
    pub async fn broadcast(&self, room: &str, message: WsServerMessage) -> bool {
        let rooms = self.rooms.read().await;
        if let Some(tx) = rooms.get(room) {
            tx.send(message).is_ok()
        } else {
            false
        }
    }

    /// Get room subscriber count
    pub async fn get_room_size(&self, room: &str) -> usize {
        let rooms = self.rooms.read().await;
        rooms.get(room).map(|tx| tx.receiver_count()).unwrap_or(0)
    }

    /// Cleanup empty rooms
    pub async fn cleanup_empty_rooms(&self) {
        let mut rooms = self.rooms.write().await;
        rooms.retain(|_, tx| tx.receiver_count() > 0);
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new(1000, 100)
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// WebSocket upgrade handler for function invocation
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ApiServer>>,
    Path(function_name): Path<String>,
    Query(params): Query<WsConnectParams>,
) -> impl IntoResponse {
    // Validate token
    let jwt_manager = match state.get_jwt_manager() {
        Some(m) => m,
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, "WebSocket not available").into_response();
        }
    };

    let claims = match jwt_manager.validate_token(&params.token) {
        Ok(c) => c,
        Err(_) => {
            return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
        }
    };

    // Check if function exists
    match state.storage().get_function(&function_name) {
        Ok(Some(_)) => {} // Function exists, continue
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                format!("Function '{}' not found", function_name),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to check function",
            )
                .into_response();
        }
    }

    info!(
        "WebSocket connection: user={}, function={}",
        claims.sub, function_name
    );

    ws.on_upgrade(move |socket| {
        handle_socket(socket, state, claims.sub, function_name, params.room)
    })
}

/// Handle individual WebSocket connection
async fn handle_socket(
    socket: WebSocket,
    state: Arc<ApiServer>,
    user_id: String,
    function_name: String,
    initial_room: Option<String>,
) {
    let (mut sender, mut receiver) = socket.split();
    let session_id = uuid::Uuid::new_v4().to_string();

    // Send connected message
    let connected_msg = WsServerMessage::Connected {
        user_id: user_id.clone(),
        session_id: session_id.clone(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    // Subscribe to initial room if specified
    let mut room_receivers: HashMap<String, broadcast::Receiver<WsServerMessage>> = HashMap::new();

    if let Some(room) = initial_room {
        // Create room manager if needed (simplified - in production use shared state)
        let room_manager = RoomManager::default();
        if let Some(rx) = room_manager.subscribe(&room).await {
            room_receivers.insert(room.clone(), rx);
            let subscribed_msg = WsServerMessage::Subscribed { room };
            if let Ok(json) = serde_json::to_string(&subscribed_msg) {
                let _ = sender.send(Message::Text(json.into())).await;
            }
        }
    }

    // Create channel for sending messages
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsServerMessage>(100);

    // Spawn task to forward messages to client
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // Serialize and send message - break on error
            if let Ok(json) = serde_json::to_string(&msg)
                && sender.send(Message::Text(json.into())).await.is_err()
            {
                break;
            }
        }
    });

    // Handle incoming messages
    let tx_clone = tx.clone();
    let state_clone = state.clone();
    let function_name_clone = function_name.clone();
    let user_id_clone = user_id.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Text(text)) => match serde_json::from_str::<WsClientMessage>(&text) {
                    Ok(msg) => {
                        handle_client_message(
                            msg,
                            &state_clone,
                            &function_name_clone,
                            &user_id_clone,
                            &tx_clone,
                        )
                        .await;
                    }
                    Err(e) => {
                        let error_msg = WsServerMessage::Error {
                            message: format!("Invalid message: {}", e),
                            code: "INVALID_MESSAGE".to_string(),
                        };
                        let _ = tx_clone.send(error_msg).await;
                    }
                },
                Ok(Message::Close(_)) => {
                    info!("WebSocket closed by client: {}", user_id_clone);
                    break;
                }
                Ok(Message::Ping(data)) => {
                    // Pong is handled automatically by axum
                    let _ = data;
                }
                Err(e) => {
                    warn!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    info!(
        "WebSocket disconnected: user={}, session={}",
        user_id, session_id
    );
}

/// Handle a client message
async fn handle_client_message(
    msg: WsClientMessage,
    state: &Arc<ApiServer>,
    default_function: &str,
    user_id: &str,
    tx: &tokio::sync::mpsc::Sender<WsServerMessage>,
) {
    match msg {
        WsClientMessage::Invoke {
            function_name,
            payload,
            request_id,
        } => {
            let func = if function_name.is_empty() {
                default_function.to_string()
            } else {
                function_name
            };

            let req_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let start = std::time::Instant::now();

            // Check limits before invocation
            if let Some(user_manager) = state.get_user_manager() {
                match user_manager.check_limits(user_id).await {
                    Ok(result) => {
                        if !result.within_limits {
                            let error_msg = WsServerMessage::Error {
                                message: "Plan limits exceeded".to_string(),
                                code: "LIMIT_EXCEEDED".to_string(),
                            };
                            let _ = tx.send(error_msg).await;
                            return;
                        }
                    }
                    Err(e) => {
                        error!("Failed to check limits: {}", e);
                    }
                }
            }

            // Execute function
            match state.invoke_function_by_name(&func, payload).await {
                Ok(result) => {
                    let execution_time_ms = start.elapsed().as_millis() as u64;

                    let response = WsServerMessage::Result {
                        request_id: req_id,
                        status_code: result
                            .get("statusCode")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(200) as u16,
                        body: result
                            .get("body")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        error: None,
                        execution_time_ms,
                    };
                    let _ = tx.send(response).await;

                    // Track usage
                    if let Some(user_manager) = state.get_user_manager() {
                        let _ = user_manager.increment_usage(user_id, 1, 0).await;
                    }
                }
                Err(e) => {
                    let execution_time_ms = start.elapsed().as_millis() as u64;
                    let error_msg = WsServerMessage::Result {
                        request_id: req_id,
                        status_code: 500,
                        body: serde_json::Value::Null,
                        error: Some(e.to_string()),
                        execution_time_ms,
                    };
                    let _ = tx.send(error_msg).await;
                }
            }
        }
        WsClientMessage::Subscribe { room } => {
            // Simplified - in production, use shared room manager
            let subscribed_msg = WsServerMessage::Subscribed { room };
            let _ = tx.send(subscribed_msg).await;
        }
        WsClientMessage::Unsubscribe { room } => {
            let unsubscribed_msg = WsServerMessage::Unsubscribed { room };
            let _ = tx.send(unsubscribed_msg).await;
        }
        WsClientMessage::Ping => {
            let pong_msg = WsServerMessage::Pong {
                timestamp: chrono::Utc::now().timestamp(),
            };
            let _ = tx.send(pong_msg).await;
        }
    }
}

/// REST endpoint to get WebSocket connection info
pub async fn get_ws_info(State(_state): State<Arc<ApiServer>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "endpoint": "/ws/{function_name}",
            "protocol": "wss",
            "authentication": "query parameter: ?token=<jwt_token>",
            "messages": {
                "invoke": {
                    "type": "invoke",
                    "function_name": "string (optional, defaults to URL function)",
                    "payload": "object (optional)",
                    "request_id": "string (optional)"
                },
                "subscribe": {
                    "type": "subscribe",
                    "room": "string"
                },
                "unsubscribe": {
                    "type": "unsubscribe",
                    "room": "string"
                },
                "ping": {
                    "type": "ping"
                }
            }
        })),
    )
}
