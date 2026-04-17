//! HNSW (Hierarchical Navigable Small World) Index Implementation
//!
//! A high-performance approximate nearest neighbor search algorithm
//! optimized for edge computing environments.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Distance metric for vector similarity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        DistanceMetric::Cosine
    }
}

/// HNSW index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    /// Maximum number of connections per node at layer 0
    pub m: usize,
    /// Maximum number of connections per node at layers > 0
    pub m_max: usize,
    /// Size of dynamic candidate list during construction
    pub ef_construction: usize,
    /// Size of dynamic candidate list during search
    pub ef_search: usize,
    /// Multiplier for level generation
    pub ml: f64,
    /// Distance metric
    pub metric: DistanceMetric,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            m: 16,
            m_max: 32,
            ef_construction: 200,
            ef_search: 50,
            ml: 1.0 / (16_f64).ln(),
            metric: DistanceMetric::Cosine,
        }
    }
}

/// A node in the HNSW graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswNode {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub level: usize,
    pub neighbors: Vec<Vec<String>>, // neighbors[layer] = list of neighbor IDs
}

/// HNSW Index for approximate nearest neighbor search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswIndex {
    config: HnswConfig,
    nodes: HashMap<String, HnswNode>,
    entry_point: Option<String>,
    max_level: usize,
    dimensions: Option<usize>,
}

