# RustDrop - High-Performance File Sharing

RustDrop is a lightweight, self-hosted, and high-performance file sharing web application. It features a modern, drag-and-drop web interface for uploading files and folders while maintaining their directory structures, built with a Rust (Axum/Tokio) backend and a WebAssembly (Yew) frontend.

---

## 🐳 Container Installation

### Option 1: Docker Compose (Recommended)

1. Create a `docker-compose.yml` file:

```yaml
version: '3'
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
      - PORT=4401
      - UPLOAD_DIR=/app/uploads
      - BASE_URL=http://localhost:4401/
      - RUSTDROP_TITLE=RustDrop
      - MAX_FILE_SIZE=1024
      - RUSTDROP_PIN=123456
      - AUTO_UPLOAD=true
      - SHOW_FILE_LIST=true
```

2. Run the container:

```bash
docker compose up -d
```

3. Open your browser and navigate to `http://localhost:4401`.

### Option 2: Docker CLI

Run the following command to start the container:

```bash
docker run -d \
  --name rustdrop \
  --restart unless-stopped \
  -p 4401:4401 \
  -v $(pwd)/uploads:/app/uploads \
  -e RUSTDROP_PIN=123456 \
  -e SHOW_FILE_LIST=true \
  ubermetroid/rustdrop:latest
```

---

## 📋 Configuration Options

Configure these settings inside your Docker Compose environment or container environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | Port the web server listens on inside the container. | `4401` |
| `BASE_URL` | Application base URL. | `http://localhost:4401/` |
| `UPLOAD_DIR` | Main directory path where uploaded files are stored. | `/app/uploads` |
| `MAX_FILE_SIZE` | Maximum file size limit in MB. | `1024` (1GB) |
| `AUTO_UPLOAD` | Start uploading immediately upon dragging files. | `false` |
| `SHOW_FILE_LIST` | Enable file explorer listing/deletion interface. | `false` |
| `RUSTDROP_PIN` | 4-10 digit PIN (numerical only) for upload protection. | None |
| `RUSTDROP_TITLE` | Site title shown in headers and browser tab. | `RustDrop` |
| `TRUST_PROXY` | Set `true` if backend is hosted behind a reverse proxy. | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated IP list of trusted upstream proxies. | None |
| `MAX_STORAGE_LIMIT_GB` | Maximum capacity limit for upload directory in GB. | None |
| `RETENTION_PERIOD_DAYS` | Automatically delete files older than this many days. | None |
| `ALLOWED_EXTENSIONS` | Comma-separated list of allowed extensions (e.g. `.png,.pdf`). | None (All) |
