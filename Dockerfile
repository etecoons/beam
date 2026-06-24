# Stage 1: Build the Frontend (Yew WebAssembly)
FROM rust:1.96-alpine as frontend-builder
RUN apk add --no-cache musl-dev wget tar
WORKDIR /app

# Install wasm32 target
RUN rustup target add wasm32-unknown-unknown
RUN wget -qO- "https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-x86_64-unknown-linux-musl.tar.gz" | tar -xzf- -C /usr/local/bin

COPY Cargo.toml Cargo.lock ./
COPY backend/ ./backend/
COPY frontend/ ./frontend/
WORKDIR /app/frontend
RUN trunk build --release

# Stage 2: Build the Backend
FROM rust:1.96-alpine as backend-builder
RUN apk add --no-cache musl-dev
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY backend/ ./backend/
COPY frontend/ ./frontend/
# Compile backend binary
RUN cargo build --release --bin backend

# Stage 3: Final package
FROM alpine:latest
LABEL org.opencontainers.image.source="https://github.com/UberMetroid/RustDrop"
WORKDIR /app

# Install runtime dependencies (ca-certificates for HTTPS/SSL, wget for health checks)
RUN apk add --no-cache ca-certificates wget libc6-compat

ENV PORT=4401
ENV NODE_ENV=production

COPY --from=backend-builder /app/target/release/backend ./rustdrop
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

RUN mkdir -p uploads data && chown -R 99:100 /app

# Run as Unraid nobody:users
USER 99:100

EXPOSE 4401

CMD ["./rustdrop"]
