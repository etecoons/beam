# Beam - High-Performance File Sharing

<p align="center">
  <img src="https://raw.githubusercontent.com/UberMetroid/beam/main/frontend/Assets/favicon.png?v=3.0.1" alt="Beam Logo" width="128" height="128">
</p>

Beam is a lightweight, self-hosted, and high-performance file sharing web application. It features a modern, drag-and-drop web interface for uploading files and folders while maintaining their directory structures. Built with a high-performance Rust (Axum/Tokio) backend and a WebAssembly (Yew) frontend.

---

## ⚡ Key Features

*   **Dynamic Themes**: Dynamic theme options.
*   **Access PIN Security**: Lock down the interface with an optional numerical PIN for absolute privacy.
*   **Internationalization**: Built-in multilingual translation selector support.
*   **Print Optimization**: Customized print stylesheet layout and print header action button.
*   **Performance First**: Tiny resource footprint, zero external JS engine dependencies, and rapid page load speeds.
*   **Drag-and-Drop Uploads**: Upload files and complete folder structures seamlessly while preserving directory layouts.
*   **Quota & Retention**: Configurable total storage limits and automatic age-based file purges.

---

## 📦 Container Registry

The Docker image is built with **Nix** (no Alpine, fully reproducible) and published to Docker Hub:

*   **Docker Hub**: [ubermetroid/beam](https://hub.docker.com/r/ubermetroid/beam)

The image can also be built locally from the Nix flake:

```bash
nix build .#dockerImage
docker load < result
```

---

## 🐳 Container Installation



1. Create a `docker-compose.yml` file:

```yaml
version: '3'
services:
  beam:
    image: ubermetroid/beam:latest
    container_name: beam
    restart: unless-stopped
    ports:
      - 4401:4401
    volumes:
      - ./data:/app/data
      - ./uploads:/app/uploads
    environment:
      - PORT=4401
      - SITE_TITLE=Beam
      - BASE_URL=http://localhost:4401
      - ALLOWED_ORIGINS=*
      - BEAM_PIN=1234
      - TZ=UTC
      - MAX_FILE_SIZE=20
      - MAX_STORAGE_LIMIT_GB=100
      - RETENTION_PERIOD_DAYS=30
      - ALLOWED_EXTENSIONS=
      - ENABLE_TRANSLATION=false
      - ENABLE_THEMES=true
      - ENABLE_PRINT=false
```

2. Run the container:

```bash
docker compose up -d
```

3. Open your browser and navigate to `http://localhost:4401`.

### Building the Image Locally

To build the Docker container locally from the source files using Nix:

```bash
nix build .#dockerImage
docker load < result
docker tag beam-nix:latest ubermetroid/beam:latest
```

The image is Nix-built (no Alpine, no Docker daemon dependency for the build).
For development iteration, use the devShell:

```bash
nix develop
```

## 📋 Configuration Options

Configure these settings inside your Docker Compose environment or container environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | The port number the backend HTTP server will bind to inside the container. | `4401` |
| `SITE_TITLE` | Custom website title rendered in navigation headers, browser tabs, and PWA manifest. *(Supports fallbacks `BEAM_SITE_TITLE` or `BEAM_TITLE`)* | `Beam` |
| `BASE_URL` | Application base URL. Essential when deploying behind reverse proxies to ensure redirect and websocket links are resolved correctly. | `http://localhost:4401/` |
| `ALLOWED_ORIGINS` | Comma-separated list of allowed HTTP request origins (CORS filter). Use `*` to allow all origins. | `*` |
| `BEAM_PIN` | Optional 4–64 character PIN (any characters) to lock access to the interface. Leave empty for public mode. | None |
| `TZ` | Timezone for the container processes and logs. | `UTC` |
| `UPLOAD_DIR` | Main directory path where uploaded files are stored. | `/app/uploads` |
| `MAX_FILE_SIZE` | Maximum file size limit in MB. | `1024` (1GB) |
| `AUTO_UPLOAD` | Start uploading immediately upon dragging files. | `false` |
| `SHOW_FILE_LIST` | Enable file explorer listing/deletion interface. | `false` |
| `TRUST_PROXY` | Set `true` if backend is hosted behind a reverse proxy. | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated IP/CIDR list of trusted upstream proxies. | None |
| `MAX_STORAGE_LIMIT_GB` | Maximum capacity limit for upload directory in GB. | None |
| `RETENTION_PERIOD_DAYS` | Automatically delete files older than this many days. | None |
| `ALLOWED_EXTENSIONS` | Comma-separated list of allowed extensions (e.g. `.png,.pdf`). | None (All) |
| `ENABLE_TRANSLATION` | Enable the multi-language / translation selector in the navigation header (true/false). | `false` |
| `ENABLE_THEMES` | Enable the Super Metroid theme selector in the navigation header (true/false). | `true` |
| `ENABLE_PRINT` | Enable the print button in the navigation header (true/false). | `false` |
| `MAX_ATTEMPTS` | Number of failed PIN attempts permitted before locking out the user client IP address. | `5` |
| `LOCKOUT_TIME_MINUTES` | Lockout duration in minutes for IPs exceeding `MAX_ATTEMPTS`. | `15` |
| `COOKIE_MAX_AGE_HOURS` | Duration in hours that the user's PIN session cookie remains valid. | `24` |
| `SHUTDOWN_DRAIN_SECONDS` | Seconds to wait for active connections to finish before shutting down. | `5` |
| `SHOW_VERSION` | Display the application version number in the footer (true/false). | `true` |
| `SHOW_GITHUB` | Display the GitHub repository link in the footer (true/false). | `true` |
| `CLIENT_MAX_RETRIES` | Number of connection retry attempts permitted for chunked file uploads. | `5` |



---

*Note: This repository was forked from [DumbDrop](https://github.com/DumbWareio/DumbDrop).*
