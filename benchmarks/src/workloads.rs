use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WorkloadType {
    HelloWorld,
    JsonProcessing,
    ComputeHeavy,
    IoOperations,
}

impl WorkloadType {
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "hello" | "helloworld" => Ok(Self::HelloWorld),
            "json" | "jsonprocessing" => Ok(Self::JsonProcessing),
            "compute" | "computeheavy" => Ok(Self::ComputeHeavy),
            "io" | "iooperations" => Ok(Self::IoOperations),
            _ => Err(anyhow::anyhow!("Unknown workload type: {}", s)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Workload {
    workload_type: WorkloadType,
}

impl Workload {
    pub fn new(workload_type: WorkloadType) -> Self {
        Self { workload_type }
    }

    pub fn name(&self) -> &str {
        match self.workload_type {
            WorkloadType::HelloWorld => "Hello World",
            WorkloadType::JsonProcessing => "JSON Processing",
            WorkloadType::ComputeHeavy => "Compute Heavy",
            WorkloadType::IoOperations => "I/O Operations",
        }
    }

    pub fn function_name(&self) -> String {
        match self.workload_type {
            WorkloadType::HelloWorld => "bench-hello-world".to_string(),
            WorkloadType::JsonProcessing => "bench-json-processing".to_string(),
            WorkloadType::ComputeHeavy => "bench-compute-heavy".to_string(),
            WorkloadType::IoOperations => "bench-io-operations".to_string(),
        }
    }

    pub fn code(&self) -> &str {
        match self.workload_type {
            WorkloadType::HelloWorld => HELLO_WORLD_CODE,
            WorkloadType::JsonProcessing => JSON_PROCESSING_CODE,
            WorkloadType::ComputeHeavy => COMPUTE_HEAVY_CODE,
            WorkloadType::IoOperations => IO_OPERATIONS_CODE,
        }
    }

    pub fn payload(&self) -> serde_json::Value {
        match self.workload_type {
            WorkloadType::HelloWorld => serde_json::json!({
                "name": "Benchmark"
            }),
            WorkloadType::JsonProcessing => serde_json::json!({
                "records": [
                    {"id": 1, "name": "Alice", "age": 30, "score": 95.5},
                    {"id": 2, "name": "Bob", "age": 25, "score": 87.3},
                    {"id": 3, "name": "Charlie", "age": 35, "score": 92.1},
                    {"id": 4, "name": "David", "age": 28, "score": 88.7},
                    {"id": 5, "name": "Eve", "age": 32, "score": 91.2}
                ]
            }),
            WorkloadType::ComputeHeavy => serde_json::json!({
                "iterations": 100000
            }),
            WorkloadType::IoOperations => serde_json::json!({
                "file_count": 10,
                "file_size": 1024
            }),
        }
    }
}

const HELLO_WORLD_CODE: &str = r#"
import json

def handler(event):
    name = event.get('name', 'World')
    return {
        'statusCode': 200,
        'body': json.dumps({'message': f'Hello, {name}!'})
    }
"#;

const JSON_PROCESSING_CODE: &str = r#"
import json
from typing import List, Dict

def handler(event):
    records = event.get('records', [])
    
    # Process records: filter, transform, aggregate
    filtered = [r for r in records if r['age'] > 25]
    transformed = [
        {
            'id': r['id'],
            'name': r['name'].upper(),
            'age_group': 'senior' if r['age'] > 30 else 'junior',
            'grade': 'A' if r['score'] >= 90 else 'B' if r['score'] >= 80 else 'C'
        }
        for r in filtered
    ]
    
    stats = {
        'total': len(records),
        'filtered': len(filtered),
        'avg_age': sum(r['age'] for r in filtered) / len(filtered) if filtered else 0,
        'avg_score': sum(r['score'] for r in filtered) / len(filtered) if filtered else 0,
    }
    
    return {
        'statusCode': 200,
        'body': json.dumps({
            'records': transformed,
            'statistics': stats
        })
    }
"#;

const COMPUTE_HEAVY_CODE: &str = r#"
import json
import math

def handler(event):
    iterations = event.get('iterations', 10000)
    
    # CPU-intensive calculations
    result = 0.0
    for i in range(iterations):
        # Trigonometric calculations
        result += math.sin(i) * math.cos(i)
        result += math.sqrt(abs(result) + 1)
        result = result % 1000000
        
        # Prime check for some numbers
        if i % 100 == 0:
            is_prime = all(i % j != 0 for j in range(2, int(math.sqrt(i)) + 1)) if i > 1 else False
            result += 1 if is_prime else 0
    
    return {
        'statusCode': 200,
        'body': json.dumps({
            'iterations': iterations,
            'result': result,
            'computation': 'heavy'
        })
    }
"#;

const IO_OPERATIONS_CODE: &str = r#"
import json
import tempfile
import os

def handler(event):
    file_count = event.get('file_count', 10)
    file_size = event.get('file_size', 1024)
    
    # Create temporary directory
    with tempfile.TemporaryDirectory() as tmpdir:
        files_created = []
        total_bytes = 0
        
        # Write files
        for i in range(file_count):
            filepath = os.path.join(tmpdir, f'file_{i}.txt')
            content = f'Data block {i}\n' * (file_size // 20)
            
            with open(filepath, 'w') as f:
                f.write(content)
            
            files_created.append(filepath)
            total_bytes += len(content)
        
        # Read and process files
        processed_data = []
        for filepath in files_created:
            with open(filepath, 'r') as f:
                lines = f.readlines()
                processed_data.append({
                    'file': os.path.basename(filepath),
                    'lines': len(lines),
                    'size': os.path.getsize(filepath)
                })
        
        return {
            'statusCode': 200,
            'body': json.dumps({
                'files_created': file_count,
                'total_bytes_written': total_bytes,
                'files_processed': processed_data
            })
        }
"#;
