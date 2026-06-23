# Stage 1: Build the Frontend (Yew WebAssembly)
FROM rust:1.96-slim as frontend-builder
WORKDIR /app

# Install compilation dependencies and Trunk binary
RUN apt-get update && apt-get install -y --no-install-recommends \
    wget pkg-config libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown
RUN wget -qO- "https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-x86_64-unknown-linux-gnu.tar.gz" | tar -xzf- -C /usr/local/bin

COPY Cargo.toml Cargo.lock ./
COPY backend/ ./backend/
COPY frontend/ ./frontend/
WORKDIR /app/frontend
RUN trunk build --release

# Stage 2: Build the Backend
FROM rust:1.96-slim as backend-builder
WORKDIR /app

# Install dependencies required to build native packages like openssl-sys
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev gcc g++ make && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY backend/ ./backend/
COPY frontend/ ./frontend/
# We only compile the backend binary here
RUN cargo build --release --bin backend

# Stage 3: Final package
FROM debian:bookworm-slim
LABEL org.opencontainers.image.source="https://github.com/UberMetroid/RustDrop"
WORKDIR /app

# Install runtime dependencies (SSL certificates for HTTPS notification requests)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

ENV PORT=4401
ENV NODE_ENV=production

COPY --from=backend-builder /app/target/release/backend ./rustdrop
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

RUN mkdir -p uploads data && chown -R 99:100 /app

# Run as Unraid nobody:users
USER 99:100

EXPOSE 4401

CMD ["./rustdrop"]
