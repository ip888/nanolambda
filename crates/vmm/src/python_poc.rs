//! Python runtime support for POC
//!
//! Basic Python function execution in VM.

use std::path::PathBuf;
use tracing::{info, debug, error};
use crate::{Result, VmmError};
use crate::vm_poc::{SecurityConfig, ResourceLimits};
use crate::vm_poc::{Vm, VmConfig};

/// Python function configuration
#[derive(Debug, Clone)]
pub struct PythonFunctionConfig {
    /// Function code
    pub code: String,
    /// Function name to call
    pub handler: String,
    /// Function arguments (JSON)
    pub args: Option<String>,
}

/// Python runtime manager
pub struct PythonRuntime {
    vm: Vm,
    working_dir: PathBuf,
}

impl PythonRuntime {
    /// Create a new Python runtime
    pub fn new() -> Result<Self> {
        info!("Initializing Python runtime");

        // Create VM for Python environment
        let config = VmConfig {
            memory_size_mb: 256, // More memory for Python
            vcpu_count: 1,
            kernel_path: None,
            security: SecurityConfig::default(),
            limits: ResourceLimits::default(),
            enable_hardware_virtualization: true,
            enable_ksm: false,
            enable_ballooning: false,
        };

        let vm = Vm::new(config)?;

        // Create working directory
        let working_dir = std::env::temp_dir().join("nanolambda-python");
        std::fs::create_dir_all(&working_dir)?;

        // Create Python environment setup script
        let setup_script = r#"
#!/bin/sh
# Basic Python environment setup
apt-get update
apt-get install -y python3 python3-pip

# Create virtualenv
python3 -m venv /opt/venv
source /opt/venv/bin/activate

# Install basic requirements
pip install jsonschema requests"#;

        let setup_path = working_dir.join("setup.sh");
        std::fs::write(&setup_path, setup_script)?;

        // Make script executable
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&setup_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&setup_path, perms)?;

        Ok(Self {
            vm,
            working_dir,
        })
    }

    /// Get the underlying VM
    pub fn get_vm(&self) -> &Vm {
        &self.vm
    }

    /// Get the working directory
    pub fn get_working_dir(&self) -> &PathBuf {
        &self.working_dir
    }

    /// Start the Python runtime VM
    pub fn start(&mut self) -> Result<()> {
        self.vm.run()
    }

    /// Execute a Python function
    pub fn execute_function(&mut self, function: PythonFunctionConfig) -> Result<String> {
        info!("Executing Python function: {}", function.handler);

        // Write function code to file
        let func_path = self.working_dir.join("function.py");
        std::fs::write(&func_path, &function.code)?;

        // Create wrapper script to handle imports and execution
        let wrapper = format!(
            r#"#!/usr/bin/env python3
import json
import sys
from pathlib import Path

def main():
    # Setup environment
    function_dir = Path('{}')
    sys.path.insert(0, str(function_dir))

    # Import user function
    try:
        from function import {}
    except ImportError as e:
        print(json.dumps({{"error": f"Import error: {{e}}"}}))
        sys.exit(1)
    except Exception as e:
        print(json.dumps({{"error": f"Setup error: {{e}}"}}))
        sys.exit(1)

    # Parse arguments
    try:
        args = {}
    except json.JSONDecodeError as e:
        print(json.dumps({{"error": f"Invalid JSON arguments: {{e}}"}}))
        sys.exit(1)

    # Execute function
    try:
        result = {}(*args if args else [])
        print(json.dumps({{"result": result}}))
    except Exception as e:
        print(json.dumps({{"error": f"Execution error: {{e}}"}}))
        sys.exit(1)

if __name__ == '__main__':
    main()
"#,
            self.working_dir.display(),
            function.handler,
            function.args.unwrap_or_else(|| "[]".to_string()),
            function.handler
        );

        // Write wrapper script
        let wrapper_path = self.working_dir.join("wrapper.py");
        std::fs::write(&wrapper_path, wrapper)?;

        // Make wrapper executable
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&wrapper_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&wrapper_path, perms)?;

        // Execute the function in the VM
        let output = self.execute_python_script(&wrapper_path)?;

        // Parse and return the JSON output
        match serde_json::from_str::<serde_json::Value>(&output) {
            Ok(value) => {
                if let Some(error) = value.get("error") {
                    Err(VmmError::RuntimeError(error.as_str().unwrap_or("Unknown error").to_string()))
                } else if let Some(result) = value.get("result") {
                    Ok(result.to_string())
                } else {
                    Err(VmmError::RuntimeError("Invalid response format".to_string()))
                }
            }
            Err(e) => Err(VmmError::RuntimeError(format!("Failed to parse result: {}", e)))
        }
    }

    fn execute_python_script(&mut self, script_path: &PathBuf) -> Result<String> {
        debug!("Executing Python script in VM: {}", script_path.display());
        
        // Copy script to VM
        let vm_script_path = PathBuf::from("/tmp/function.py");
        self.vm.copy_file_to_guest(script_path, &vm_script_path)?;
        
        // Prepare Python environment in VM
        self.vm.execute_command("python3 -m venv /tmp/venv")?;
        self.vm.execute_command("source /tmp/venv/bin/activate")?;
        
        // Execute script in VM
        let result = self.vm.execute_command(&format!(
            "python3 {} 2>&1",
            vm_script_path.display()
        ))?;
        
        // Parse output
        if result.contains("Traceback") || result.contains("Error:") {
            error!("Python script execution failed in VM: {}", result);
            return Err(VmmError::RuntimeError(format!("Python script failed in VM: {}", result)));
        }
        
        debug!("Python script execution output from VM: {}", result);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_runtime_creation() {
        let runtime = PythonRuntime::new();
        assert!(runtime.is_ok(), "Python runtime creation should succeed");
    }

    #[test]
    fn test_simple_function_execution() {
        let mut runtime = PythonRuntime::new().unwrap();
        
        let function = PythonFunctionConfig {
            code: r#"
def handler():
    return {"message": "Hello from Python!"}
            "#.to_string(),
            handler: "handler".to_string(),
            args: None,
        };

        let result = runtime.execute_function(function).unwrap();
        assert!(result.contains("Hello from Python!"));
    }

    #[test]
    fn test_function_with_arguments() {
        let mut runtime = PythonRuntime::new().unwrap();
        
        let function = PythonFunctionConfig {
            code: r#"
def handler(name):
    return {"message": f"Hello, {name}!"}
            "#.to_string(),
            handler: "handler".to_string(),
            args: Some(r#"["World"]"#.to_string()),
        };

        let result = runtime.execute_function(function).unwrap();
        assert!(result.contains("Hello, World!"));
    }
}