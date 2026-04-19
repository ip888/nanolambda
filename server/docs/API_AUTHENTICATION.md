# API Authentication

NanoLambda uses **API key authentication** with Bearer tokens to secure all function management and invocation endpoints.

## Overview

- **Authentication Method**: Bearer token (HTTP Authorization header)
- **Key Format**: `nl_<64-character-hex-hash>`
- **Scope**: Per-key permissions control access to specific operations
- **Lifecycle**: Keys can be created, listed, and revoked

## Quick Start

### 1. Create Your First API Key

```bash
curl -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-first-key",
    "permissions": ["functions:create", "functions:invoke"]
  }'
```

**Response:**
```json
{
  "id": 1,
  "key": "nl_7f0c6136fbcd739e08db488e769633f0f416dca8cc4a065a9ffde65a6e8e75d8",
  "name": "my-first-key",
  "permissions": ["functions:create", "functions:invoke"],
  "created_at": 1764205689,
  "expires_at": null
}
```

⚠️ **Save this key immediately!** It won't be shown again.

### 2. Use Your API Key

Include the key in the `Authorization` header for all protected endpoints:

```bash
curl -X POST http://localhost:8080/functions \
  -H "Authorization: Bearer nl_7f0c6136fbcd739e08db488e769633f0f416dca8cc4a065a9ffde65a6e8e75d8" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hello-world",
    "runtime": "python3.12",
    "handler": "handler.main",
    "code": "def main(event, context):\n    return {\"message\": \"Hello, World!\"}",
    "memory_mb": 128,
    "timeout_ms": 5000
  }'
```

## API Endpoints

### Create API Key

**Endpoint:** `POST /auth/keys`  
**Auth Required:** ❌ No (public endpoint for bootstrap)  
**Description:** Creates a new API key

**Request Body:**
```json
{
  "name": "string",            // Required: Human-readable name
  "permissions": ["string"],   // Optional: Permission list (default: empty)
  "expires_at": 1735689600     // Optional: Unix timestamp (default: never)
}
```

**Response (201 Created):**
```json
{
  "id": 1,
  "key": "nl_<hash>",
  "name": "my-key",
  "permissions": ["functions:create"],
  "created_at": 1764205689,
  "expires_at": null
}
```

**Example:**
```bash
# Create key with expiration
curl -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{
    "name": "temp-key",
    "permissions": ["functions:invoke"],
    "expires_at": 1735689600
  }'
```

---

### List API Keys

**Endpoint:** `GET /auth/keys`  
**Auth Required:** ✅ Yes  
**Description:** Lists all API keys (actual key strings are hidden for security)

**Response (200 OK):**
```json
{
  "keys": [
    {
      "id": 1,
      "name": "my-key",
      "permissions": ["functions:create", "functions:invoke"],
      "status": "active",
      "created_at": 1764205689,
      "expires_at": null,
      "last_used_at": 1764205700
    }
  ],
  "count": 1
}
```

**Example:**
```bash
curl -X GET http://localhost:8080/auth/keys \
  -H "Authorization: Bearer nl_<your-key>"
```

---

### Revoke API Key

**Endpoint:** `DELETE /auth/keys/{id}`  
**Auth Required:** ✅ Yes  
**Description:** Revokes an API key (sets status to "revoked")

**Response (204 No Content)**

**Example:**
```bash
curl -X DELETE http://localhost:8080/auth/keys/1 \
  -H "Authorization: Bearer nl_<your-key>"
```

## Protected Endpoints

All the following endpoints require authentication:

### Function Management
- `POST /functions` - Create function
- `GET /functions` - List functions
- `GET /functions/{name}` - Get function details
- `PUT /functions/{name}` - Update function
- `DELETE /functions/{name}` - Delete function

### Function Invocation
- `POST /functions/{name}/invoke` - Invoke function

### Function Versioning
- `GET /functions/{name}/versions` - List versions
- `POST /functions/{name}/versions` - Publish new version
- `GET /functions/{name}/versions/{version}` - Get specific version

### API Key Management
- `GET /auth/keys` - List keys
- `DELETE /auth/keys/{id}` - Revoke key

## Public Endpoints

These endpoints do NOT require authentication:

- `POST /auth/keys` - Create API key (for initial bootstrap)
- `GET /health` - Health check

## Authentication Flow

```
Client Request
     │
     ├─→ Header: "Authorization: Bearer nl_<key>"
     │
     ├─→ Middleware extracts token
     │
     ├─→ Validate token in database
     │   ├─ Key exists?
     │   ├─ Status = "active"?
     │   └─ Not expired?
     │
     ├─→ Update last_used_at (async)
     │
     └─→ Add AuthContext to request
             │
             └─→ Handler processes request
```

## Error Responses

### 401 Unauthorized - Missing Token
```json
{
  "error": "Missing authorization token. Include 'Authorization: Bearer <token>' header.",
  "status": 401
}
```

### 401 Unauthorized - Invalid Token
```json
{
  "error": "Invalid or unknown API key",
  "status": 401
}
```

### 403 Forbidden - Revoked Key
```json
{
  "error": "API key has been revoked",
  "status": 403
}
```

### 403 Forbidden - Expired Key
```json
{
  "error": "API key has expired",
  "status": 403
}
```

## Permissions

