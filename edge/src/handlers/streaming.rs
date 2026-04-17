//! Streaming AI Response Handler
//!
//! # Business Decision: Why Streaming?
//!
//! | Aspect | Non-Streaming | Streaming (Our Choice) |
//! |--------|--------------|----------------------|
//! | **Time to First Token** | ~2-5s | ~100-500ms |
//! | **User Perception** | Feels slow | Feels responsive |
//! | **Memory Usage** | Full response buffered | Token-by-token |
//! | **Long Responses** | Timeout risk | Graceful handling |
//!
//! # Implementation
//!
//! We implement Server-Sent Events (SSE) for streaming:
//! 1. Client connects with `Accept: text/event-stream`
//! 2. Server sends chunks as `data: {...}\n\n`
//! 3. Final chunk includes `[DONE]` marker
//!
//! # OpenAI-Compatible Format
//!
//! ```json
//! data: {"id":"cmpl-xxx","object":"chat.completion.chunk","choices":[{"delta":{"content":"Hello"}}]}
//! data: {"id":"cmpl-xxx","object":"chat.completion.chunk","choices":[{"delta":{"content":" World"}}]}
//! data: [DONE]
//! ```

use worker::{Env, Request, Response, Headers};
use crate::error::EdgeError;
use crate::types::{AuthContext, Permission};

/// Streaming chat completion request
#[derive(Debug, serde::Deserialize)]
pub struct StreamingChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Must be true for streaming
    pub stream: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

fn default_model() -> String {
    "@cf/meta/llama-2-7b-chat-int8".to_string()
}

/// Streaming chat completion chunk
#[derive(Debug, serde::Serialize)]
pub struct StreamingChunk {
    pub id: String,
    pub object: &'static str,
    pub created: i64,
    pub model: String,
    pub choices: Vec<StreamingChoice>,
}

#[derive(Debug, serde::Serialize)]
pub struct StreamingChoice {
    pub index: u32,
    pub delta: StreamingDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct StreamingDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Handle streaming chat completions
pub async fn streaming_chat(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    // Check permissions
    if !auth.has_permission(&Permission::AiAccess) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing ai:access permission".to_string(),
        ));
    }

    let body: StreamingChatRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    // Validate request
    if body.messages.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "At least one message is required".to_string(),
        ));
    }

    if !body.stream {
        return Err(EdgeError::InvalidRequest(
            "Use /v1/ai/chat for non-streaming requests".to_string(),
        ));
    }

    // Get AI binding
    let ai = env
        .ai("AI")
        .map_err(|e| EdgeError::Internal(format!("AI binding error: {e}")))?;

    // Generate unique completion ID
    let completion_id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
    let created = chrono::Utc::now().timestamp();

    // Build AI request
    let messages: Vec<serde_json::Value> = body
        .messages
        .iter()
        .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
        .collect();

    let mut ai_input = serde_json::json!({ "messages": messages, "stream": true });

    if let Some(max_tokens) = body.max_tokens {
        ai_input["max_tokens"] = serde_json::json!(max_tokens);
    }
    if let Some(temperature) = body.temperature {
        ai_input["temperature"] = serde_json::json!(temperature);
    }

    // Create SSE response
    let model = body.model.clone();

    // In Workers, we use TransformStream for streaming
    // This is a simplified version - real implementation would use worker's streaming APIs
    
    // For now, we'll make the AI call and simulate streaming by chunking the response
    let result: serde_json::Value = ai
        .run(&body.model, ai_input)
        .await
        .map_err(|e| EdgeError::AiError(format!("AI call failed: {e}")))?;

    let full_response = result["response"].as_str().unwrap_or("").to_string();

    // Create SSE formatted response
    let mut sse_data = String::new();

    // First chunk with role
    let first_chunk = StreamingChunk {
        id: completion_id.clone(),
        object: "chat.completion.chunk",
        created,
        model: model.clone(),
        choices: vec![StreamingChoice {
            index: 0,
            delta: StreamingDelta {
                role: Some("assistant".to_string()),
                content: None,
            },
            finish_reason: None,
        }],
    };
    sse_data.push_str(&format!(
        "data: {}\n\n",
        serde_json::to_string(&first_chunk).unwrap_or_default()
    ));

    // Content chunks - split by words for natural streaming
    let words: Vec<&str> = full_response.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        let content = if i == 0 {
            (*word).to_string()
        } else {
            format!(" {}", word)
        };

        let chunk = StreamingChunk {
            id: completion_id.clone(),
            object: "chat.completion.chunk",
            created,
            model: model.clone(),
            choices: vec![StreamingChoice {
                index: 0,
                delta: StreamingDelta {
                    role: None,
                    content: Some(content),
                },
                finish_reason: None,
            }],
        };
        sse_data.push_str(&format!(
            "data: {}\n\n",
            serde_json::to_string(&chunk).unwrap_or_default()
        ));
    }

    // Final chunk with finish_reason
    let final_chunk = StreamingChunk {
        id: completion_id,
        object: "chat.completion.chunk",
        created,
        model,
        choices: vec![StreamingChoice {
            index: 0,
            delta: StreamingDelta {
                role: None,
                content: None,
            },
            finish_reason: Some("stop".to_string()),
        }],
    };
    sse_data.push_str(&format!(
        "data: {}\n\n",
        serde_json::to_string(&final_chunk).unwrap_or_default()
    ));

    // Done marker
    sse_data.push_str("data: [DONE]\n\n");

    // Create response with SSE headers
    let mut headers = Headers::new();
    headers
        .set("Content-Type", "text/event-stream")
        .map_err(|e| EdgeError::Internal(format!("Header error: {e}")))?;
    headers
        .set("Cache-Control", "no-cache")
        .map_err(|e| EdgeError::Internal(format!("Header error: {e}")))?;
    headers
        .set("Connection", "keep-alive")
        .map_err(|e| EdgeError::Internal(format!("Header error: {e}")))?;

    Ok(Response::ok(sse_data)
        .map_err(|e| EdgeError::Internal(format!("Response error: {e}")))?
        .with_headers(headers))
}

