# RustDrop

<p align="center">
  <img src="https://img.shields.io/github/v/release/UberMetroid/RustDrop?label=version" alt="GitHub release" />
  <img src="https://img.shields.io/badge/license-GPL--3.0-blue.svg" alt="License" />
  <img src="https://img.shields.io/github/actions/workflow/status/UberMetroid/RustDrop/docker-publish.yml" alt="GitHub Actions Workflow Status" />
</p>

A stupid simple file upload application that provides a clean, modern interface for dragging and dropping files. Built with Rust (Axum/Tokio backend and Yew/Trunk WebAssembly frontend).

![RustDrop](https://github.com/user-attachments/assets/1b909d26-9ead-4dc7-85bc-8bfda0d366c1)

---

## Features

- 🚀 **Drag and Drop**: Smooth drag-and-drop uploading for files and folders (maintaining directory structure).
- 🎨 **Minimalist Design**: Responsive interface with system theme sync (Light, Dark, Sepia, Nord, Dracula).
- 📂 **File Listing**: Option to list, download, and delete uploaded files right from the UI.
- 🔒 **PIN Security**: Lock down your uploader with an optional 4-10 digit PIN.
- 🛡️ **Security Built-in**: Constant-time PIN comparisons, brute-force IP lockout protection, and trusted reverse-proxy IP checks.
- 🔔 **Upload Notifications**: Receive instant notifications on Discord, Telegram, or other services via Apprise.

---

## Quick Start

### Docker (Recommended)

```bash
docker run -d -p 3000:3000 -v ./uploads:/app/uploads -e RUSTDROP_PIN=1234 ubermetroid/rustdrop:latest
```

1. Go to `http://localhost:3000`
2. Enter PIN `1234`
3. Drag & drop files to upload! They will save in `./uploads` on your host.

### Docker Compose

Create a `docker-compose.yml` file:

```yaml
services:
  rustdrop:
    image: ubermetroid/rustdrop:latest
    container_name: rustdrop
    restart: unless-stopped
    ports:
      - 3000:3000
    volumes:
      - ./uploads:/app/uploads
    environment:
      UPLOAD_DIR: /app/uploads
      BASE_URL: http://localhost:3000
      RUSTDROP_TITLE: RustDrop
      RUSTDROP_PIN: 123456 # Leave empty to disable auth
      MAX_FILE_SIZE: 1024 # In MB
      AUTO_UPLOAD: "true"
      SHOW_FILE_LIST: "true"
```

Start the container:
```bash
docker compose up -d
```

---

## Configuration

RustDrop can be configured via environment variables. The most common settings:

| Variable | Description | Default |
| --- | --- | --- |
| `PORT` | Port the web server listens on | `3000` |
| `BASE_URL` | Application base URL (must end with `/`) | `http://localhost:PORT/` |
| `RUSTDROP_PIN` | Optional 4-10 digit authentication PIN | None |
| `SHOW_FILE_LIST` | Enable file explorer listing/deletion | `false` |
| `AUTO_UPLOAD` | Start uploading immediately upon selection | `false` |
| `MAX_FILE_SIZE` | Maximum file size limit in MB | `1024` (1GB) |
| `ALLOWED_EXTENSIONS` | Comma-separated list of allowed extensions (e.g. `.png,.txt`) | All extensions |
| `TRUST_PROXY` | Set to `true` if behind Nginx, Cloudflare, etc. | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated trusted proxy IPs to prevent IP spoofing | None |
| `APPRISE_URL` | Apprise notification webhook URL | None |

> [!NOTE]
> For a full list of settings and advanced proxy configurations, see the [.env.example](file:///.env.example) template.

---

## Development

To build from source or run locally:

1. Check the [Local Development Guide](file:///LOCAL_DEVELOPMENT.md).
2. Configure settings using a `.env` file.

---

## Technical Details

- **Backend**: Rust (Axum + Tokio)
- **Frontend**: Rust (Yew + WebAssembly via Trunk)
- **Styling**: Vanilla CSS variables
- **Container**: Multi-stage lightweight Docker image

---

## Contributing & License

1. Fork the repo and create your feature branch.
2. Commit changes using Conventional Commits.
3. Open a Pull Request.

Distributed under the **GPL-3.0 License**. See [LICENSE](file:///LICENSE) for more information.
