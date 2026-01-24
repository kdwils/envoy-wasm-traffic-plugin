FROM rust:1.93.0 AS builder

RUN rustup target add wasm32-wasip1

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --target wasm32-wasip1 --release

FROM scratch

COPY --from=builder /build/target/wasm32-wasip1/release/envoy_wasm_traffic_plugin.wasm /plugin.wasm