Currently, permissions are stored but not enforced (placeholder for future RBAC):

**Standard Permissions:**
- `functions:create` - Create new functions
- `functions:read` - Read function details
- `functions:update` - Update existing functions
- `functions:delete` - Delete functions
- `functions:invoke` - Invoke functions
- `keys:create` - Create API keys
- `keys:read` - List API keys
- `keys:revoke` - Revoke API keys

*Note: Permission enforcement will be added in a future release.*

## Key Management Best Practices

### 1. Secure Storage
- Store keys in environment variables or secret managers
- Never commit keys to version control
- Rotate keys regularly

### 2. Least Privilege
- Create keys with minimal required permissions
- Use separate keys for different applications

### 3. Key Rotation
```bash
# 1. Create new key
NEW_KEY=$(curl -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "rotated-key"}' | jq -r '.key')

# 2. Update applications to use new key
export NANOLAMBDA_API_KEY=$NEW_KEY

# 3. Revoke old key
curl -X DELETE http://localhost:8080/auth/keys/1 \
  -H "Authorization: Bearer $NEW_KEY"
```

### 4. Expiration
Set expiration for temporary access:
```bash
# Key expires in 30 days
EXPIRES_AT=$(date -d '+30 days' +%s)
curl -X POST http://localhost:8080/auth/keys \
  -H "Content-Type: application/json" \
  -d "{\"name\": \"temp-key\", \"expires_at\": $EXPIRES_AT}"
```

## Multi-User Support

Each API key is independent, enabling multi-user scenarios:

```bash
# User 1 creates their key
curl -X POST http://localhost:8080/auth/keys \
  -d '{"name": "user1-key"}'

# User 2 creates their key
curl -X POST http://localhost:8080/auth/keys \
  -d '{"name": "user2-key"}'

# Both users can create functions independently
# (Currently functions are not isolated by key - future feature)
```

## Integration Examples

### Python
```python
import requests

API_KEY = "nl_7f0c6136fbcd739e08db488e769633f0f416dca8cc4a065a9ffde65a6e8e75d8"
BASE_URL = "http://localhost:8080"

headers = {
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json"
}

# Create function
response = requests.post(
    f"{BASE_URL}/functions",
    headers=headers,
    json={
        "name": "hello",
        "runtime": "python3.12",
        "handler": "handler.main",
        "code": "def main(event, context):\n    return {'message': 'Hello!'}",
        "memory_mb": 128,
        "timeout_ms": 5000
    }
)
print(response.json())
```

### JavaScript/Node.js
```javascript
const API_KEY = "nl_7f0c6136fbcd739e08db488e769633f0f416dca8cc4a065a9ffde65a6e8e75d8";
const BASE_URL = "http://localhost:8080";

const headers = {
  "Authorization": `Bearer ${API_KEY}`,
  "Content-Type": "application/json"
};

// Invoke function
fetch(`${BASE_URL}/functions/hello/invoke`, {
  method: "POST",
  headers,
  body: JSON.stringify({ name: "World" })
})
  .then(res => res.json())
  .then(data => console.log(data));
```

### cURL with .env File
```bash
# .env
export NANOLAMBDA_API_KEY="nl_7f0c6136fbcd739e08db488e769633f0f416dca8cc4a065a9ffde65a6e8e75d8"

# Load environment
source .env

# Make requests
curl -X GET http://localhost:8080/functions \
  -H "Authorization: Bearer $NANOLAMBDA_API_KEY"
```

## Security Considerations

### ✅ Implemented
- SHA256-based key generation (64-character hex)
- Secure database storage
- Bearer token authentication
- Key revocation
- Expiration timestamps
- Last used tracking

### 🔜 Future Enhancements
- Rate limiting per key
- IP whitelisting
- Permission enforcement (RBAC)
- Audit logging
- Key scopes (per-function access)
- JWT support

## Comparison with AWS Lambda

| Feature | NanoLambda | AWS Lambda |
|---------|------------|------------|
| Auth Method | API Keys | IAM + AWS Signature V4 |
| Bootstrap | POST /auth/keys | AWS Console/CLI setup |
| Key Format | `nl_<hash>` | AWS Access Key ID + Secret |
| Revocation | DELETE /auth/keys/{id} | IAM Console |
| Expiration | Optional timestamp | IAM policy |
| Permissions | Placeholder array | Full IAM policies |

NanoLambda's authentication is simpler and more developer-friendly for self-hosted scenarios, while AWS offers enterprise-grade IAM integration.

## Troubleshooting

### Issue: "Missing authorization token"
**Solution:** Ensure you're including the `Authorization` header:
```bash
-H "Authorization: Bearer nl_<your-key>"
```

### Issue: "Invalid or unknown API key"
**Solution:** 
- Verify the key is correct (check for typos)
- Confirm the key hasn't been revoked
- Create a new key if needed

### Issue: "API key has expired"
**Solution:** Create a new key:
```bash
curl -X POST http://localhost:8080/auth/keys \
  -d '{"name": "new-key"}'
```

### Issue: "API key has been revoked"
**Solution:** The key was deleted. Create a new one and update your application.

## Next Steps

- **Explore the API**: See all available endpoints in the main README
- **Production deployment**: Enable HTTPS and configure environment variables

---

**Last Updated:** 2026-04-19  
**NanoLambda Version:** 0.1.0
