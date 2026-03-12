# ── Stage 1: Build Vue admin panel ───────────────────────────────────────────
FROM oven/bun:1 AS frontend

ARG VITE_DEFAULT_DOMAIN
ENV VITE_DEFAULT_DOMAIN=$VITE_DEFAULT_DOMAIN

WORKDIR /app/admin
COPY admin/package.json admin/bunfig.toml ./
RUN bun install

COPY admin/ ./
RUN bun run build-only


# ── Stage 2: Build Rust proxy ─────────────────────────────────────────────────
FROM rust:1-slim AS backend

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app/proxy
COPY proxy/Cargo.toml proxy/Cargo.lock ./
# Cache deps by building a dummy main first
RUN mkdir src && echo 'fn main(){}' > src/main.rs && cargo build --release && rm -rf src

COPY proxy/src ./src
COPY proxy/migrations ./migrations
# Touch main.rs so cargo rebuilds it
RUN touch src/main.rs && cargo build --release


# ── Stage 3: Runtime image ────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Binary
COPY --from=backend /app/proxy/target/release/api-key-fy ./api-key-fy

# DB migrations
COPY --from=backend /app/proxy/migrations ./migrations

# Built admin SPA — served as static files by the proxy
COPY --from=frontend /app/admin/dist ./static

EXPOSE 8800

CMD ["./api-key-fy"]
