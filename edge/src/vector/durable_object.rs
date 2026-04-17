//! VectorIndex Durable Object handler

use crate::VectorIndex;
use crate::types::{
    VectorDeleteRequest, VectorMatch, VectorQueryRequest, VectorQueryResponse, VectorUpsertRequest,
};
use crate::vector::{DistanceMetric, HnswConfig, HnswIndex};
use worker::{Request, Response};

const INDEX_STORAGE_KEY: &str = "hnsw_index";

/// Handle requests to the VectorIndex Durable Object
pub async fn handle_request(index_do: &VectorIndex, mut req: Request) -> worker::Result<Response> {
    let url = req.url()?;
    let path = url.path();

    match path {
        "/upsert" => handle_upsert(index_do, &mut req).await,
        "/query" => handle_query(index_do, &mut req).await,
        "/delete" => handle_delete(index_do, &mut req).await,
        "/stats" => handle_stats(index_do).await,
        _ => Response::error("Not found", 404),
    }
}

/// Load the HNSW index from storage
async fn load_index(index_do: &VectorIndex) -> worker::Result<HnswIndex> {
    let storage = index_do.state().storage();

    let data: Option<Vec<u8>> = storage.get(INDEX_STORAGE_KEY).await?;

    match data {
        Some(bytes) => HnswIndex::deserialize(&bytes).map_err(|e| worker::Error::RustError(e)),
        None => Ok(HnswIndex::new(HnswConfig {
            m: 16,
            m_max: 32,
            ef_construction: 200,
            ef_search: 50,
            ml: 1.0 / 16_f64.ln(),
            metric: DistanceMetric::Cosine,
        })),
    }
}

/// Save the HNSW index to storage
async fn save_index(index_do: &VectorIndex, index: &HnswIndex) -> worker::Result<()> {
    let storage = index_do.state().storage();

    let data = index.serialize().map_err(|e| worker::Error::RustError(e))?;

    storage.put(INDEX_STORAGE_KEY, data).await?;
    Ok(())
}

/// Handle vector upsert requests
async fn handle_upsert(index_do: &VectorIndex, req: &mut Request) -> worker::Result<Response> {
    let body: VectorUpsertRequest = req.json().await?;

    let mut index = load_index(index_do).await?;

    let mut upserted = 0;
    let mut errors = Vec::new();

    for vector_input in body.vectors {
        match index.insert(
            vector_input.id.clone(),
            vector_input.values,
            vector_input.metadata,
        ) {
            Ok(()) => upserted += 1,
            Err(e) => errors.push(format!("{}: {e}", vector_input.id)),
        }
    }

    // Save the updated index
    save_index(index_do, &index).await?;

    Response::from_json(&serde_json::json!({
        "upserted": upserted,
        "errors": errors,
        "total_vectors": index.len()
    }))
}

/// Handle vector query requests
async fn handle_query(index_do: &VectorIndex, req: &mut Request) -> worker::Result<Response> {
    let body: VectorQueryRequest = req.json().await?;

    let index = load_index(index_do).await?;

    // Perform the search
    let results = index.search(&body.vector, body.top_k);

    // Build response matches
    let matches: Vec<VectorMatch> = results
        .into_iter()
        .filter_map(|(id, score)| {
            let node = index.get(&id)?;
            Some(VectorMatch {
                id,
                score: 1.0 - score, // Convert distance to similarity
                values: if body.include_values {
                    Some(node.vector.clone())
                } else {
                    None
                },
                metadata: if body.include_metadata {
                    Some(node.metadata.clone())
                } else {
                    None
                },
            })
        })
        .collect();

    let response = VectorQueryResponse {
        matches,
        namespace: body.namespace,
    };

    Response::from_json(&response)
}

/// Handle vector delete requests
async fn handle_delete(index_do: &VectorIndex, req: &mut Request) -> worker::Result<Response> {
    let body: VectorDeleteRequest = req.json().await?;

    let mut index = load_index(index_do).await?;

    let mut deleted = 0;
    let mut errors = Vec::new();

    if body.delete_all {
        // Reset the index
        index = HnswIndex::new(HnswConfig::default());
        deleted = index.len();
    } else if let Some(ids) = body.ids {
        for id in ids {
            match index.delete(&id) {
                Ok(()) => deleted += 1,
                Err(e) => errors.push(format!("{id}: {e}")),
            }
        }
    }
    // TODO: Handle filter-based deletion

    // Save the updated index
    save_index(index_do, &index).await?;

    Response::from_json(&serde_json::json!({
        "deleted": deleted,
        "errors": errors,
        "remaining_vectors": index.len()
    }))
}

/// Handle stats requests
async fn handle_stats(index_do: &VectorIndex) -> worker::Result<Response> {
    let index = load_index(index_do).await?;

    Response::from_json(&serde_json::json!({
        "total_vectors": index.len(),
        "dimensions": index.dimensions(),
        "index_type": "hnsw"
    }))
}
