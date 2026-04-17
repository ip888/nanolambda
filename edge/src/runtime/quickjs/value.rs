//! JavaScript Value Types
//!
//! This module provides value types for the QuickJS runtime.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JavaScript value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsVal {
    /// undefined value
    Undefined,
    /// null value
    Null,
    /// Boolean value
    Boolean(bool),
    /// Number value (f64 as per ECMAScript)
    Number(f64),
    /// String value
    String(String),
    /// Array value
    Array(JsArray),
    /// Object value
    Object(JsObject),
    /// Function (stored as source + captured environment)
    Function(JsFunction),
}

impl JsVal {
    /// Check if value is undefined
    pub fn is_undefined(&self) -> bool {
        matches!(self, JsVal::Undefined)
    }

    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, JsVal::Null)
    }

    /// Check if value is nullish (undefined or null)
    pub fn is_nullish(&self) -> bool {
        matches!(self, JsVal::Undefined | JsVal::Null)
    }

    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            JsVal::Undefined | JsVal::Null => false,
            JsVal::Boolean(b) => *b,
            JsVal::Number(n) => *n != 0.0 && !n.is_nan(),
            JsVal::String(s) => !s.is_empty(),
            JsVal::Array(_) | JsVal::Object(_) | JsVal::Function(_) => true,
        }
    }

    /// Convert to boolean
    pub fn to_boolean(&self) -> bool {
        self.is_truthy()
    }

    /// Convert to number
    pub fn to_number(&self) -> f64 {
        match self {
            JsVal::Undefined => f64::NAN,
            JsVal::Null => 0.0,
            JsVal::Boolean(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            JsVal::Number(n) => *n,
            JsVal::String(s) => s.parse().unwrap_or(f64::NAN),
            JsVal::Array(_) | JsVal::Object(_) | JsVal::Function(_) => f64::NAN,
        }
    }

    /// Convert to string
    pub fn to_js_string(&self) -> String {
        match self {
            JsVal::Undefined => "undefined".to_string(),
            JsVal::Null => "null".to_string(),
            JsVal::Boolean(b) => b.to_string(),
            JsVal::Number(n) => {
                if n.is_nan() {
                    "NaN".to_string()
                } else if n.is_infinite() {
                    if *n > 0.0 { "Infinity" } else { "-Infinity" }.to_string()
                } else if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    n.to_string()
                }
            }
            JsVal::String(s) => s.clone(),
            JsVal::Array(arr) => arr.join(","),
            JsVal::Object(_) => "[object Object]".to_string(),
            JsVal::Function(_) => "[Function]".to_string(),
        }
    }

    /// Get typeof string
    pub fn type_of(&self) -> &'static str {
        match self {
            JsVal::Undefined => "undefined",
            JsVal::Null => "object", // Historical quirk
            JsVal::Boolean(_) => "boolean",
            JsVal::Number(_) => "number",
            JsVal::String(_) => "string",
            JsVal::Array(_) => "object",
            JsVal::Object(_) => "object",
            JsVal::Function(_) => "function",
        }
    }

    /// Try to get as string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            JsVal::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as number
    pub fn as_number(&self) -> Option<f64> {
        match self {
            JsVal::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get as boolean
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            JsVal::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as object
    pub fn as_object(&self) -> Option<&JsObject> {
        match self {
            JsVal::Object(obj) => Some(obj),
            _ => None,
        }
    }

    /// Try to get as array
    pub fn as_array(&self) -> Option<&JsArray> {
        match self {
            JsVal::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Convert from serde_json::Value
    pub fn from_json(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => JsVal::Null,
            serde_json::Value::Bool(b) => JsVal::Boolean(b),
            serde_json::Value::Number(n) => JsVal::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => JsVal::String(s),
            serde_json::Value::Array(arr) => JsVal::Array(JsArray::from_json_array(arr)),
            serde_json::Value::Object(obj) => JsVal::Object(JsObject::from_json_object(obj)),
        }
    }

    /// Convert to serde_json::Value
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            JsVal::Undefined | JsVal::Null => serde_json::Value::Null,
            JsVal::Boolean(b) => serde_json::Value::Bool(*b),
            JsVal::Number(n) => {
                if n.is_nan() || n.is_infinite() {
                    serde_json::Value::Null
                } else {
                    serde_json::json!(*n)
                }
            }
            JsVal::String(s) => serde_json::Value::String(s.clone()),
            JsVal::Array(arr) => arr.to_json(),
            JsVal::Object(obj) => obj.to_json(),
            JsVal::Function(_) => serde_json::Value::Null,
        }
    }
}

