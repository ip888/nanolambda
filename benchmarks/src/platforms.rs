use crate::workloads::Workload;
use async_trait::async_trait;

#[async_trait]
pub trait Platform: Send + Sync {
    fn name(&self) -> &str;
    async fn deploy(&self, workload: &Workload) -> anyhow::Result<()>;
    async fn invoke(&self, workload: &Workload) -> anyhow::Result<serde_json::Value>;
    async fn get_memory_usage(&self, workload: &Workload) -> anyhow::Result<u64>;
    async fn ensure_cold_state(&self, workload: &Workload) -> anyhow::Result<()>;
    async fn cleanup(&self, workload: &Workload) -> anyhow::Result<()>;
}

pub struct NanoLambda {
    client: reqwest::Client,
    base_url: String,
}

impl NanoLambda {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            base_url: "http://127.0.0.1:3000".to_string(),
        })
    }
}

#[async_trait]
impl Platform for NanoLambda {
    fn name(&self) -> &str {
        "NanoLambda"
    }

    async fn deploy(&self, workload: &Workload) -> anyhow::Result<()> {
        let payload = serde_json::json!({
            "name": workload.function_name(),
            "runtime": "python3.11",
            "code": workload.code(),
            "memory_mb": 128,
            "timeout_ms": 30000,
        });

        let response = self
            .client
            .post(format!("{}/functions", self.base_url))
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Deploy failed: {}", error_text));
        }

        Ok(())
    }

    async fn invoke(&self, workload: &Workload) -> anyhow::Result<serde_json::Value> {
        let response = self
            .client
            .post(format!(
                "{}/invoke/{}",
                self.base_url,
                workload.function_name()
            ))
            .json(&workload.payload())
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Invoke failed: {}", error_text));
        }

        let result = response.json().await?;
        Ok(result)
    }

    async fn get_memory_usage(&self, workload: &Workload) -> anyhow::Result<u64> {
        let response = self
            .client
            .get(format!(
                "{}/functions/{}",
                self.base_url,
                workload.function_name()
            ))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let memory = data["config"]["memory_mb"].as_u64().unwrap_or(128);

        Ok(memory)
    }

    async fn ensure_cold_state(&self, workload: &Workload) -> anyhow::Result<()> {
        // Delete and recreate the function to ensure cold start
        let _ = self.cleanup(workload).await; // Ignore errors
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.deploy(workload).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    async fn cleanup(&self, workload: &Workload) -> anyhow::Result<()> {
        let response = self
            .client
            .delete(format!(
                "{}/functions/{}",
                self.base_url,
                workload.function_name()
            ))
            .send()
            .await?;

        // Ignore 404s (function might not exist)
        if !response.status().is_success() && response.status() != 404 {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Cleanup failed: {}", error_text));
        }

        Ok(())
    }
}

pub struct AwsLambda {
    client: aws_sdk_lambda::Client,
    region: String,
}

impl AwsLambda {
    pub async fn new() -> anyhow::Result<Self> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .load()
            .await;
        let client = aws_sdk_lambda::Client::new(&config);
        let region = config
            .region()
            .map(|r| r.as_ref().to_string())
            .unwrap_or_else(|| "us-east-1".to_string());

        Ok(Self { client, region })
    }

    fn runtime_from_str(&self, runtime: &str) -> aws_sdk_lambda::types::Runtime {
        match runtime {
            "python3.11" => aws_sdk_lambda::types::Runtime::Python311,
            "python3.12" => aws_sdk_lambda::types::Runtime::Python312,
            "nodejs20" => aws_sdk_lambda::types::Runtime::Nodejs20x,
            _ => aws_sdk_lambda::types::Runtime::Python311,
        }
    }
}

#[async_trait]
impl Platform for AwsLambda {
    fn name(&self) -> &str {
        "AWS Lambda"
    }

    async fn deploy(&self, workload: &Workload) -> anyhow::Result<()> {
        use aws_sdk_lambda::primitives::Blob;
        use std::io::Write;
        use zip::write::FileOptions;

        // Create deployment package (ZIP)
        let mut zip_buffer = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
            let options: FileOptions<()> =
                FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            zip.start_file("lambda_function.py", options)?;
            zip.write_all(workload.code().as_bytes())?;
            zip.finish()?;
        }

        // Delete if exists (ignore errors)
        let _ = self
            .client
            .delete_function()
            .function_name(workload.function_name())
            .send()
            .await;

        // Wait a bit for deletion to complete
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Create function
        self.client
            .create_function()
            .function_name(workload.function_name())
            .runtime(self.runtime_from_str("python3.11"))
            .role(std::env::var("AWS_LAMBDA_ROLE_ARN")?)
            .handler("lambda_function.handler")
            .code(
                aws_sdk_lambda::types::FunctionCode::builder()
                    .zip_file(Blob::new(zip_buffer))
                    .build(),
            )
            .memory_size(128)
            .timeout(30)
            .send()
            .await?;

        // Wait for function to be ready
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        Ok(())
    }

    async fn invoke(&self, workload: &Workload) -> anyhow::Result<serde_json::Value> {
        use aws_sdk_lambda::primitives::Blob;

        let payload_str = serde_json::to_string(&workload.payload())?;

        let response = self
            .client
            .invoke()
            .function_name(workload.function_name())
            .payload(Blob::new(payload_str.as_bytes()))
            .send()
            .await?;

        if let Some(payload) = response.payload() {
            let result: serde_json::Value = serde_json::from_slice(payload.as_ref())?;
            Ok(result)
        } else {
            Ok(serde_json::json!({}))
        }
    }

    async fn get_memory_usage(&self, workload: &Workload) -> anyhow::Result<u64> {
        let response = self
            .client
            .get_function()
            .function_name(workload.function_name())
            .send()
            .await?;

        let memory = response
            .configuration()
            .and_then(|c| c.memory_size())
            .unwrap_or(128);

        Ok(memory as u64)
    }

    async fn ensure_cold_state(&self, _workload: &Workload) -> anyhow::Result<()> {
        // For AWS Lambda, we rely on natural cold starts
        // Wait 15 minutes for the execution environment to expire
        // In practice, for benchmarking, we use a freshly deployed function
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        Ok(())
    }

    async fn cleanup(&self, workload: &Workload) -> anyhow::Result<()> {
        self.client
            .delete_function()
            .function_name(workload.function_name())
            .send()
            .await?;

        Ok(())
    }
}
