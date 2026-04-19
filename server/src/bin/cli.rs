//! NanoLambda CLI
//!
//! Command-line interface for managing functions.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Client;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "nanolambda")]
#[command(about = "Self-hosted serverless platform", long_about = None)]
#[command(version)]
struct Cli {
    /// API server URL
    #[arg(long, default_value = "http://localhost:8080")]
    url: String,

    /// API key for authentication
    #[arg(long)]
    api_key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new function
    Init {
        /// Function name
        name: String,

        /// Runtime (python)
        #[arg(short, long, default_value = "python")]
        runtime: String,
    },

    /// Deploy a function
    Deploy {
        /// Path to function directory or file
        #[arg(default_value = ".")]
        path: String,
    },

    /// Invoke a function
    Invoke {
        /// Function name
        name: String,

        /// Event data (JSON)
        #[arg(short, long)]
        data: Option<String>,

        /// Read event data from file
        #[arg(short, long)]
        file: Option<String>,
    },

    /// View function logs
    Logs {
        /// Function name
        name: String,

        /// Number of recent invocations to show
        #[arg(short, long, default_value = "10")]
        tail: usize,
    },

    /// List all functions
    List,

    /// Delete a function
    Delete {
        /// Function name
        name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check for API key
    let api_key = cli
        .api_key
        .or_else(|| std::env::var("NANOLAMBDA_API_KEY").ok())
        .context(
            "API key required. Set NANOLAMBDA_API_KEY environment variable or use --api-key",
        )?;

    let client = Client::new();

    match cli.command {
        Commands::Init { name, runtime } => {
            init_function(&name, &runtime)?;
        }
        Commands::Deploy { path } => {
            deploy_function(&client, &cli.url, &api_key, &path).await?;
        }
        Commands::Invoke { name, data, file } => {
            invoke_function(&client, &cli.url, &api_key, &name, data, file).await?;
        }
        Commands::Logs { name, tail } => {
            fetch_logs(&client, &cli.url, &api_key, &name, tail).await?;
        }
        Commands::List => {
            list_functions(&client, &cli.url, &api_key).await?;
        }
        Commands::Delete { name, force } => {
            delete_function(&client, &cli.url, &api_key, &name, force).await?;
        }
    }

    Ok(())
}

/// Initialize a new function template
fn init_function(name: &str, runtime: &str) -> Result<()> {
    println!("Initializing function: {} ({})", name, runtime);

    // Create function directory
    let dir = Path::new(name);
    fs::create_dir_all(dir)?;

    // Create function.json config
    let config = json!({
        "name": name,
        "runtime": runtime,
        "handler": match runtime {
            "python" => "handler",
            _ => "handler"
        },
        "memory_mb": 128,
        "timeout_ms": 5000,
        "environment": {}
    });

    fs::write(
        dir.join("function.json"),
        serde_json::to_string_pretty(&config)?,
    )?;

    // Create template code file
    let (code_file, template_code) = match runtime {
        "python" => (
            "handler.py",
            r#"def handler(event):
    """
    NanoLambda function handler.

    Args:
        event: Input event data (dict)

    Returns:
        dict: Response data
    """
    return {
        "message": "Hello from NanoLambda!",
        "event": event
    }
"#,
        ),
        _ => return Err(anyhow::anyhow!("Unsupported runtime: {}. Only Python is currently supported.", runtime)),
    };

    fs::write(dir.join(code_file), template_code)?;

    println!("✓ Created function template in ./{}", name);
    println!("  - function.json (configuration)");
    println!("  - {} (code)", code_file);
    println!("\nNext steps:");
    println!("  1. Edit {} to implement your function", code_file);
    println!("  2. Run 'nanolambda deploy {}' to deploy", name);

    Ok(())
}

/// Deploy a function from local directory
async fn deploy_function(client: &Client, url: &str, api_key: &str, path: &str) -> Result<()> {
    println!("Deploying function from: {}", path);

    // Read function.json
    let config_path = Path::new(path).join("function.json");
    let config_str = fs::read_to_string(&config_path)
        .context("function.json not found. Run 'nanolambda init' first.")?;
    let mut config: Value = serde_json::from_str(&config_str)?;

    // Read code file
    let runtime = config["runtime"]
        .as_str()
        .context("runtime not specified in function.json")?;
    let code_file = match runtime {
        "python" => "handler.py",
        _ => return Err(anyhow::anyhow!("Unsupported runtime: {}. Only Python is currently supported.", runtime)),
    };

    let code_path = Path::new(path).join(code_file);
    let code = fs::read_to_string(&code_path)
        .with_context(|| format!("Code file {} not found", code_file))?;

    // Add code to config
    config["code"] = json!(code);

    // Send to API
    let response = client
        .post(format!("{}/functions", url))
        .header("X-API-Key", api_key)
        .json(&config)
        .send()
        .await?;

    if response.status().is_success() {
        let result: Value = response.json().await?;
        println!("✓ Function deployed successfully");
        println!("  Name: {}", result["name"]);
        println!("  Runtime: {}", result["runtime"]);
        println!("  Version: {}", result["version"]);
    } else {
        let error: Value = response.json().await?;
        return Err(anyhow::anyhow!(
            "Deployment failed: {}",
            error["message"].as_str().unwrap_or("Unknown error")
        ));
    }

    Ok(())
}

/// Invoke a function via API
async fn invoke_function(
    client: &Client,
    url: &str,
    api_key: &str,
    name: &str,
    data: Option<String>,
    file: Option<String>,
) -> Result<()> {
    println!("Invoking function: {}", name);

    // Parse event data
    let event: Value = if let Some(file_path) = file {
        let content = fs::read_to_string(file_path)?;
        serde_json::from_str(&content)?
    } else if let Some(data_str) = data {
        serde_json::from_str(&data_str)?
    } else {
        json!({})
    };

    // Invoke function
    let response = client
        .post(format!("{}/functions/{}/invoke", url, name))
        .header("X-API-Key", api_key)
        .json(&json!({ "payload": event }))
        .send()
        .await?;

    if response.status().is_success() {
        let result: Value = response.json().await?;
        println!("✓ Function executed successfully");
        println!("\nResult:");
        println!("{}", serde_json::to_string_pretty(&result["result"])?);
        println!("\nMetrics:");
        println!("  Execution time: {} ms", result["execution_ms"]);
        println!("  Memory used: {} MB", result["memory_mb"]);
        println!("  Cold start: {}", result["cold_start"]);
    } else {
        let error: Value = response.json().await?;
        return Err(anyhow::anyhow!(
            "Invocation failed: {}",
            error["message"].as_str().unwrap_or("Unknown error")
        ));
    }

    Ok(())
}

/// Fetch function logs (recent invocations)
async fn fetch_logs(
    client: &Client,
    url: &str,
    api_key: &str,
    name: &str,
    tail: usize,
) -> Result<()> {
    println!("Fetching logs for: {}", name);

    // Get function to find its ID
    let func_response = client
        .get(format!("{}/functions/{}", url, name))
        .header("X-API-Key", api_key)
        .send()
        .await?;

    if !func_response.status().is_success() {
        return Err(anyhow::anyhow!("Function not found: {}", name));
    }

    let func: Value = func_response.json().await?;
    let function_id = func["id"].as_i64().context("Invalid function ID")?;

    // Get invocations
    let response = client
        .get(format!("{}/invocations", url))
        .header("X-API-Key", api_key)
        .query(&[
            ("function_id", function_id.to_string()),
            ("limit", tail.to_string()),
        ])
        .send()
        .await?;

    if response.status().is_success() {
        let invocations: Vec<Value> = response.json().await?;
        println!("\nRecent invocations (last {}):", tail);
        println!(
            "{:<36} {:<10} {:<12} {:<10}",
            "Request ID", "Status", "Duration", "Time"
        );
        println!("{}", "-".repeat(70));

        for inv in invocations {
            println!(
                "{:<36} {:<10} {:<12} {}",
                inv["request_id"].as_str().unwrap_or("N/A"),
                inv["status"].as_str().unwrap_or("unknown"),
                format!("{} ms", inv["execution_time_ms"].as_i64().unwrap_or(0)),
                inv["started_at"].as_str().unwrap_or("N/A")
            );
        }
    } else {
        println!("No logs available");
    }

    Ok(())
}

/// List all functions
async fn list_functions(client: &Client, url: &str, api_key: &str) -> Result<()> {
    println!("Fetching functions...");

    let response = client
        .get(format!("{}/functions", url))
        .header("X-API-Key", api_key)
        .send()
        .await?;

    if response.status().is_success() {
        let functions: Vec<Value> = response.json().await?;
        println!("\nFunctions ({}):", functions.len());
        println!(
            "{:<30} {:<10} {:<10} {:<15}",
            "Name", "Runtime", "Version", "Status"
        );
        println!("{}", "-".repeat(70));

        for func in functions {
            println!(
                "{:<30} {:<10} {:<10} {:<15}",
                func["name"].as_str().unwrap_or("N/A"),
                func["runtime"].as_str().unwrap_or("N/A"),
                func["version"].as_i64().unwrap_or(0),
                func["status"].as_str().unwrap_or("unknown")
            );
        }
    } else {
        return Err(anyhow::anyhow!("Failed to list functions"));
    }

    Ok(())
}

/// Delete a function
async fn delete_function(
    client: &Client,
    url: &str,
    api_key: &str,
    name: &str,
    force: bool,
) -> Result<()> {
    if !force {
        println!("Delete function '{}'? (y/N): ", name);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    println!("Deleting function: {}", name);

    let response = client
        .delete(format!("{}/functions/{}", url, name))
        .header("X-API-Key", api_key)
        .send()
        .await?;

    if response.status().is_success() {
        println!("✓ Function deleted successfully");
    } else {
        let error: Value = response.json().await?;
        return Err(anyhow::anyhow!(
            "Deletion failed: {}",
            error["message"].as_str().unwrap_or("Unknown error")
        ));
    }

    Ok(())
}