impl Default for JsVal {
    fn default() -> Self {
        JsVal::Undefined
    }
}

impl From<bool> for JsVal {
    fn from(b: bool) -> Self {
        JsVal::Boolean(b)
    }
}

impl From<f64> for JsVal {
    fn from(n: f64) -> Self {
        JsVal::Number(n)
    }
}

impl From<i32> for JsVal {
    fn from(n: i32) -> Self {
        JsVal::Number(f64::from(n))
    }
}

impl From<String> for JsVal {
    fn from(s: String) -> Self {
        JsVal::String(s)
    }
}

impl From<&str> for JsVal {
    fn from(s: &str) -> Self {
        JsVal::String(s.to_string())
    }
}

/// JavaScript object (property map)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JsObject {
    /// Properties
    pub properties: HashMap<String, JsVal>,
    /// Prototype reference (for prototype chain)
    #[serde(skip)]
    pub prototype: Option<Box<JsObject>>,
}

impl JsObject {
    /// Create empty object
    pub fn new() -> Self {
        Self::default()
    }

    /// Get property
    pub fn get(&self, key: &str) -> Option<&JsVal> {
        self.properties.get(key)
    }

    /// Get property or undefined
    pub fn get_or_undefined(&self, key: &str) -> JsVal {
        self.properties
            .get(key)
            .cloned()
            .unwrap_or(JsVal::Undefined)
    }

    /// Set property
    pub fn set(&mut self, key: impl Into<String>, value: JsVal) {
        self.properties.insert(key.into(), value);
    }

    /// Check if has own property
    pub fn has_own(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    /// Get all values
    pub fn values(&self) -> Vec<JsVal> {
        self.properties.values().cloned().collect()
    }

    /// Get all entries (key-value pairs)
    pub fn entries(&self) -> impl Iterator<Item = (&String, &JsVal)> {
        self.properties.iter()
    }

    /// Convert from JSON object
    pub fn from_json_object(obj: serde_json::Map<String, serde_json::Value>) -> Self {
        let properties = obj
            .into_iter()
            .map(|(k, v)| (k, JsVal::from_json(v)))
            .collect();
        Self {
            properties,
            prototype: None,
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> serde_json::Value {
        let map: serde_json::Map<String, serde_json::Value> = self
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), v.to_json()))
            .collect();
        serde_json::Value::Object(map)
    }
}

/// JavaScript array
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JsArray {
    /// Elements
    pub elements: Vec<JsVal>,
}

impl JsArray {
    /// Create empty array
    pub fn new() -> Self {
        Self::default()
    }

    /// Create array from elements
    pub fn from_elements(elements: Vec<JsVal>) -> Self {
        Self { elements }
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get element at index
    pub fn get(&self, index: usize) -> JsVal {
        self.elements
            .get(index)
            .cloned()
            .unwrap_or(JsVal::Undefined)
    }

    /// Set element at index
    pub fn set(&mut self, index: usize, value: JsVal) {
        if index >= self.elements.len() {
            self.elements.resize(index + 1, JsVal::Undefined);
        }
        self.elements[index] = value;
    }

    /// Push element
    pub fn push(&mut self, value: JsVal) {
        self.elements.push(value);
    }

    /// Pop element
    pub fn pop(&mut self) -> JsVal {
        self.elements.pop().unwrap_or(JsVal::Undefined)
    }

    /// Join elements into string
    pub fn join(&self, separator: &str) -> String {
        self.elements
            .iter()
            .map(|v| v.to_js_string())
            .collect::<Vec<_>>()
            .join(separator)
    }

    /// Convert from JSON array
    pub fn from_json_array(arr: Vec<serde_json::Value>) -> Self {
        Self {
            elements: arr.into_iter().map(JsVal::from_json).collect(),
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::Value::Array(self.elements.iter().map(|v| v.to_json()).collect())
    }
}

/// JavaScript function representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsFunction {
    /// Function name (empty for anonymous)
    pub name: String,
    /// Parameter names
    pub params: Vec<String>,
    /// Function body source code
    pub body: String,
    /// Captured environment (closure)
    #[serde(skip)]
    pub closure: HashMap<String, JsVal>,
    /// Whether this is async
    pub is_async: bool,
    /// Whether this is a generator
    pub is_generator: bool,
    /// Whether this is an arrow function
    pub is_arrow: bool,
}

impl JsFunction {
    /// Create a new function
    pub fn new(name: impl Into<String>, params: Vec<String>, body: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params,
            body: body.into(),
            closure: HashMap::new(),
            is_async: false,
            is_generator: false,
            is_arrow: false,
        }
    }

