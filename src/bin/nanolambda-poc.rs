//! Simple CLI for NanoLambda POC
//! This provides basic function deployment and execution capabilities.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use std::time::Instant;
use tracing::{info, error};
use nanolambda_vmm::{PythonRuntime, PythonFunctionConfig};

#[derive(Parser)]
#[command(name = "nanolambda-poc")]
#[command(about = "NanoLambda Function Runner (POC)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deploy a Python function
    Deploy {
        /// Path to Python file
        #[arg(short, long)]
        file: PathBuf,
        
        /// Function name to call
        #[arg(short, long)]
        handler: String,
    },
    /// Run a deployed function
    Run {
        /// Function name to execute
        #[arg(short, long)]
        name: String,
        
        /// JSON string of arguments
        #[arg(short, long)]
        args: Option<String>,
    },
    /// Run performance tests
    Bench {
        /// Number of iterations
        #[arg(short, long, default_value = "100")]
        iterations: usize,
    },
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Deploy { file, handler } => {
            info!("Deploying function from {} with handler {}", file.display(), handler);
            
            let start = Instant::now();
            
            // Read function code
            let code = fs::read_to_string(&file)?;
            
            // Create runtime
            let mut runtime = PythonRuntime::new()?;
            
            // Create function config
            let config = PythonFunctionConfig {
                code,
                handler,
                args: None,
            };
            
            // Verify function is valid by running it once
            runtime.execute_function(config)?;
            
            let duration = start.elapsed();
            info!("Function deployed successfully in {:?}!", duration);
        }
        
        Commands::Run { name, args } => {
            info!("Running function {} with args: {:?}", name, args);
            
            let start = Instant::now();
            
            // Create runtime
            let mut runtime = PythonRuntime::new()?;
            
            // Read function code from deployed location
            // TODO: In real implementation, this would come from storage
            let code = fs::read_to_string(format!("{}.py", name))?;
            
            // Create function config
            let config = PythonFunctionConfig {
                code,
                handler: "handler".to_string(), // Default handler name
                args,
            };
            
            // Execute function
            let result = runtime.execute_function(config)?;
            
            let duration = start.elapsed();
            println!("Result: {}", result);
            info!("Execution completed in {:?}", duration);
        }

        Commands::Bench { iterations } => {
            info!("Running performance benchmark with {} iterations", iterations);
            
            // Create runtime once
            let mut runtime = PythonRuntime::new()?;
            
            // Simple test function
            let config = PythonFunctionConfig {
                code: r#"
def handler():
    return {"message": "Hello from Python!"}
                "#.to_string(),
                handler: "handler".to_string(),
                args: None,
            };
            
            let mut total_duration = std::time::Duration::new(0, 0);
            let mut min_duration = std::time::Duration::new(u64::MAX, 0);
            let mut max_duration = std::time::Duration::new(0, 0);
            
            for i in 0..iterations {
                let start = Instant::now();
                runtime.execute_function(config.clone())?;
                let duration = start.elapsed();
                
                total_duration += duration;
                min_duration = min_duration.min(duration);
                max_duration = max_duration.max(duration);
                
                if (i + 1) % 10 == 0 {
                    info!("Completed {} iterations", i + 1);
                }
            }
            
            let avg_duration = total_duration / iterations as u32;
            
            println!("\nPerformance Results:");
            println!("---------------------");
            println!("Total iterations: {}", iterations);
            println!("Average duration: {:?}", avg_duration);
            println!("Minimum duration: {:?}", min_duration);
            println!("Maximum duration: {:?}", max_duration);
            println!("Total time: {:?}", total_duration);
        }
    }
    
    Ok(())
}