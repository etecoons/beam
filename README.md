# RustDrop

<p align="center">
  <img src="https://img.shields.io/github/v/tag/UberMetroid/RustDrop?label=version" alt="GitHub tag" />
  <img src="https://img.shields.io/badge/license-GPL--3.0-blue.svg" alt="License" />
  <img src="https://img.shields.io/github/actions/workflow/status/UberMetroid/RustDrop/docker-publish.yml" alt="GitHub Actions Workflow Status" />
</p>

---

## Overview

RustDrop is a lightweight, self-hosted, and high-performance file sharing web application. It features a modern, drag-and-drop web interface for uploading files and folders while maintaining their directory structures. The application is built from the ground up to be safe, reliable, and highly resource-efficient by combining an Axum/Tokio Rust backend with a Yew/WebAssembly frontend.

---

## Features

*   🚀 **Drag and Drop**: Recursive folder and file uploading while maintaining original directory structures.
*   🎨 **Minimalist Design**: Zero-tracker interface with light, dark, sepia, nord, and dracula theme synchronization.
*   🔒 **Optional PIN Security**: Lock down uploads behind a 4-to-10 digit PIN.
*   🛡️ **Built-in Protection**: Constant-time PIN comparisons (`constant_time_eq`) to prevent timing attacks, and automatic IP brute-force lockout.
*   🌐 **Reverse Proxy Aware**: Securely parses and normalizes client IPs behind Nginx, Cloudflare, etc., via proxy trust settings.
*   📦 **Flexible Storage Limits**: Enforce maximum storage ceilings (GB) and retention rules (prune files older than X days).
*   🔔 **Instant notifications**: Connect uploads to 100+ services (Discord, Telegram, Slack, etc.) using Apprise.

---

## Prerequisites & Environment Variables

### System Prerequisites
*   **Compilation Environment**: Rust Toolchain 1.80+ installed.
*   **WebAssembly Target**: `wasm32-unknown-unknown` target.
*   **Frontend Bundler**: `trunk` binary installed.

### Environment Variables

Configure these settings inside a `.env` file at the project root or inject them directly into your runtime environment:

| Variable | Description | Default | Status |
| :--- | :--- | :--- | :--- |
| `PORT` | Port the web server listens on. | `4401` | Optional |
| `BASE_URL` | Application base URL (must end with a `/`). | `http://localhost:4401/` | Optional |
| `UPLOAD_DIR` | Main directory path where uploaded files are stored. | `./local_uploads` | Optional |
| `LOCAL_UPLOAD_DIR` | Fallback directory path if `UPLOAD_DIR` is empty or unset. | `./local_uploads` | Optional |
| `MAX_FILE_SIZE` | Maximum file size limit in MB. | `1024` (1GB) | Optional |
| `AUTO_UPLOAD` | Start uploading immediately upon dragging files. | `false` | Optional |
| `SHOW_FILE_LIST` | Enable file explorer listing/deletion interface. | `false` | Optional |
| `RUSTDROP_PIN` | 4-10 digit PIN (numerical only) for upload protection. | None | Optional |
| `PIN` | Alias for `RUSTDROP_PIN`. | None | Optional |
| `RUSTDROP_TITLE` | Site title shown in headers and browser tab. | `RustDrop` | Optional |
| `SITE_TITLE` | Alias for `RUSTDROP_TITLE`. | `RustDrop` | Optional |
| `TRUST_PROXY` | Set `true` if backend is hosted behind a reverse proxy. | `false` | Optional |
| `TRUSTED_PROXY_IPS` | Comma-separated IP list of trusted upstream proxies. | None | Optional |
| `MAX_STORAGE_LIMIT_GB` | Maximum capacity limit for upload directory in GB. | None | Optional |
| `RETENTION_PERIOD_DAYS` | Automatically delete files older than this many days. | None | Optional |
| `APPRISE_URL` | Webhook URL for Apprise alerts (e.g. `discord://webhookid/token`). | None | Optional |
| `APPRISE_MESSAGE` | Alert message template. Supports `{filename}`, `{size}`, `{storage}`. | See below | Optional |
| `APPRISE_SIZE_UNIT` | Size format unit for notifications (B, KB, MB, GB, TB, or Auto). | `Auto` | Optional |
| `ALLOWED_EXTENSIONS` | Comma-separated list of allowed file extensions (e.g. `.png,.pdf`). | None (All) | Optional |
| `CLIENT_MAX_RETRIES` | Max network retry attempts client-side before failing an upload. | `5` | Optional |
 
> **Note**: Default Apprise message is: `"New file uploaded - {filename} ({size}), Storage used {storage}"`.
 
---
 
## Quick Start
 
Spin up your environment locally using one of the two execution paths below:
 
### Path A: Local Development (Build from Source)
This method is recommended for customizing code or testing locally:
 
```bash
# 1. Add WASM target and install Trunk
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
 
# 2. Configure environment values
cp .env.example .env
 
# 3. Build & bundle the WebAssembly frontend
cd frontend
trunk build --release
cd ..
 
# 4. Compile and start the backend HTTP server
cargo run --release --bin backend
```
 
