//! Fetch API for QuickJS Runtime
//!
//! Implements the Web Fetch API for making HTTP requests from JavaScript.
//! In Cloudflare Workers, fetch is backed by the native Workers fetch binding.
//!
//! # Example
//!
//! ```javascript
//! const response = await fetch('https://api.example.com/data', {
//!     method: 'POST',
//!     headers: { 'Content-Type': 'application/json' },
//!     body: JSON.stringify({ key: 'value' })
//! });
//! const data = await response.json();
//! ```

use super::value::{JsArray, JsFunction, JsObject, JsVal};
use super::{QuickJsError, QuickJsResultType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum RequestMethod {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

impl std::fmt::Display for RequestMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestMethod::GET => write!(f, "GET"),
            RequestMethod::POST => write!(f, "POST"),
            RequestMethod::PUT => write!(f, "PUT"),
            RequestMethod::DELETE => write!(f, "DELETE"),
            RequestMethod::PATCH => write!(f, "PATCH"),
            RequestMethod::HEAD => write!(f, "HEAD"),
            RequestMethod::OPTIONS => write!(f, "OPTIONS"),
        }
    }
}

impl From<&str> for RequestMethod {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GET" => RequestMethod::GET,
            "POST" => RequestMethod::POST,
            "PUT" => RequestMethod::PUT,
            "DELETE" => RequestMethod::DELETE,
            "PATCH" => RequestMethod::PATCH,
            "HEAD" => RequestMethod::HEAD,
            "OPTIONS" => RequestMethod::OPTIONS,
            _ => RequestMethod::GET,
        }
    }
}

/// Request credentials mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RequestCredentials {
    #[default]
    SameOrigin,
    Include,
    Omit,
}

/// Request redirect mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RequestRedirect {
    #[default]
    Follow,
    Error,
    Manual,
}

/// Request cache mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RequestCache {
    #[default]
    Default,
    NoStore,
    Reload,
    NoCache,
    ForceCache,
    OnlyIfCached,
}

/// HTTP Headers collection
#[derive(Debug, Clone, Default)]
pub struct Headers {
    entries: HashMap<String, String>,
}

impl Headers {
    /// Create empty headers
    pub fn new() -> Self {
        Self::default()
    }

    /// Create headers from a map
    pub fn from_map(map: HashMap<String, String>) -> Self {
        // Normalize header names to lowercase
        let entries = map
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect();
        Self { entries }
    }

    /// Get a header value
    pub fn get(&self, name: &str) -> Option<&String> {
        self.entries.get(&name.to_lowercase())
    }

    /// Set a header value
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.entries
            .insert(name.into().to_lowercase(), value.into());
    }

    /// Append a header value (for multi-value headers)
    pub fn append(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let key = name.into().to_lowercase();
        let val = value.into();
        if let Some(existing) = self.entries.get_mut(&key) {
            existing.push_str(", ");
            existing.push_str(&val);
        } else {
            self.entries.insert(key, val);
        }
    }

    /// Delete a header
    pub fn delete(&mut self, name: &str) {
        self.entries.remove(&name.to_lowercase());
    }

    /// Check if header exists
    pub fn has(&self, name: &str) -> bool {
        self.entries.contains_key(&name.to_lowercase())
    }

    /// Get all entries
    pub fn entries(&self) -> impl Iterator<Item = (&String, &String)> {
        self.entries.iter()
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Get all values
    pub fn values(&self) -> impl Iterator<Item = &String> {
        self.entries.values()
    }

    /// Convert to JsVal Object
    pub fn to_js_object(&self) -> JsObject {
        let mut obj = JsObject::new();
        for (key, value) in &self.entries {
            obj.set(key.clone(), JsVal::String(value.clone()));
        }
        obj
    }

    /// Create from JsVal
    pub fn from_js_val(val: &JsVal) -> Self {
        let mut headers = Self::new();
        if let JsVal::Object(obj) = val {
            for (key, value) in obj.entries() {
                if let JsVal::String(v) = value {
                    headers.set(key.clone(), v.clone());
                } else {
                    headers.set(key.clone(), value.to_js_string());
                }
            }
        }
        headers
    }
}