    /// Create an async function
    pub fn new_async(
        name: impl Into<String>,
        params: Vec<String>,
        body: impl Into<String>,
    ) -> Self {
        let mut func = Self::new(name, params, body);
        func.is_async = true;
        func
    }

    /// Create an arrow function
    pub fn new_arrow(params: Vec<String>, body: impl Into<String>) -> Self {
        let mut func = Self::new("", params, body);
        func.is_arrow = true;
        func
    }

    /// Mark function as async (builder pattern)
    pub fn with_async(mut self, is_async: bool) -> Self {
        self.is_async = is_async;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_types() {
        assert!(JsVal::Undefined.is_undefined());
        assert!(JsVal::Null.is_null());
        assert!(JsVal::Null.is_nullish());
        assert!(JsVal::Undefined.is_nullish());
    }

    #[test]
    fn test_truthiness() {
        assert!(!JsVal::Undefined.is_truthy());
        assert!(!JsVal::Null.is_truthy());
        assert!(!JsVal::Boolean(false).is_truthy());
        assert!(!JsVal::Number(0.0).is_truthy());
        assert!(!JsVal::String(String::new()).is_truthy());

        assert!(JsVal::Boolean(true).is_truthy());
        assert!(JsVal::Number(1.0).is_truthy());
        assert!(JsVal::String("hello".to_string()).is_truthy());
        assert!(JsVal::Array(JsArray::new()).is_truthy());
        assert!(JsVal::Object(JsObject::new()).is_truthy());
    }

    #[test]
    fn test_type_of() {
        assert_eq!(JsVal::Undefined.type_of(), "undefined");
        assert_eq!(JsVal::Null.type_of(), "object");
        assert_eq!(JsVal::Boolean(true).type_of(), "boolean");
        assert_eq!(JsVal::Number(42.0).type_of(), "number");
        assert_eq!(JsVal::String("test".to_string()).type_of(), "string");
    }

    #[test]
    fn test_to_number() {
        assert_eq!(JsVal::Null.to_number(), 0.0);
        assert_eq!(JsVal::Boolean(true).to_number(), 1.0);
        assert_eq!(JsVal::Boolean(false).to_number(), 0.0);
        assert_eq!(JsVal::String("42".to_string()).to_number(), 42.0);
        assert!(JsVal::Undefined.to_number().is_nan());
    }

    #[test]
    fn test_to_string() {
        assert_eq!(JsVal::Undefined.to_js_string(), "undefined");
        assert_eq!(JsVal::Null.to_js_string(), "null");
        assert_eq!(JsVal::Number(42.0).to_js_string(), "42");
        assert_eq!(JsVal::Number(3.14).to_js_string(), "3.14");
        assert_eq!(JsVal::Boolean(true).to_js_string(), "true");
    }

    #[test]
    fn test_object_operations() {
        let mut obj = JsObject::new();
        obj.set("name", JsVal::String("test".to_string()));
        obj.set("value", JsVal::Number(42.0));

        assert!(obj.has_own("name"));
        assert!(!obj.has_own("missing"));
        assert_eq!(obj.get("name").and_then(|v| v.as_string()), Some("test"));
        assert!(obj.get("missing").is_none());
    }

    #[test]
    fn test_array_operations() {
        let mut arr = JsArray::new();
        arr.push(JsVal::Number(1.0));
        arr.push(JsVal::Number(2.0));
        arr.push(JsVal::Number(3.0));

        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(0).as_number(), Some(1.0));
        assert_eq!(arr.join(","), "1,2,3");
    }

    #[test]
    fn test_json_roundtrip() {
        // Note: integers become floats because JS only has 'number' type
        let json = serde_json::json!({
            "name": "test",
            "values": [1.0, 2.0, 3.0],
            "nested": { "flag": true }
        });

        let val = JsVal::from_json(json.clone());
        let back = val.to_json();

        assert_eq!(json, back);
    }
}
