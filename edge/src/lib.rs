//! NanoLambda Edge - Rust-based Edge Serverless Platform
//!
//! A high-performance edge computing platform built with Rust and WebAssembly
//! for Cloudflare Workers. Features include:
//!
//! - Function invocation with JavaScript/Python support
//! - Built-in vector database (QuartzDB) for AI/ML workloads
//! - Workers AI integration for embeddings and inference
//! - API key and JWT authentication
//! - Rate limiting and usage tracking
//! - **Semantic Caching** - Intelligent AI response caching based on query similarity
//! - **Hybrid Search** - Combined BM25 (keyword) and vector search
//! - **Reranking** - Multi-signal result reranking for superior relevance
//! - **Smart Routing** - Geo-aware request routing with A/B testing
//! - **Function Composition** - Chain functions for complex workflows
//! - **Durable State** - Persistent key-value state with versioning
//! - **Predictive Pre-warming** - ML-based function pre-warming
//! - **Analytics Engine** - Real-time analytics and metrics collection

use worker::*;

mod auth;
pub mod cache;
mod error;
mod handlers;
mod router;
pub mod runtime;
pub mod search;
mod types;
mod vector;

// Advanced edge features
pub mod analytics;
pub mod composition;
pub mod prewarm;
pub mod routing;
pub mod state;

pub use error::EdgeError;
pub use types::*;

/// Main entry point for Cloudflare Workers
#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    // Initialize panic hook for better error messages in console
    console_error_panic_hook::set_once();

    // Route the request
    router::handle_request(req, env, ctx).await
}

/// Durable Object for vector index storage
#[durable_object]
pub struct VectorIndex {
    state: State,
    env: Env,
}

impl DurableObject for VectorIndex {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        vector::durable_object::handle_request(self, req).await
    }
}

impl VectorIndex {
    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn env(&self) -> &Env {
        &self.env
    }
}

/// Durable Object for user session management
#[durable_object]
pub struct UserSession {
    state: State,
    env: Env,
}

impl DurableObject for UserSession {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        handlers::session::handle_request(self, req).await
    }
}

impl UserSession {
    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn env(&self) -> &Env {
        &self.env
    }
}

/// Durable Object for rate limiting
#[durable_object]
pub struct RateLimiter {
    state: State,
    env: Env,
}

impl DurableObject for RateLimiter {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        handlers::rate_limit::handle_request(self, req).await
    }
}

impl RateLimiter {
    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn env(&self) -> &Env {
        &self.env
    }
}
