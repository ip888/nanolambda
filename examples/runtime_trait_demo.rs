//! Example of using the Runtime trait for language-agnostic function execution
//!
//! This demonstrates how the Runtime trait provides a consistent interface
//! regardless of the underlying programming language.
//!
//! Run with: cargo run --example runtime_trait_demo

use nanolambda_runtime::{GenericFunctionConfig, Language};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Runtime Trait Demo ===\n");

    // In the future, we could have a runtime manager that supports multiple languages
    // For now, we'll demonstrate the trait design with a conceptual example

    println!("The Runtime trait provides a unified interface for:");
    println!("  • Python functions");
    println!("  • Node.js functions");
    println!("  • Java functions");
    println!("  • Any future language runtimes\n");

    // Example Python configuration
    let python_config = GenericFunctionConfig {
        name: "python-example".to_string(),
        language: Language::Python,
        code: r#"
def handler(event, context):
    name = event.get('name', 'World')
    return {'message': f'Hello from Python, {name}!'}
"#
        .to_string(),
        handler: "handler".to_string(),
        environment: HashMap::new(),
        memory_limit_mb: 256,
        timeout_seconds: 30,
        working_dir: None,
        extra_config: None,
    };

    // Example Node.js configuration (future)
    let nodejs_config = GenericFunctionConfig {
        name: "nodejs-example".to_string(),
        language: Language::NodeJS,
        code: r#"
exports.handler = async (event) => {
    const name = event.name || 'World';
    return { message: `Hello from Node.js, ${name}!` };
};
"#
        .to_string(),
        handler: "handler".to_string(),
        environment: HashMap::new(),
        memory_limit_mb: 256,
        timeout_seconds: 30,
        working_dir: None,
        extra_config: None,
    };

    println!("Python Function Config:");
    println!("  Name: {}", python_config.name);
    println!("  Language: {}", python_config.language);
    println!("  Handler: {}", python_config.handler);
    println!("  Memory Limit: {} MB", python_config.memory_limit_mb);
    println!("  Timeout: {} seconds\n", python_config.timeout_seconds);

    println!("Node.js Function Config (future):");
    println!("  Name: {}", nodejs_config.name);
    println!("  Language: {}", nodejs_config.language);
    println!("  Handler: {}", nodejs_config.handler);
    println!("  Memory Limit: {} MB", nodejs_config.memory_limit_mb);
    println!("  Timeout: {} seconds\n", nodejs_config.timeout_seconds);

    // Demonstrate the Runtime trait usage (conceptual)
    println!("=== Runtime Trait Interface ===\n");
    println!("trait Runtime {{");
    println!("    async fn execute(&self, config: &GenericFunctionConfig, event: Value)");
    println!("        -> Result<ExecutionResult>;");
    println!("    fn runtime_info(&self) -> RuntimeInfo;");
    println!("    fn health_check(&self) -> Result<()>;");
    println!("    fn set_warm_starts(&mut self, enabled: bool);");
    println!("}}\n");

    println!("Usage example:");
    println!("  // Python runtime");
    println!("  let python_runtime = PythonRuntime::new()?;");
    println!("  let result = python_runtime.execute(&python_config, event).await?;");
    println!();
    println!("  // Node.js runtime (future)");
    println!("  let nodejs_runtime = NodeJSRuntime::new()?;");
    println!("  let result = nodejs_runtime.execute(&nodejs_config, event).await?;");
    println!();
    println!("  // Generic usage with trait objects");
    println!("  let runtime: Box<dyn Runtime> = match language {{");
    println!("      Language::Python => Box::new(PythonRuntime::new()?),");
    println!("      Language::NodeJS => Box::new(NodeJSRuntime::new()?),");
    println!("      _ => return Err(\"Unsupported language\"),");
    println!("  }};");
    println!("  let result = runtime.execute(&config, event).await?;");
    println!();

    println!("=== Benefits ===\n");
    println!("✓ Unified API across all languages");
    println!("✓ Type-safe language specification");
    println!("✓ Easy to add new language runtimes");
    println!("✓ Consistent metrics and error handling");
    println!("✓ Shared process pooling infrastructure");
    println!("✓ Language-agnostic monitoring and logging");

    println!("\n=== Next Steps ===\n");
    println!("1. Implement Runtime trait for PythonExecutor");
    println!("2. Create NodeJSRuntime with Runtime trait");
    println!("3. Build RuntimeManager to coordinate multiple languages");
    println!("4. Update API server to use Runtime trait");

    Ok(())
}