impl HnswIndex {
    /// Create a new HNSW index with the given configuration
    pub fn new(config: HnswConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            entry_point: None,
            max_level: 0,
            dimensions: None,
        }
    }

    /// Create a new HNSW index with default configuration
    /// Reserved for future convenience API usage
    #[allow(dead_code)]
    pub fn with_defaults() -> Self {
        Self::new(HnswConfig::default())
    }

    /// Get the number of vectors in the index
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the index is empty
    /// Reserved for future index management features
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get the dimensions of vectors in the index
    pub fn dimensions(&self) -> Option<usize> {
        self.dimensions
    }

    /// Insert a vector into the index
    pub fn insert(
        &mut self,
        id: String,
        vector: Vec<f32>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        // Validate dimensions
        if let Some(dims) = self.dimensions {
            if vector.len() != dims {
                return Err(format!(
                    "Vector dimension mismatch: expected {}, got {}",
                    dims,
                    vector.len()
                ));
            }
        } else {
            self.dimensions = Some(vector.len());
        }

        // Normalize vector if using cosine distance
        let vector = if self.config.metric == DistanceMetric::Cosine {
            normalize(&vector)
        } else {
            vector
        };

        // Generate random level for this node
        let level = self.random_level();

        // Create the new node
        let mut new_node = HnswNode {
            id: id.clone(),
            vector,
            metadata,
            level,
            neighbors: vec![Vec::new(); level + 1],
        };

        // If this is the first node, make it the entry point
        if self.entry_point.is_none() {
            self.entry_point = Some(id.clone());
            self.max_level = level;
            self.nodes.insert(id, new_node);
            return Ok(());
        }

        // Find entry point for insertion
        // SAFETY: entry_point is guaranteed to be Some here due to the check above
        let mut current_id = self.entry_point.clone().expect("Entry point checked above");
        let mut current_level = self.max_level;

        // Traverse from top to the level of the new node
        while current_level > level {
            let nearest = self.search_layer(&new_node.vector, &current_id, 1, current_level);
            if let Some((nearest_id, _)) = nearest.first() {
                current_id = nearest_id.clone();
            }
            current_level -= 1;
        }

        // Insert at each level from level down to 0
        for lc in (0..=level.min(self.max_level)).rev() {
            let candidates = self.search_layer(
                &new_node.vector,
                &current_id,
                self.config.ef_construction,
                lc,
            );

            // Select M nearest neighbors
            let m = if lc == 0 {
                self.config.m * 2
            } else {
                self.config.m
            };
            let neighbors: Vec<String> = candidates
                .iter()
                .take(m)
                .map(|(id, _)| id.clone())
                .collect();

            // Connect new node to neighbors
            new_node.neighbors[lc] = neighbors.clone();

            // Connect neighbors back to new node (bidirectional)
            for neighbor_id in &neighbors {
                if let Some(neighbor) = self.nodes.get_mut(neighbor_id) {
                    if lc < neighbor.neighbors.len() {
                        neighbor.neighbors[lc].push(id.clone());
                        // Prune if necessary
                        if neighbor.neighbors[lc].len() > self.config.m_max {
                            self.prune_neighbors(neighbor_id.clone(), lc);
                        }
                    }
                }
            }

            if let Some((nearest_id, _)) = candidates.first() {
                current_id = nearest_id.clone();
            }
        }

        // Update entry point if new node has higher level
        if level > self.max_level {
            self.max_level = level;
            self.entry_point = Some(id.clone());
        }

        self.nodes.insert(id, new_node);
        Ok(())
    }

    /// Search for k nearest neighbors
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        if self.entry_point.is_none() {
            return Vec::new();
        }

        // Normalize query if using cosine distance
        let query = if self.config.metric == DistanceMetric::Cosine {
            normalize(query)
        } else {
            query.to_vec()
        };

        // SAFETY: entry_point is guaranteed to be Some here due to the check above
        let mut current_id = self.entry_point.clone().expect("Entry point checked above");
        let mut current_level = self.max_level;

        // Traverse from top to level 1
        while current_level > 0 {
            let nearest = self.search_layer(&query, &current_id, 1, current_level);
            if let Some((nearest_id, _)) = nearest.first() {
                current_id = nearest_id.clone();
            }
            current_level -= 1;
        }

        // Search at level 0 with ef_search candidates
        let candidates = self.search_layer(&query, &current_id, self.config.ef_search, 0);

        // Return top k
        candidates.into_iter().take(k).collect()
    }

    /// Delete a vector from the index
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let node = self.nodes.remove(id).ok_or("Vector not found")?;

        // Remove references from neighbors
        for (level, neighbors) in node.neighbors.iter().enumerate() {
            for neighbor_id in neighbors {
                if let Some(neighbor) = self.nodes.get_mut(neighbor_id) {
                    if level < neighbor.neighbors.len() {
                        neighbor.neighbors[level].retain(|n| n != id);
                    }
                }
            }
        }

        // Update entry point if we deleted it
        if self.entry_point.as_ref() == Some(&id.to_string()) {
            // Find new entry point at max level
            self.entry_point = self
                .nodes
                .iter()
                .filter(|(_, n)| n.level == self.max_level)
                .map(|(id, _)| id.clone())
                .next();

            // If no node at max level, reduce max level
            if self.entry_point.is_none() && !self.nodes.is_empty() {
                self.max_level = self.nodes.values().map(|n| n.level).max().unwrap_or(0);
                self.entry_point = self
                    .nodes
                    .iter()
                    .filter(|(_, n)| n.level == self.max_level)
                    .map(|(id, _)| id.clone())
                    .next();
            }
        }

        Ok(())
    }

    /// Get a vector by ID
    pub fn get(&self, id: &str) -> Option<&HnswNode> {
        self.nodes.get(id)
    }

    /// Search within a single layer
    fn search_layer(
        &self,
        query: &[f32],
        entry_id: &str,
        ef: usize,
        level: usize,
    ) -> Vec<(String, f32)> {
        let entry_node = match self.nodes.get(entry_id) {
            Some(n) => n,
            None => return Vec::new(),
        };

        let entry_dist = self.distance(query, &entry_node.vector);

        // Candidates: min-heap of (distance, id) - closest first
        let mut candidates: BinaryHeap<Candidate> = BinaryHeap::new();
        // Results: max-heap of (distance, id) - furthest first for pruning
        let mut results: BinaryHeap<Candidate> = BinaryHeap::new();
        let mut visited: HashSet<String> = HashSet::new();

        candidates.push(Candidate {
            distance: entry_dist,
            id: entry_id.to_string(),
            is_min_heap: true,
        });
        results.push(Candidate {
            distance: entry_dist,
            id: entry_id.to_string(),
            is_min_heap: false,
        });
        visited.insert(entry_id.to_string());

        while let Some(current) = candidates.pop() {
            // Check if we should stop
            if let Some(furthest) = results.peek() {
                if current.distance > furthest.distance && results.len() >= ef {
                    break;
                }
            }

            // Explore neighbors
            if let Some(node) = self.nodes.get(&current.id) {
                if level < node.neighbors.len() {
                    for neighbor_id in &node.neighbors[level] {
                        if !visited.contains(neighbor_id) {
                            visited.insert(neighbor_id.clone());

                            if let Some(neighbor) = self.nodes.get(neighbor_id) {
                                let dist = self.distance(query, &neighbor.vector);

                                let should_add = results.len() < ef
                                    || results.peek().map_or(true, |f| dist < f.distance);

                                if should_add {
                                    candidates.push(Candidate {
                                        distance: dist,
                                        id: neighbor_id.clone(),
                                        is_min_heap: true,
                                    });
                                    results.push(Candidate {
                                        distance: dist,
                                        id: neighbor_id.clone(),
                                        is_min_heap: false,
                                    });

                                    // Prune results if too large
                                    while results.len() > ef {
                                        results.pop();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Convert results to sorted vec (closest first)
        let mut result_vec: Vec<(String, f32)> =
            results.into_iter().map(|c| (c.id, c.distance)).collect();
        result_vec.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        result_vec
    }

    /// Prune neighbors of a node to maintain connectivity
    fn prune_neighbors(&mut self, node_id: String, level: usize) {
        let node = match self.nodes.get(&node_id) {
            Some(n) => n.clone(),
            None => return,
        };

        if level >= node.neighbors.len() {
            return;
        }

        // Get distances to all neighbors
        let mut neighbor_distances: Vec<(String, f32)> = node.neighbors[level]
            .iter()
            .filter_map(|n_id| {
                self.nodes
                    .get(n_id)
                    .map(|n| (n_id.clone(), self.distance(&node.vector, &n.vector)))
            })
            .collect();

        // Sort by distance and keep only M closest
        neighbor_distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        neighbor_distances.truncate(self.config.m);

        // Update the node's neighbors
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.neighbors[level] = neighbor_distances.into_iter().map(|(id, _)| id).collect();
        }
    }

    /// Calculate distance between two vectors
    fn distance(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.config.metric {
            DistanceMetric::Cosine => {
                // For normalized vectors, cosine distance = 1 - dot product
                let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
                1.0 - dot
            }
            DistanceMetric::Euclidean => {
                let sum: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();
                sum.sqrt()
            }
            DistanceMetric::DotProduct => {
                // Negative dot product (higher dot product = more similar = lower distance)
                let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
                -dot
            }
        }
    }

    /// Generate a random level for a new node
    fn random_level(&self) -> usize {
        let mut rng = rand::rng();
        let mut level = 0;
        // ml is typically 1/ln(M), so exp(-ml) gives probability for level increase
        let prob = (-1.0 / self.config.ml).exp();
        while rng.random::<f64>() < prob && level < 16 {
            level += 1;
        }
        level
    }

    /// Serialize the index for storage
    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| e.to_string())
    }

    /// Deserialize an index from storage
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(data).map_err(|e| e.to_string())
    }
}

/// Normalize a vector to unit length
fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter().map(|x| x / norm).collect()
    } else {
        v.to_vec()
    }
}

/// Candidate for heap operations
#[derive(Clone)]
struct Candidate {
    distance: f32,
    id: String,
    is_min_heap: bool,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance && self.id == other.id
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        let cmp = self
            .distance
            .partial_cmp(&other.distance)
            .unwrap_or(Ordering::Equal);
        if self.is_min_heap {
            cmp.reverse() // Min heap: smaller distance = higher priority
        } else {
            cmp // Max heap: larger distance = higher priority
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_search() {
        let mut index = HnswIndex::with_defaults();

        // Insert some vectors
        index
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], HashMap::new())
            .unwrap();
        index
            .insert("v2".to_string(), vec![0.9, 0.1, 0.0], HashMap::new())
            .unwrap();
        index
            .insert("v3".to_string(), vec![0.0, 1.0, 0.0], HashMap::new())
            .unwrap();

        assert_eq!(index.len(), 3);

        // Search for nearest to [1, 0, 0]
        let results = index.search(&[1.0, 0.0, 0.0], 2);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "v1");
    }

    #[test]
    fn test_delete() {
        let mut index = HnswIndex::with_defaults();

        index
            .insert("v1".to_string(), vec![1.0, 0.0], HashMap::new())
            .unwrap();
        index
            .insert("v2".to_string(), vec![0.0, 1.0], HashMap::new())
            .unwrap();

        assert_eq!(index.len(), 2);

        index.delete("v1").unwrap();
        assert_eq!(index.len(), 1);
        assert!(index.get("v1").is_none());
        assert!(index.get("v2").is_some());
    }

    #[test]
    fn test_dimension_validation() {
        let mut index = HnswIndex::with_defaults();

        index
            .insert("v1".to_string(), vec![1.0, 0.0, 0.0], HashMap::new())
            .unwrap();

        // Try to insert vector with wrong dimensions
        let result = index.insert("v2".to_string(), vec![1.0, 0.0], HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization() {
        let mut index = HnswIndex::with_defaults();
        index
            .insert("v1".to_string(), vec![1.0, 0.0], HashMap::new())
            .unwrap();

        let data = index.serialize().unwrap();
        let restored = HnswIndex::deserialize(&data).unwrap();

        assert_eq!(restored.len(), 1);
        assert!(restored.get("v1").is_some());
    }
}