Access the app at [http://localhost:4401](http://localhost:4401).
 
### Path B: Production Container (Docker Run)
Run RustDrop immediately without configuring compilation toolchains:
 
```bash
docker run -d \
  -p 4401:4401 \
  -v ./uploads:/app/uploads \
  -e RUSTDROP_PIN=123456 \
  -e SHOW_FILE_LIST=true \
  ubermetroid/rustdrop:latest
```
 
---
 
## Docker & Docker Compose Configurations
 
### Docker Compose Setup
For a robust, persistent service layout, create a `docker-compose.yml` file:
 
```yaml
services:
  rustdrop:
    image: ubermetroid/rustdrop:latest
    container_name: rustdrop
    restart: unless-stopped
    ports:
      - 4401:4401
    volumes:
      - ./uploads:/app/uploads
    environment:
      UPLOAD_DIR: /app/uploads
      BASE_URL: http://localhost:4401/
      RUSTDROP_TITLE: RustDrop
      MAX_FILE_SIZE: 1024
      RUSTDROP_PIN: 123456
      AUTO_UPLOAD: "true"
      SHOW_FILE_LIST: "true"
```

Start the service container:
```bash
docker compose up -d
```

# Build the multi-stage, production-ready container yourself:
docker build -t rustdrop:local .
```

### Nix Layered Container Building (Alternative)

For maximum isolation, reproducibility, and minimal footprints (no terminal tools, no shell, running strictly as `USER nobody`), you can compile and package the server using the provided Nix flake:

```bash
# 1. Build the layered Docker image tarball via Nix flake
nix build .#dockerImage

# 2. Load the resulting tarball image directly into Docker
docker load < result

# 3. Execute the Nix-built container
docker run -d \
  --name rustdrop-nix \
  -p 4401:4401 \
  -v ./uploads:/app/uploads \
  -e RUSTDROP_PIN=123456 \
  rustdrop-nix:latest
```

---

## Technical Details

RustDrop separates concerns into two distinct workspace packages:

```
                  ┌──────────────────────┐
                  │ Yew WASM Frontend    │
                  │ (Compiles to WASM)   │
                  └──────────┬───────────┘
                             │
                  HTTP POST  │ (Multipart API calls)
                             ▼
                  ┌──────────────────────┐
                  │ Axum / Tokio Backend │
                  │ (File upload / Auth) │
                  └──────────┬───────────┘
                             │
                             ▼
                  ┌──────────────────────┐
                  │ Local Filesystem     │
                  │ (/app/uploads)       │
                  └──────────────────────┘
```

*   **Backend Architecture (Axum + Tokio)**:
    *   **HTTP Routing**: Handled using `axum::Router`. Routes are nested logically under `/api/auth`, `/api/upload`, and `/api/files`.
    *   **File Streaming**: Multithreaded streams write incoming multipart byte arrays into chunk structures.
    *   **Security layer**: PIN validations are verified using timing-attack safe comparisons (`constant_time_eq`). Lockout attempt tables are cached in a thread-safe mutex container.
    *   **Cleanup Service**: Background tokio tasks periodically remove incomplete chunks and handle retention expiration.
*   **Frontend Architecture (Yew + WebAssembly)**:
    *   **State Loop**: Written as a pure single-page client app. Messages drive state changes asynchronously.
    *   **Async Requests**: Interacts with the backend via `gloo-net` request tasks wrapped under `wasm-bindgen-futures`.
    *   **Tree Resolution**: Directory dragging utilizes JavaScript interop bindings to recursively parse file metadata before triggering uploads.
    *   **Localization (i18n)**: Features a custom, type-safe localization module supporting English, Chinese (Simplified), Spanish, German, Japanese, French, Portuguese, and Russian.

---

## File Tree

The workspace is organized into separate crates for frontend and backend:

```text
RustDrop/
├── Cargo.lock
├── Cargo.toml          # Workspace manifest
├── Dockerfile          # Multi-stage container instructions
├── docker-compose.yml  # Docker Compose config file
├── README.md           # Main documentation
├── LOCAL_DEVELOPMENT.md# Local setup and notification guide
├── backend/            # Backend Crate (Axum API)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs     # Server entrypoint and HSTS binding
│       ├── config.rs   # Configuration parsing and defaults
│       ├── security.rs # Brute force prevention and safe compares
│       ├── utils.rs    # File and storage helpers
│       ├── services.rs # Native Apprise alert notifications
│       ├── routes/     # Routing controllers (auth, files, upload)
│       └── tests.rs    # Backend unit test suite
└── frontend/           # Frontend Crate (Yew Application)
    ├── Cargo.toml
    ├── index.html      # Trunk template HTML index
    ├── Assets/         # Static assets and stylesheets
    │   ├── app.css     # Drag zone and upload queue transitions
    │   ├── base.css    # Styling variables and base layout resets
    │   ├── header.css  # Navigation header layout styling
    │   ├── login.css   # Card wrapper layout styling
    │   ├── assets/     # SVG and PNG icons
    │   └── service-worker.js # PWA offline service worker
    └── src/
        ├── main.rs     # Application entry mount point
        ├── api.rs      # HTTP fetch implementation
        ├── header.rs   # Shared navigation header component
        ├── i18n.rs     # Localization module and translation dictionaries
        ├── js_api.rs   # JavaScript drag-drop hooks
        ├── storage.rs  # LocalStorage abstractions
        ├── types.rs    # Message and config structures
        ├── utils.rs    # Formatting utilities
        └── app/        # Routing, update loop, and view layers
```

---

## Testing & Linting

Enforce quality and reproducibility before committing codebase updates:

```bash
# Check code formatting matches standard rules
cargo fmt --all -- --check

# Format files in-place
cargo fmt

# Run Clippy static analysis with warnings denied
cargo clippy --workspace --all-targets -- -D warnings

# Execute workspace unit and integration tests
cargo test --workspace
```

---

## Contributing

1.  Fork the repository and create your feature branch: `git checkout -b feature/your-feature-name`.
2.  Follow coding standards. Ensure `cargo fmt` and `cargo clippy` pass successfully without warnings.
3.  Commit your updates using the Conventional Commits style.
4.  Push changes to your fork and submit a Pull Request.

---

## License

Distributed under the **GPL-3.0 License**. See [LICENSE](file:///LICENSE) for more information.
