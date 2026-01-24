# Envoy WASM Traffic Plugin

A WebAssembly plugin for Envoy Gateway that sends HTTP request events to a webhook service.

## Features

- Extracts configured headers from incoming HTTP requests
- Sends header data as JSON to a webhook endpoint via HTTP POST
- Configurable list of headers to track
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
    - ":authority"
    - "x-forwarded-for"
```

## Configuration Reference

### webhook_cluster (required)
The name of the Envoy cluster to use for HTTP calls. Must be defined via EnvoyPatchPolicy.

### webhook_authority (required)
The hostname for the `:authority` HTTP/2 header. Should match the actual hostname of your webhook service.

### webhook_path (required)
The HTTP path to POST events to on the webhook service.

### headers (required)
Array of header names to extract from requests. Common values:
- `:method` - HTTP method (GET, POST, etc.)
- `:path` - Request path
- `:authority` - Host header
- `x-forwarded-for` - Client IP
- Any custom headers

## Webhook Payload

The plugin sends a JSON payload with the following structure:

```json
{
  "headers": {
    ":method": "GET",
    ":path": "/api/users",
    ":authority": "example.com",
    "x-forwarded-for": "192.168.1.1"
  }
}
```

Only headers that exist in the request are included.

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