/// Fetch Request object
#[derive(Debug, Clone)]
pub struct Request {
    /// Request URL
    pub url: String,
    /// HTTP method
    pub method: RequestMethod,
    /// Request headers
    pub headers: Headers,
    /// Request body (serialized)
    pub body: Option<String>,
    /// Credentials mode
    pub credentials: RequestCredentials,
    /// Redirect mode
    pub redirect: RequestRedirect,
    /// Cache mode
    pub cache: RequestCache,
    /// Request ID for tracking
    pub id: u64,
}

impl Request {
    /// Create a new GET request
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: RequestMethod::GET,
            headers: Headers::new(),
            body: None,
            credentials: RequestCredentials::default(),
            redirect: RequestRedirect::default(),
            cache: RequestCache::default(),
            id: 0,
        }
    }

    /// Create from URL and options object
    pub fn from_js_args(url: &JsVal, options: Option<&JsVal>) -> QuickJsResultType<Self> {
        let url_str = match url {
            JsVal::String(s) => s.clone(),
            _ => {
                return Err(QuickJsError::TypeError {
                    message: "URL must be a string".to_string(),
                });
            }
        };

        let mut request = Self::get(&url_str);

        if let Some(JsVal::Object(opts)) = options {
            // Method
            if let Some(JsVal::String(method)) = opts.get("method") {
                request.method = RequestMethod::from(method.as_str());
            }

            // Headers
            if let Some(headers_val) = opts.get("headers") {
                request.headers = Headers::from_js_val(headers_val);
            }

            // Body
            if let Some(body_val) = opts.get("body") {
                request.body = Some(match body_val {
                    JsVal::String(s) => s.clone(),
                    _ => serde_json::to_string(&body_val.to_json()).unwrap_or_default(),
                });
            }

            // Credentials
            if let Some(JsVal::String(creds)) = opts.get("credentials") {
                request.credentials = match creds.as_str() {
                    "include" => RequestCredentials::Include,
                    "omit" => RequestCredentials::Omit,
                    _ => RequestCredentials::SameOrigin,
                };
            }

            // Redirect
            if let Some(JsVal::String(redir)) = opts.get("redirect") {
                request.redirect = match redir.as_str() {
                    "error" => RequestRedirect::Error,
                    "manual" => RequestRedirect::Manual,
                    _ => RequestRedirect::Follow,
                };
            }

            // Cache
            if let Some(JsVal::String(cache)) = opts.get("cache") {
                request.cache = match cache.as_str() {
                    "no-store" => RequestCache::NoStore,
                    "reload" => RequestCache::Reload,
                    "no-cache" => RequestCache::NoCache,
                    "force-cache" => RequestCache::ForceCache,
                    "only-if-cached" => RequestCache::OnlyIfCached,
                    _ => RequestCache::Default,
                };
            }
        }

        Ok(request)
    }

    /// Convert to JsVal Object (for inspection)
    pub fn to_js_object(&self) -> JsObject {
        let mut obj = JsObject::new();
        obj.set("url", JsVal::String(self.url.clone()));
        obj.set("method", JsVal::String(self.method.to_string()));
        obj.set("headers", JsVal::Object(self.headers.to_js_object()));
        if let Some(body) = &self.body {
            obj.set("body", JsVal::String(body.clone()));
        }
        obj
    }
}

/// Fetch Response object
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code
    pub status: u16,
    /// Status text
    pub status_text: String,
    /// Response headers
    pub headers: Headers,
    /// Response body (text)
    pub body: String,
    /// Whether response was from redirect
    pub redirected: bool,
    /// Final URL after redirects
    pub url: String,
    /// Response type
    pub response_type: ResponseType,
    /// Whether body has been consumed
    body_used: bool,
}

/// Response type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseType {
    #[default]
    Basic,
    Cors,
    Error,
    Opaque,
    OpaqueRedirect,
}

