//! BM25 Sparse Vector Search Implementation
//!
//! BM25 (Best Matching 25) is a ranking function used in information retrieval.
//! It considers term frequency (TF), inverse document frequency (IDF), and
//! document length normalization.
//!
//! Formula: BM25(D, Q) = Σ IDF(qi) × (f(qi,D) × (k1 + 1)) / (f(qi,D) + k1 × (1 - b + b × |D|/avgdl))
//!
//! Key Features:
//! - Term frequency saturation (diminishing returns for repeated terms)
//! - Document length normalization
//! - Inverse document frequency weighting
//! - Configurable k1 and b parameters

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// BM25 index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25Config {
    /// Term frequency saturation parameter (default: 1.2)
    pub k1: f32,
    /// Length normalization parameter (default: 0.75)
    pub b: f32,
    /// Minimum term length to index
    pub min_term_length: usize,
    /// Maximum terms per document
    pub max_terms_per_doc: usize,
    /// Stop words to exclude
    pub stop_words: HashSet<String>,
}

impl Default for Bm25Config {
    fn default() -> Self {
        let stop_words: HashSet<String> = [
            "a", "an", "and", "are", "as", "at", "be", "by", "for", "from",
            "has", "he", "in", "is", "it", "its", "of", "on", "that", "the",
            "to", "was", "were", "will", "with", "the", "this", "but", "they",
            "have", "had", "what", "when", "where", "who", "which", "why", "how"
        ].iter().map(|s| s.to_string()).collect();
        
        Self {
            k1: 1.2,
            b: 0.75,
            min_term_length: 2,
            max_terms_per_doc: 10000,
            stop_words,
        }
    }
}

/// A tokenized document with term frequencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizedDocument {
    /// Document ID
    pub id: String,
    /// Original text
    pub text: String,
    /// Term frequencies (term -> count)
    pub term_freqs: HashMap<String, u32>,
    /// Total number of terms
    pub term_count: usize,
    /// Optional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TokenizedDocument {
    /// Create a new tokenized document
    pub fn new(id: String, text: String, config: &Bm25Config) -> Self {
        let terms = tokenize(&text, config);
        let mut term_freqs: HashMap<String, u32> = HashMap::new();
        
        for term in &terms {
            *term_freqs.entry(term.clone()).or_insert(0) += 1;
        }
        
        Self {
            id,
            text,
            term_count: terms.len(),
            term_freqs,
            metadata: HashMap::new(),
        }
    }
    
    /// Create with metadata
    pub fn with_metadata(
        id: String,
        text: String,
        metadata: HashMap<String, serde_json::Value>,
        config: &Bm25Config,
    ) -> Self {
        let mut doc = Self::new(id, text, config);
        doc.metadata = metadata;
        doc
    }
}

/// BM25 search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25Result {
    /// Document ID
    pub id: String,
    /// BM25 score
    pub score: f32,
    /// Matching terms
    pub matching_terms: Vec<String>,
}

/// BM25 Index for sparse keyword search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25Index {
    /// Configuration
    config: Bm25Config,
    /// Inverted index: term -> list of (doc_id, term_freq)
    inverted_index: HashMap<String, Vec<(String, u32)>>,
    /// Document lengths (term counts)
    doc_lengths: HashMap<String, usize>,
    /// Total number of documents
    doc_count: usize,
    /// Average document length
    avg_doc_length: f32,
    /// Document frequency per term
    doc_freq: HashMap<String, usize>,
    /// Original documents (for retrieval)
    documents: HashMap<String, TokenizedDocument>,
}

impl Bm25Index {
    /// Create a new BM25 index with default configuration
    pub fn new() -> Self {
        Self::with_config(Bm25Config::default())
    }
    
    /// Create a new BM25 index with custom configuration
    pub fn with_config(config: Bm25Config) -> Self {
        Self {
            config,
            inverted_index: HashMap::new(),
            doc_lengths: HashMap::new(),
            doc_count: 0,
            avg_doc_length: 0.0,
            doc_freq: HashMap::new(),
            documents: HashMap::new(),
        }
    }
    
