//! # Hybrid Search Module
//!
//! Combines sparse (BM25/keyword) and dense (vector) search for
//! superior retrieval accuracy.
//!
//! ## Business Decision
//!
//! Hybrid search is a critical differentiator for RAG (Retrieval-Augmented Generation)
//! applications. Research shows hybrid search improves retrieval quality by 10-20%
//! compared to vector-only search:
//! - BM25 excels at exact keyword matching (product IDs, technical terms)
//! - Vector search excels at semantic similarity (conceptual matches)
//! - Combined results capture both precision and recall
//!
//! ## Technical Design
//!
//! Three fusion methods are supported:
//! 1. **RRF (Reciprocal Rank Fusion)** - Default, robust, doesn't require score normalization
//! 2. **Weighted** - Linear combination with configurable weights
//! 3. **MaxScore** - Takes best score from either method
//!
//! ## Competitor Analysis
//!
//! - Pinecone: Announced hybrid search in 2023, limited availability
//! - Qdrant: Supports hybrid but requires separate BM25 index
//! - Weaviate: Has hybrid but requires self-hosting
//! - **NanoLambda**: Built-in, edge-native, zero configuration required
//!
//! ## Reranking
//!
//! Multi-signal reranking (similar to Cohere Rerank API):
//! - Semantic similarity (primary signal)
//! - Keyword overlap (Jaccard similarity)
//! - Recency scoring (fresher content ranked higher)
//! - Popularity boosting (engagement metrics)
//!
//! Cohere charges $1 per 1000 reranks. Our implementation is FREE.

pub mod bm25;
pub mod hybrid;
pub mod rerank;

pub use bm25::{Bm25Config, Bm25Index, TokenizedDocument};
pub use hybrid::{FusionMethod, HybridConfig, HybridResult, HybridSearch, VectorResult};
pub use rerank::{RankedResult, Reranker, RerankerConfig, RerankerDocument};