impl Response {
    /// Create a successful response
    pub fn ok(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            status_text: "OK".to_string(),
            headers: Headers::new(),
            body: body.into(),
            redirected: false,
            url: String::new(),
            response_type: ResponseType::Basic,
            body_used: false,
        }
    }

    /// Create an error response
    pub fn error() -> Self {
        Self {
            status: 0,
            status_text: String::new(),
            headers: Headers::new(),
            body: String::new(),
            redirected: false,
            url: String::new(),
            response_type: ResponseType::Error,
            body_used: false,
        }
    }

    /// Create from status code and body
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        let status_text = status_text_for_code(status);
        Self {
            status,
            status_text,
            headers: Headers::new(),
            body: body.into(),
            redirected: false,
            url: String::new(),
            response_type: ResponseType::Basic,
            body_used: false,
        }
    }

    /// Check if response is OK (status 200-299)
    pub fn ok_status(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Get body as text
    pub fn text(&mut self) -> QuickJsResultType<String> {
        if self.body_used {
            return Err(QuickJsError::TypeError {
                message: "Body has already been consumed".to_string(),
            });
        }
        self.body_used = true;
        Ok(self.body.clone())
    }

    /// Get body as JSON (parsed)
    pub fn json(&mut self) -> QuickJsResultType<JsVal> {
        if self.body_used {
            return Err(QuickJsError::TypeError {
                message: "Body has already been consumed".to_string(),
            });
        }
        self.body_used = true;

        let parsed: serde_json::Value =
            serde_json::from_str(&self.body).map_err(|e| QuickJsError::SyntaxError {
                message: format!("JSON parse error: {}", e),
                line: 0,
                column: 0,
            })?;

        Ok(JsVal::from_json(parsed))
    }

    /// Clone response (for streaming/multiple reads)
    pub fn clone_response(&self) -> Self {
        Self {
            status: self.status,
            status_text: self.status_text.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            redirected: self.redirected,
            url: self.url.clone(),
            response_type: self.response_type,
            body_used: false, // Reset for clone
        }
    }

    /// Convert to JsVal Object (the Response object seen by JS)
    pub fn to_js_object(&self) -> JsObject {
        let mut obj = JsObject::new();
        obj.set("status", JsVal::Number(self.status as f64));
        obj.set("statusText", JsVal::String(self.status_text.clone()));
        obj.set("ok", JsVal::Boolean(self.ok_status()));
        obj.set("redirected", JsVal::Boolean(self.redirected));
        obj.set("url", JsVal::String(self.url.clone()));
        obj.set("headers", JsVal::Object(self.headers.to_js_object()));
        obj.set("bodyUsed", JsVal::Boolean(self.body_used));

        // Methods (stored as function references, will be handled by engine)
        obj.set("text", JsVal::Function(JsFunction::new("text", vec![], "")));
        obj.set("json", JsVal::Function(JsFunction::new("json", vec![], "")));
        obj.set(
            "clone",
            JsVal::Function(JsFunction::new("clone", vec![], "")),
        );

        // Store raw body for method calls (internal use)
        obj.set("_body", JsVal::String(self.body.clone()));

        obj
    }
}