    /// Get the configuration
    pub fn config(&self) -> &Bm25Config {
        &self.config
    }
    
    /// Add a document to the index
    pub fn add_document(&mut self, doc: TokenizedDocument) {
        let doc_id = doc.id.clone();
        
        // Update inverted index
        for (term, freq) in &doc.term_freqs {
            self.inverted_index
                .entry(term.clone())
                .or_insert_with(Vec::new)
                .push((doc_id.clone(), *freq));
            
            // Track how many documents contain this term
            *self.doc_freq.entry(term.clone()).or_insert(0) += 1;
        }
        
        // Update document lengths
        self.doc_lengths.insert(doc_id.clone(), doc.term_count);
        
        // Store document
        self.documents.insert(doc_id, doc);
        
        // Update stats
        self.doc_count += 1;
        self.update_avg_doc_length();
    }
    
    /// Add multiple documents
    pub fn add_documents(&mut self, docs: Vec<TokenizedDocument>) {
        for doc in docs {
            self.add_document(doc);
        }
    }
    
    /// Remove a document from the index
    pub fn remove_document(&mut self, doc_id: &str) -> Option<TokenizedDocument> {
        if let Some(doc) = self.documents.remove(doc_id) {
            // Update inverted index
            for term in doc.term_freqs.keys() {
                if let Some(postings) = self.inverted_index.get_mut(term) {
                    postings.retain(|(id, _)| id != doc_id);
                    if postings.is_empty() {
                        self.inverted_index.remove(term);
                    }
                }
                if let Some(df) = self.doc_freq.get_mut(term) {
                    *df = df.saturating_sub(1);
                    if *df == 0 {
                        self.doc_freq.remove(term);
                    }
                }
            }
            
            self.doc_lengths.remove(doc_id);
            self.doc_count = self.doc_count.saturating_sub(1);
            self.update_avg_doc_length();
            
            return Some(doc);
        }
        None
    }
    
    /// Search the index with a query
    pub fn search(&self, query: &str, top_k: usize) -> Vec<Bm25Result> {
        let query_terms = tokenize(query, &self.config);
        
        if query_terms.is_empty() || self.doc_count == 0 {
            return Vec::new();
        }
        
        // Score accumulator for each document
        let mut scores: HashMap<String, (f32, Vec<String>)> = HashMap::new();
        
        for term in &query_terms {
            if let Some(postings) = self.inverted_index.get(term) {
                let idf = self.calculate_idf(term);
                
                for (doc_id, term_freq) in postings {
                    let doc_len = *self.doc_lengths.get(doc_id).unwrap_or(&1) as f32;
                    let score = self.bm25_score(*term_freq as f32, doc_len, idf);
                    
                    let entry = scores.entry(doc_id.clone()).or_insert((0.0, Vec::new()));
                    entry.0 += score;
                    if !entry.1.contains(term) {
                        entry.1.push(term.clone());
                    }
                }
            }
        }
        
        // Sort by score and return top_k results
        let mut results: Vec<Bm25Result> = scores
            .into_iter()
            .map(|(id, (score, matching_terms))| Bm25Result {
                id,
                score,
                matching_terms,
            })
            .collect();
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        
        results
    }
    
    /// Calculate BM25 score for a single term
    fn bm25_score(&self, term_freq: f32, doc_len: f32, idf: f32) -> f32 {
        let k1 = self.config.k1;
        let b = self.config.b;
        let avgdl = self.avg_doc_length;
        
        let numerator = term_freq * (k1 + 1.0);
        let denominator = term_freq + k1 * (1.0 - b + b * (doc_len / avgdl));
        
        idf * (numerator / denominator)
    }
    
