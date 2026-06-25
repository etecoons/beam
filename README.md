# Beam - High-Performance File Sharing

<p align="center">
  <img src="https://raw.githubusercontent.com/UberMetroid/Beam/main/frontend/Assets/assets/icon.png" alt="Beam Logo" width="128" height="128">
</p>

Beam is a lightweight, self-hosted, and high-performance file sharing web application. It features a modern, drag-and-drop web interface for uploading files and folders while maintaining their directory structures, built with a Rust (Axum/Tokio) backend and a WebAssembly (Yew) frontend.

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
      - ./uploads:/app/uploads
    environment:
      - PORT=4401
      - UPLOAD_DIR=/app/uploads
      - BASE_URL=http://localhost:4401/
      - BEAM_TITLE=Beam
      - MAX_FILE_SIZE=1024
      - BEAM_PIN=123456
      - AUTO_UPLOAD=true
      - SHOW_FILE_LIST=true
```

2. Run the container:

```bash
docker compose up -d
```

3. Open your browser and navigate to `http://localhost:4401`.


---

## 📋 Configuration Options

Configure these settings inside your Docker Compose environment or container environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | The port number the backend HTTP server will bind to inside the container. | `4401` |
| `SITE_TITLE` | Custom website title rendered in navigation headers, browser tabs, and PWA manifest. *(Supports fallback `RUSTBEAM_TITLE`)* | `Beam` |
| `BASE_URL` | Application base URL. Essential when deploying behind reverse proxies to ensure redirect and websocket links are resolved correctly. | `http://localhost:4401/` |
| `ALLOWED_ORIGINS` | Comma-separated list of allowed HTTP request origins (CORS filter). Use `*` to allow all origins. | `*` |
| `BEAM_PIN` | Optional 4–10 digit PIN (numerical only) to lock access to the interface. Leave empty for public mode. | None |
| `TZ` | Timezone for the container processes and logs. | `UTC` |
| `UPLOAD_DIR` | Main directory path where uploaded files are stored. | `/app/uploads` |
| `MAX_FILE_SIZE` | Maximum file size limit in MB. | `1024` (1GB) |
| `AUTO_UPLOAD` | Start uploading immediately upon dragging files. | `false` |
| `SHOW_FILE_LIST` | Enable file explorer listing/deletion interface. | `false` |
| `TRUST_PROXY` | Set `true` if backend is hosted behind a reverse proxy. | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated IP list of trusted upstream proxies. | None |
| `MAX_STORAGE_LIMIT_GB` | Maximum capacity limit for upload directory in GB. | None |
| `RETENTION_PERIOD_DAYS` | Automatically delete files older than this many days. | None |
| `ALLOWED_EXTENSIONS` | Comma-separated list of allowed extensions (e.g. `.png,.pdf`). | None (All) |
| `ENABLE_TRANSLATION` | Enable the multi-language / translation selector in the navigation header (true/false). | `false` |
| `ENABLE_THEMES` | Enable the Super Metroid theme selector in the navigation header (true/false). | `true` |
| `ENABLE_PRINT` | Enable the print button in the navigation header (true/false). | `true` |
| `MAX_ATTEMPTS` | Number of failed PIN attempts permitted before locking out the user client IP address. | `5` |



---

*Note: This repository was forked from [DumbDrop](https://github.com/DumbWareio/DumbDrop).*