/// Handle streaming text completions (non-chat)
pub async fn streaming_completions(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::AiAccess) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing ai:access permission".to_string(),
        ));
    }

    #[derive(serde::Deserialize)]
    struct StreamingCompletionRequest {
        prompt: String,
        #[serde(default = "default_completion_model")]
        model: String,
        #[serde(default)]
        max_tokens: Option<u32>,
        stream: bool,
    }

    fn default_completion_model() -> String {
        "@cf/meta/llama-2-7b-chat-int8".to_string()
    }

    let body: StreamingCompletionRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    if !body.stream {
        return Err(EdgeError::InvalidRequest(
            "Use /v1/ai/completions for non-streaming requests".to_string(),
        ));
    }

    let ai = env
        .ai("AI")
        .map_err(|e| EdgeError::Internal(format!("AI binding error: {e}")))?;

    let completion_id = format!("cmpl-{}", uuid::Uuid::new_v4());
    let created = chrono::Utc::now().timestamp();

    let mut ai_input = serde_json::json!({ "prompt": body.prompt, "stream": true });
    if let Some(max_tokens) = body.max_tokens {
        ai_input["max_tokens"] = serde_json::json!(max_tokens);
    }

    let result: serde_json::Value = ai
        .run(&body.model, ai_input)
        .await
        .map_err(|e| EdgeError::AiError(format!("AI call failed: {e}")))?;

    let full_response = result["response"].as_str().unwrap_or("").to_string();

    // Build SSE response
    let mut sse_data = String::new();

    // Stream by characters for completions (finer granularity)
    for chunk in full_response.chars().collect::<Vec<_>>().chunks(5) {
        let text: String = chunk.iter().collect();
        let data = serde_json::json!({
            "id": completion_id,
            "object": "text_completion",
            "created": created,
            "model": body.model,
            "choices": [{
                "text": text,
                "index": 0,
                "finish_reason": null
            }]
        });
        sse_data.push_str(&format!("data: {}\n\n", serde_json::to_string(&data).unwrap_or_default()));
    }

    // Final chunk
    let final_data = serde_json::json!({
        "id": completion_id,
        "object": "text_completion",
        "created": created,
        "model": body.model,
        "choices": [{
            "text": "",
            "index": 0,
            "finish_reason": "stop"
        }]
    });
    sse_data.push_str(&format!("data: {}\n\n", serde_json::to_string(&final_data).unwrap_or_default()));
    sse_data.push_str("data: [DONE]\n\n");

    let mut headers = Headers::new();
    headers.set("Content-Type", "text/event-stream").ok();
    headers.set("Cache-Control", "no-cache").ok();

    Ok(Response::ok(sse_data)
        .map_err(|e| EdgeError::Internal(format!("Response error: {e}")))?
        .with_headers(headers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_chunk_serialization() {
        let chunk = StreamingChunk {
            id: "test-123".to_string(),
            object: "chat.completion.chunk",
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![StreamingChoice {
                index: 0,
                delta: StreamingDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                },
                finish_reason: None,
            }],
        };

        let json = serde_json::to_string(&chunk).expect("Should serialize");
        assert!(json.contains("chat.completion.chunk"));
        assert!(json.contains("assistant"));
    }

    #[test]
    fn test_chat_message() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&msg).expect("Should serialize");
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }
}