    /// Calculate IDF for a term
    fn calculate_idf(&self, term: &str) -> f32 {
        let n = self.doc_count as f32;
        let df = *self.doc_freq.get(term).unwrap_or(&0) as f32;
        
        if df == 0.0 {
            return 0.0;
        }
        
        // IDF with smoothing: log((N - df + 0.5) / (df + 0.5) + 1)
        ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
    }
    
    /// Update average document length
    fn update_avg_doc_length(&mut self) {
        if self.doc_count == 0 {
            self.avg_doc_length = 0.0;
            return;
        }
        
        let total_length: usize = self.doc_lengths.values().sum();
        self.avg_doc_length = total_length as f32 / self.doc_count as f32;
    }
    
    /// Get document by ID
    pub fn get_document(&self, id: &str) -> Option<&TokenizedDocument> {
        self.documents.get(id)
    }
    
    /// Get number of documents
    pub fn len(&self) -> usize {
        self.doc_count
    }
    
    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.doc_count == 0
    }
    
    /// Get vocabulary size
    pub fn vocabulary_size(&self) -> usize {
        self.inverted_index.len()
    }
    
    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    
    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for Bm25Index {
    fn default() -> Self {
        Self::new()
    }
}

/// Tokenize text into terms
fn tokenize(text: &str, config: &Bm25Config) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= config.min_term_length)
        .filter(|s| !config.stop_words.contains(*s))
        .take(config.max_terms_per_doc)
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bm25_basic_search() {
        let mut index = Bm25Index::new();
        
        let config = &index.config.clone();
        index.add_document(TokenizedDocument::new(
            "doc1".to_string(),
            "The quick brown fox jumps over the lazy dog".to_string(),
            config,
        ));
        index.add_document(TokenizedDocument::new(
            "doc2".to_string(),
            "A fast brown fox leaps across the sleepy canine".to_string(),
            config,
        ));
        index.add_document(TokenizedDocument::new(
            "doc3".to_string(),
            "The weather today is sunny and warm".to_string(),
            config,
        ));
        
        let results = index.search("brown fox", 10);
        
        assert!(!results.is_empty());
        // Both doc1 and doc2 should match
        assert!(results.len() >= 2);
        // doc1 and doc2 should score higher than doc3 (if it appears at all)
    }
    
    #[test]
    fn test_bm25_relevance_ranking() {
        let mut index = Bm25Index::new();
        let config = &index.config.clone();
        
        // Add documents with varying relevance to "machine learning"
        index.add_document(TokenizedDocument::new(
            "doc1".to_string(),
            "Machine learning is a subset of artificial intelligence".to_string(),
            config,
        ));
        index.add_document(TokenizedDocument::new(
            "doc2".to_string(),
            "Deep learning and machine learning are transforming technology".to_string(),
            config,
        ));
        index.add_document(TokenizedDocument::new(
            "doc3".to_string(),
            "The weather is nice today".to_string(),
            config,
        ));
        
        let results = index.search("machine learning", 10);
        
        // doc2 mentions "machine learning" and more relevant terms, should rank high
        // doc3 should not appear (no matching terms)
        assert!(!results.is_empty());
        for result in &results {
            assert_ne!(result.id, "doc3");
        }
    }
    
    #[test]
    fn test_tokenization() {
        let config = Bm25Config::default();
        let tokens = tokenize("Hello, World! This is a TEST.", &config);
        
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        // "a" should be filtered out as stop word
        assert!(!tokens.contains(&"a".to_string()));
    }
    
    #[test]
    fn test_document_removal() {
        let mut index = Bm25Index::new();
        let config = &index.config.clone();
        
        index.add_document(TokenizedDocument::new(
            "doc1".to_string(),
            "Test document one".to_string(),
            config,
        ));
        index.add_document(TokenizedDocument::new(
            "doc2".to_string(),
            "Test document two".to_string(),
            config,
        ));
        
        assert_eq!(index.len(), 2);
        
        index.remove_document("doc1");
        
        assert_eq!(index.len(), 1);
        assert!(index.get_document("doc1").is_none());
        assert!(index.get_document("doc2").is_some());
    }
}
