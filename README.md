<h1 align="center">
  <img src="assets/icon.png?v=1.0.31" width="48" height="48" valign="middle"> Beam
</h1>

<p align="center">
  <b>High-performance, secure self-hosted file sharing web application written in Rust.</b>
</p>

---

### Instant One-Line Install (Docker Container)

Run the official zero-dependency container on port 4401:

```bash
docker run -d --name beam -p 4401:4401 -v /mnt/user/appdata/beam:/config ghcr.io/studio2201/beam:latest
```

Open your browser to `http://localhost:4401` to start uploading and sharing files immediately.

---

### Environment Configuration

The backend service can be customized using the following environment variables:

| Variable | Description | Default |
| :--- | :--- | :---: |
| `PORT` | Network port the web server binds to | `4401` |
| `BEAM_PIN` | Security PIN required for upload authentication | *(Disabled)* |
| `UPLOAD_DIR` | Directory path for persistent data and uploads | `/config` |
| `BEAM_ALLOWED_ORIGINS` | CORS allowed origins list (comma-separated) | `*` |
| `TRUST_PROXY` | Honor reverse proxy headers (`X-Forwarded-For`) | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated CIDR list of trusted reverse proxies | *(None)* |
| `LOG_LEVEL` | Tracing filter (`error`, `warn`, `info`, `debug`) | `info` |

---

### Administration CLI & TUI Dashboard

Every container and package includes a built-in administration utility (`beam`).

Launch interactive TUI dashboard:
```bash
docker exec -it beam beam tui
```

System diagnostics and self-healing check:
```bash
docker exec -it beam beam doctor
```

CLI Command Reference:
- `beam tui` — Interactive terminal user interface.
- `beam doctor` — Diagnoses storage permissions, ports, and database health.
- `beam status` — Displays network configuration and security parameters.
- `beam data stats` — Shows storage utilization and entry metrics.
- `beam data list` — Lists database entries and uploaded records.

---

### Architecture & Security

- **Axum Web Backend**: High-concurrency async streaming runtime built on Tokio.
- **Yew WebAssembly Frontend**: Type-safe client bundle running natively in browser WASM runtime.
- **Zero-Copy Chunked Uploads**: Direct-to-disk streaming pipeline bypassing heap allocations.
- **Strict Stored XSS Defense**: Enforces `Content-Disposition: attachment` and overrides dangerous mime-types to `application/octet-stream`.

---

### License

Distributed under the Apache 2.0 License. See [LICENSE](LICENSE) for details.

---

<p align="center">
  <a href="https://github.com/studio2201/beam">
    <img src="assets/corgi-footer.jpg" alt="studio2201 banner" width="100%">
  </a>
</p>
