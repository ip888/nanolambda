//! AI handlers for embeddings and text generation

use worker::{Env, Request, Response};

use crate::error::EdgeError;
use crate::types::{
    AuthContext, EmbeddingInput, EmbeddingRequest, EmbeddingResponse, EmbeddingUsage, Permission,
    TextGenerationRequest, TextGenerationResponse, TextGenerationUsage,
};

/// Generate embeddings using Workers AI
pub async fn embeddings(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::AiAccess) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing ai:access permission".to_string(),
        ));
    }

    let body: EmbeddingRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    // Get the AI binding
    let ai = env
        .ai("AI")
        .map_err(|e| EdgeError::Internal(format!("AI binding error: {e}")))?;

    // Prepare input texts
    let texts: Vec<String> = match body.text {
        EmbeddingInput::Single(s) => vec![s],
        EmbeddingInput::Multiple(v) => v,
    };

    // Validate inputs
    if texts.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "At least one text input is required".to_string(),
        ));
    }

    for (i, text) in texts.iter().enumerate() {
        if text.is_empty() {
            return Err(EdgeError::InvalidRequest(format!(
                "Text input {} cannot be empty",
                i
            )));
        }
        // Most embedding models have a token limit around 512 tokens
        // Rough estimate: 1 token ≈ 4 characters
        if text.len() > 8000 {
            return Err(EdgeError::InvalidRequest(format!(
                "Text input {} exceeds maximum length (8000 characters)",
                i
            )));
        }
    }

    // Call Workers AI for embeddings
    // The model should be one of the supported embedding models
    let model = &body.model;

    // Build AI request
    let ai_input = serde_json::json!({
        "text": texts
    });

    let result: serde_json::Value = ai
        .run(model, ai_input)
        .await
        .map_err(|e| EdgeError::AiError(format!("Embedding generation failed: {e}")))?;

    // Parse the result
    let embeddings: Vec<Vec<f32>> = result["data"]
        .as_array()
        .map(|arr: &Vec<serde_json::Value>| {
            arr.iter()
                .filter_map(|v: &serde_json::Value| {
                    v.as_array().map(|inner: &Vec<serde_json::Value>| {
                        inner
                            .iter()
                            .filter_map(|n: &serde_json::Value| n.as_f64().map(|f| f as f32))
                            .collect()
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Calculate token usage (rough estimate)
    let total_chars: usize = texts.iter().map(|t| t.len()).sum();
    let estimated_tokens = (total_chars / 4) as u32;

    let response = EmbeddingResponse {
        model: model.clone(),
        embeddings,
        usage: EmbeddingUsage {
            prompt_tokens: estimated_tokens,
            total_tokens: estimated_tokens,
        },
    };

    Response::from_json(&response).map_err(EdgeError::from)
}

/// Generate text completions using Workers AI
pub async fn completions(
    mut req: Request,
    env: &Env,
    auth: &AuthContext,
) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::AiAccess) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing ai:access permission".to_string(),
        ));
    }

    let body: TextGenerationRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    // Validate prompt
    if body.prompt.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "Prompt cannot be empty".to_string(),
        ));
    }

    // Get the AI binding
    let ai = env
        .ai("AI")
        .map_err(|e| EdgeError::Internal(format!("AI binding error: {e}")))?;

    // Build AI request
    let mut ai_input = serde_json::json!({
        "prompt": body.prompt
    });

    if let Some(max_tokens) = body.max_tokens {
        ai_input["max_tokens"] = serde_json::json!(max_tokens);
    }

    if let Some(temperature) = body.temperature {
        ai_input["temperature"] = serde_json::json!(temperature);
    }

    // Note: Streaming would require different handling
    if body.stream {
        return Err(EdgeError::NotImplemented(
            "Streaming responses not yet implemented".to_string(),
        ));
    }

    let result: serde_json::Value = ai
        .run(&body.model, ai_input)
        .await
        .map_err(|e| EdgeError::AiError(format!("Text generation failed: {e}")))?;

    // Parse the result
    let generated_text = result["response"].as_str().unwrap_or("").to_string();

    // Estimate token usage
    let prompt_tokens = (body.prompt.len() / 4) as u32;
    let completion_tokens = (generated_text.len() / 4) as u32;

    let response = TextGenerationResponse {
        model: body.model,
        response: generated_text,
        usage: Some(TextGenerationUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }),
    };

    Response::from_json(&response).map_err(EdgeError::from)
}

/// Chat completions (OpenAI-compatible format)
pub async fn chat(mut req: Request, env: &Env, auth: &AuthContext) -> Result<Response, EdgeError> {
    if !auth.has_permission(&Permission::AiAccess) {
        return Err(EdgeError::AuthorizationDenied(
            "Missing ai:access permission".to_string(),
        ));
    }

    #[derive(serde::Deserialize)]
    struct ChatRequest {
        messages: Vec<ChatMessage>,
        #[serde(default = "default_chat_model")]
        model: String,
        #[serde(default)]
        max_tokens: Option<u32>,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        stream: bool,
    }

    #[derive(serde::Deserialize)]
    struct ChatMessage {
        role: String,
        content: String,
    }

    fn default_chat_model() -> String {
        "@cf/meta/llama-2-7b-chat-int8".to_string()
    }

    let body: ChatRequest = req
        .json()
        .await
        .map_err(|e| EdgeError::InvalidRequest(format!("Invalid request body: {e}")))?;

    // Validate messages
    if body.messages.is_empty() {
        return Err(EdgeError::InvalidRequest(
            "At least one message is required".to_string(),
        ));
    }

    if body.stream {
        return Err(EdgeError::NotImplemented(
            "Streaming responses not yet implemented".to_string(),
        ));
    }

    // Get the AI binding
    let ai = env
        .ai("AI")
        .map_err(|e| EdgeError::Internal(format!("AI binding error: {e}")))?;

    // Build messages for the model
    let messages: Vec<serde_json::Value> = body
        .messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content
            })
        })
        .collect();

    let mut ai_input = serde_json::json!({
        "messages": messages
    });

    if let Some(max_tokens) = body.max_tokens {
        ai_input["max_tokens"] = serde_json::json!(max_tokens);
    }

    if let Some(temperature) = body.temperature {
        ai_input["temperature"] = serde_json::json!(temperature);
    }

    let result: serde_json::Value = ai
        .run(&body.model, ai_input)
        .await
        .map_err(|e| EdgeError::AiError(format!("Chat completion failed: {e}")))?;

    // Return in OpenAI-compatible format
    let response_text = result["response"].as_str().unwrap_or("").to_string();

    let response = serde_json::json!({
        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": body.model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": response_text
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": body.messages.iter().map(|m| m.content.len() / 4).sum::<usize>(),
            "completion_tokens": response_text.len() / 4,
            "total_tokens": body.messages.iter().map(|m| m.content.len() / 4).sum::<usize>() + response_text.len() / 4
        }
    });

    Response::from_json(&response).map_err(EdgeError::from)
}
