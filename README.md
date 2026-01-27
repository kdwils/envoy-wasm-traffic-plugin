# Envoy WASM Traffic Plugin

A WebAssembly plugin for Envoy Gateway that sends HTTP request events to a webhook service.

## Features

- Extracts real client IP from X-Forwarded-For headers with trusted proxy support
- Automatically includes client IP and authority in webhook payload
- Supports trusted proxy IP/CIDR ranges for secure IP extraction
- Extracts configured headers from incoming HTTP requests
- Sends event data as JSON to a webhook endpoint via HTTP POST
- Non-blocking fire-and-forget webhook calls

## Building

Build the WASM plugin:

```bash
cargo build --target wasm32-wasip1 --release
```

The compiled WASM file will be at: `target/wasm32-wasip1/release/envoy_wasm_traffic_plugin.wasm`

## Local Testing

Test locally using Docker Compose and Envoy:

```bash
# Build the plugin
cargo build --target wasm32-wasip1 --release

# Start Envoy
docker-compose up

# In another terminal, send a test request
curl -v http://localhost:10000/test

# Check the logs
docker-compose logs envoy
```

## Production Deployment

See [ENVOY_GATEWAY_SETUP.md](./ENVOY_GATEWAY_SETUP.md) for complete instructions on deploying with Envoy Gateway.

### Quick Summary

1. Create an `EnvoyPatchPolicy` to define the webhook cluster
2. Configure your `EnvoyExtensionPolicy` with:
   - `webhook_cluster`: Name of the cluster (from EnvoyPatchPolicy)
   - `webhook_path`: HTTP path for the webhook
   - `headers`: List of headers to extract and send

Example configuration:

```yaml
config:
  webhook_cluster: "webhook_cluster"
  webhook_authority: "events.int.kyledev.co"
  webhook_path: "/api/v1/events/envoy-gateway/http_request"
  headers:
    - ":method"
    - ":path"
  trusted_proxies:
    - "10.0.0.0/8"
    - "172.16.0.0/12"
    - "192.168.0.0/16"
```

## Configuration Reference

### webhook_cluster (required)
The name of the Envoy cluster to use for HTTP calls. Must be defined via EnvoyPatchPolicy.

### webhook_authority (required)
The hostname for the `:authority` HTTP/2 header. Should match the actual hostname of your webhook service.

### webhook_path (required)
The HTTP path to POST events to on the webhook service.

### headers (optional)
Array of header names to extract from requests. Common values:
- `:method` - HTTP method (GET, POST, etc.)
- `:path` - Request path
- Any custom headers

Note: `client_ip` and `authority` are included by default and don't need to be in this list.

### trusted_proxies (optional)
Array of IP addresses or CIDR ranges representing trusted proxies. When a request comes from a trusted proxy, the plugin will parse the `X-Forwarded-For` header to extract the real client IP.

Examples:
- Single IP: `"192.168.1.1/32"`
- CIDR range: `"10.0.0.0/8"`
- Private networks: `["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"]`

**How it works:**
1. If the peer IP is **not** in `trusted_proxies`, the peer IP is used as the client IP
2. If the peer IP **is** in `trusted_proxies`, the plugin parses `X-Forwarded-For`
3. Walks backwards through the XFF chain to find the first untrusted IP
4. That untrusted IP is the real client IP

This prevents IP spoofing by only trusting XFF headers from known proxies.

## Webhook Payload

The plugin sends a JSON payload with the following structure:

```json
{
  "client_ip": "203.0.113.42",
  "authority": "example.com",
  "headers": {
    ":method": "GET",
    ":path": "/api/users"
  }
}
```

**Default fields** (always included):
- `client_ip` - Real client IP extracted from X-Forwarded-For (if from trusted proxy) or peer IP
- `authority` - The `:authority` header (Host header)

**Additional headers:**
Only headers specified in the `headers` configuration array that exist in the request are included.

## Development

### Prerequisites

- Rust toolchain
- `wasm32-wasip1` target: `rustup target add wasm32-wasip1`
- Docker and Docker Compose (for local testing)

### Project Structure

```
.
├── src/
│   └── lib.rs              # Plugin implementation
├── Cargo.toml              # Rust dependencies
├── envoy.yaml              # Local Envoy configuration
├── docker-compose.yaml     # Local testing setup
├── extension.yaml          # Example EnvoyExtensionPolicy
├── ENVOY_GATEWAY_SETUP.md  # Production deployment guide
└── README.md               # This file
```

## License

See LICENSE file for details.