/// Get status text for status code
fn status_text_for_code(code: u16) -> String {
    match code {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
    .to_string()
}

/// Fetch API implementation
///
/// This struct manages pending fetch requests and their resolution.
/// In a Workers environment, actual fetches are delegated to the host.
#[derive(Debug, Default)]
pub struct FetchApi {
    /// Next request ID
    next_id: u64,
    /// Pending requests (for async resolution)
    pending: HashMap<u64, Request>,
    /// Mock responses (for testing)
    mocks: HashMap<String, Response>,
}

impl FetchApi {
    /// Create new Fetch API instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a fetch request (returns request ID for async tracking)
    pub fn register_request(&mut self, request: Request) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut req = request;
        req.id = id;
        self.pending.insert(id, req);
        id
    }

    /// Get pending request by ID
    pub fn get_pending(&self, id: u64) -> Option<&Request> {
        self.pending.get(&id)
    }

    /// Remove and return pending request
    pub fn take_pending(&mut self, id: u64) -> Option<Request> {
        self.pending.remove(&id)
    }

    /// Add a mock response for testing
    pub fn add_mock(&mut self, url_pattern: impl Into<String>, response: Response) {
        self.mocks.insert(url_pattern.into(), response);
    }

    /// Clear all mocks
    pub fn clear_mocks(&mut self) {
        self.mocks.clear();
    }

    /// Execute fetch (with mocks for testing)
    pub fn execute_mock(&mut self, request: &Request) -> Option<Response> {
        // Check exact match first
        if let Some(response) = self.mocks.get(&request.url) {
            return Some(response.clone_response());
        }

        // Check pattern matches (simple prefix/suffix)
        for (pattern, response) in &self.mocks {
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                if request.url.starts_with(prefix) {
                    return Some(response.clone_response());
                }
            }
            if pattern.starts_with('*') {
                let suffix = &pattern[1..];
                if request.url.ends_with(suffix) {
                    return Some(response.clone_response());
                }
            }
        }

        None
    }

    /// Create the global fetch function object
    pub fn create_fetch_function() -> JsFunction {
        JsFunction::new("fetch", vec!["url".to_string(), "options".to_string()], "")
            .with_async(true)
    }

    /// Create the Request constructor object
    pub fn create_request_constructor() -> JsObject {
        let mut obj = JsObject::new();
        obj.set("prototype", JsVal::Object(JsObject::new()));
        obj
    }

    /// Create the Response constructor object
    pub fn create_response_constructor() -> JsObject {
        let mut obj = JsObject::new();

        // Static methods
        obj.set(
            "error",
            JsVal::Function(JsFunction::new("error", vec![], "")),
        );
        obj.set(
            "redirect",
            JsVal::Function(JsFunction::new(
                "redirect",
                vec!["url".to_string(), "status".to_string()],
                "",
            )),
        );
        obj.set(
            "json",
            JsVal::Function(JsFunction::new(
                "json",
                vec!["data".to_string(), "init".to_string()],
                "",
            )),
        );

        obj.set("prototype", JsVal::Object(JsObject::new()));
        obj
    }

    /// Create Headers constructor
    pub fn create_headers_constructor() -> JsObject {
        let mut obj = JsObject::new();
        obj.set("prototype", JsVal::Object(JsObject::new()));
        obj
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_method_from_str() {
        assert_eq!(RequestMethod::from("GET"), RequestMethod::GET);
        assert_eq!(RequestMethod::from("post"), RequestMethod::POST);
        assert_eq!(RequestMethod::from("PUT"), RequestMethod::PUT);
        assert_eq!(RequestMethod::from("delete"), RequestMethod::DELETE);
        assert_eq!(RequestMethod::from("unknown"), RequestMethod::GET);
    }

    #[test]
    fn test_headers_operations() {
        let mut headers = Headers::new();
        headers.set("Content-Type", "application/json");
        headers.set("X-Custom", "value");

        assert_eq!(
            headers.get("content-type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert!(headers.has("x-custom"));

        headers.append("accept", "text/html");
        headers.append("accept", "application/json");
        assert!(headers.get("accept").unwrap().contains("text/html"));
        assert!(headers.get("accept").unwrap().contains("application/json"));

        headers.delete("x-custom");
        assert!(!headers.has("x-custom"));
    }

    #[test]
    fn test_request_from_url() {
        let url = JsVal::String("https://api.example.com/data".to_string());
        let request = Request::from_js_args(&url, None).expect("Should create request");

        assert_eq!(request.url, "https://api.example.com/data");
        assert_eq!(request.method, RequestMethod::GET);
        assert!(request.body.is_none());
    }

    #[test]
    fn test_request_with_options() {
        let url = JsVal::String("https://api.example.com/data".to_string());
        let mut opts = JsObject::new();
        opts.set("method", JsVal::String("POST".to_string()));

        let mut headers = JsObject::new();
        headers.set(
            "Content-Type",
            JsVal::String("application/json".to_string()),
        );
        opts.set("headers", JsVal::Object(headers));

        opts.set("body", JsVal::String(r#"{"key":"value"}"#.to_string()));

        let request =
            Request::from_js_args(&url, Some(&JsVal::Object(opts))).expect("Should create request");

        assert_eq!(request.method, RequestMethod::POST);
        assert!(request.headers.has("content-type"));
        assert_eq!(request.body, Some(r#"{"key":"value"}"#.to_string()));
    }

    #[test]
    fn test_response_ok() {
        let response = Response::ok(r#"{"data": "test"}"#);
        assert_eq!(response.status, 200);
        assert!(response.ok_status());
        assert_eq!(response.status_text, "OK");
    }

    #[test]
    fn test_response_body_consumption() {
        let mut response = Response::ok("body content");

        let text = response.text().expect("Should get text");
        assert_eq!(text, "body content");

        // Second read should fail
        let result = response.text();
        assert!(result.is_err());
    }

    #[test]
    fn test_response_json() {
        let mut response = Response::ok(r#"{"key": "value", "num": 42}"#);
        let json = response.json().expect("Should parse JSON");

        if let JsVal::Object(obj) = json {
            assert_eq!(obj.get("key"), Some(&JsVal::String("value".to_string())));
            assert_eq!(obj.get("num"), Some(&JsVal::Number(42.0)));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_response_clone() {
        let mut response = Response::ok("body");
        let _ = response.text(); // Consume body

        let cloned = response.clone_response();
        assert!(!cloned.body_used); // Clone has fresh body
    }

    #[test]
    fn test_fetch_api_mock() {
        let mut api = FetchApi::new();

        // Add mock
        api.add_mock(
            "https://api.example.com/users",
            Response::ok(r#"[{"id": 1}]"#),
        );
        api.add_mock(
            "https://api.example.com/*",
            Response::new(200, "wildcard match"),
        );

        // Exact match
        let request = Request::get("https://api.example.com/users");
        let response = api.execute_mock(&request).expect("Should match mock");
        assert_eq!(response.status, 200);
        assert!(response.body.contains("id"));

        // Wildcard match
        let request = Request::get("https://api.example.com/posts/123");
        let response = api.execute_mock(&request).expect("Should match wildcard");
        assert!(response.body.contains("wildcard"));

        // No match
        let request = Request::get("https://other.com/data");
        let response = api.execute_mock(&request);
        assert!(response.is_none());
    }

    #[test]
    fn test_request_to_js_object() {
        let mut request = Request::get("https://example.com");
        request.headers.set("Accept", "application/json");

        let obj = request.to_js_object();
        assert_eq!(
            obj.get("url"),
            Some(&JsVal::String("https://example.com".to_string()))
        );
        assert_eq!(obj.get("method"), Some(&JsVal::String("GET".to_string())));
    }

    #[test]
    fn test_response_to_js_object() {
        let mut response = Response::new(201, "created");
        response.headers.set("location", "/new/resource");
        response.url = "https://api.example.com/resource".to_string();

        let obj = response.to_js_object();
        assert_eq!(obj.get("status"), Some(&JsVal::Number(201.0)));
        assert_eq!(
            obj.get("statusText"),
            Some(&JsVal::String("Created".to_string()))
        );
        assert_eq!(obj.get("ok"), Some(&JsVal::Boolean(true)));
    }

    #[test]
    fn test_headers_to_js_object() {
        let mut headers = Headers::new();
        headers.set("content-type", "application/json");
        headers.set("authorization", "Bearer token");

        let obj = headers.to_js_object();
        assert_eq!(
            obj.get("content-type"),
            Some(&JsVal::String("application/json".to_string()))
        );
        assert_eq!(
            obj.get("authorization"),
            Some(&JsVal::String("Bearer token".to_string()))
        );
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(status_text_for_code(200), "OK");
        assert_eq!(status_text_for_code(404), "Not Found");
        assert_eq!(status_text_for_code(500), "Internal Server Error");
        assert_eq!(status_text_for_code(999), "Unknown");
    }
}
