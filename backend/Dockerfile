# Build the Luna API independently from the Leptos/WASM client.
FROM rust:1.86-slim AS builder
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.toml
COPY backend/src backend/src
RUN cargo build --release --manifest-path backend/Cargo.toml

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/backend/target/release/luna-server /usr/local/bin/luna-server
ENV PORT=3000
EXPOSE 3000
CMD ["luna-server"]
