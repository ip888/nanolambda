//! Function registry

use serde::{Deserialize, Serialize};
use sled::Db;

pub struct FunctionRegistry {
    db: Db,
}

impl FunctionRegistry {
    pub fn new(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }
    
    // TODO: Implement registry methods
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionMetadata {
    pub name: String,
    pub runtime: String,
    pub handler: String,
    pub memory_mb: u32,
    pub timeout_sec: u32,
    pub code_sha256: String,
}
