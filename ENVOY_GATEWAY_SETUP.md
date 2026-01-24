# Envoy Gateway Setup for WASM Traffic Plugin

This guide explains how to configure the WASM traffic plugin with Envoy Gateway in production.

## Prerequisites

- Envoy Gateway installed and running
- A Gateway resource created

## Step 1: Enable EnvoyPatchPolicy

First, enable the EnvoyPatchPolicy feature in your Envoy Gateway configuration:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: envoy-gateway-config
  namespace: envoy-gateway-system
data:
  envoy-gateway.yaml: |
    apiVersion: gateway.envoyproxy.io/v1alpha1
    kind: EnvoyGateway
    provider:
      type: Kubernetes
    gateway:
      controllerName: gateway.envoyproxy.io/gatewayclass-controller
    extensionApis:
      enableEnvoyPatchPolicy: true
```

Then restart the Envoy Gateway deployment:

```bash
kubectl rollout restart deployment envoy-gateway -n envoy-gateway-system
```

## Step 2: Create EnvoyPatchPolicy for Webhook Cluster

Create an EnvoyPatchPolicy to add the webhook cluster that your WASM plugin will use for HTTP calls:

```yaml
apiVersion: gateway.envoyproxy.io/v1alpha1
kind: EnvoyPatchPolicy
metadata:
  name: webhook-cluster-patch
  namespace: default
spec:
  targetRef:
    group: gateway.networking.k8s.io
    kind: Gateway
    name: eg  # Replace with your gateway name
  type: JSONPatch
  jsonPatches:
    - type: "type.googleapis.com/envoy.config.cluster.v3.Cluster"
      name: webhook_cluster
      operation:
        op: add
        path: ""
        value:
          name: webhook_cluster
          type: STRICT_DNS
          connect_timeout: 5s
          lb_policy: ROUND_ROBIN
          load_assignment:
            cluster_name: webhook_cluster
            endpoints:
              - lb_endpoints:
                  - endpoint:
                      address:
                        socket_address:
                          address: events.int.kyledev.co  # Replace with your webhook host
                          port_value: 443
          transport_socket:
            name: envoy.transport_sockets.tls
            typed_config:
              "@type": type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.UpstreamTlsContext
              sni: events.int.kyledev.co  # Replace with your webhook host
```

Apply the patch:

```bash
kubectl apply -f webhook-cluster-patch.yaml
```

## Step 3: Configure EnvoyExtensionPolicy

Create your EnvoyExtensionPolicy with the WASM plugin configuration:

```yaml
apiVersion: gateway.envoyproxy.io/v1alpha1
kind: EnvoyExtensionPolicy
metadata:
  name: wasm-traffic-plugin
  namespace: default
spec:
  targetRefs:
    - group: gateway.networking.k8s.io
      kind: HTTPRoute
      name: myapp  # Replace with your HTTPRoute name
  wasm:
    - name: wasm-filter-1
      rootID: my-root-id
      code:
        type: Image
        image:
          url: my-registry/envoy-wasm-traffic-plugin:latest  # Replace with your image
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

Apply the extension policy:

```bash
kubectl apply -f envoy-extension-policy.yaml
```

## Configuration Options

### webhook_cluster (required)
The name of the Envoy cluster to use for HTTP calls. This must match the cluster name defined in your EnvoyPatchPolicy.

### webhook_authority (required)
The hostname to use in the `:authority` HTTP/2 pseudo-header. This should match the actual hostname of your webhook service (e.g., `events.int.kyledev.co`).

### webhook_path (required)
The HTTP path to send events to on the webhook service.

### headers (required)
List of header names to extract from incoming requests and include in the webhook payload. Common headers:
- `:method` - HTTP method (GET, POST, etc.)
- `:path` - Request path
- `:authority` - Host header
- `x-forwarded-for` - Client IP address
- Any custom headers you want to track

## Local Testing with Docker Compose

For local testing, see the included `envoy.yaml` and `docker-compose.yaml` files:

```bash
# Build the WASM plugin
cargo build --target wasm32-wasip1 --release

# Start Envoy
docker-compose up

# Test
curl http://localhost:10000/test
```

## References

- [Envoy Patch Policy | Envoy Gateway](https://gateway.envoyproxy.io/docs/tasks/extensibility/envoy-patch-policy/)
- [WASM Extensions | Envoy Gateway](https://gateway.envoyproxy.io/docs/tasks/extensibility/wasm/)
- [How to configure extra cluster - GitHub Discussion #5301](https://github.com/envoyproxy/gateway/discussions/5301)
