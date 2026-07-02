# Changelog

All notable changes to `shared-assets` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.1] - 2026-06-28

### Added

- **`shared_core::types`** — new module with wire-format / on-disk data
  types shared between the Yew frontend and the axum backend:
  - `TodoItem` — single todo record (`id`, `text`, `completed`)
  - `TodoLists` — `HashMap<list_name, Vec<TodoItem>>` (alias)
  - `SiteConfig` — `GET /api/config` response (`siteTitle`, `singleList`,
    `enableThemes`, `enablePrint`, `showVersion`, `showGithub`,
    camelCase JSON)
  - `PinRequiredResponse` — `GET /api/pin-required` response
  - `VerifyPinRequest` / `VerifyPinResponse` — `POST /api/verify-pin`
    request and (camelCase) response, with `Option`-typed fields that
    serialize only on failure
  - 4 unit tests covering round-trip JSON and default-value behavior
- 72 unit tests across the workspace (was 68)

### Why a 3.0.1 (not 3.1.0)

The new module is purely additive: no breaking changes to the public API
shipped in v3.0.0, no `Cargo.toml` changes required for consumers that
don't import the new types. Apps that previously vendored these types in
their own crate (notably `todo`) can now depend on `shared-core` and
delete their copies.

## [3.0.0] - 2026-06-26

### ⚠ BREAKING CHANGES

This release splits the previous single `shared-assets` crate into a
3-crate Cargo workspace. Consumers must update their `Cargo.toml` to
depend on each of the three crates and update `use` paths accordingly.

#### Migration guide

**Before (2.x):**
```rust
use shared_assets::print_unauthorized_console_message;
use shared_assets::header::Header;

shared_assets::print_unauthorized_console_message();
```

**After (3.x):**
```rust
use shared_backend::security::print_unauthorized_console_message;
use shared_frontend::components::Header;

shared_backend::security::print_unauthorized_console_message();
```

**Cargo dependency:**
```toml
# Before (2.x):
shared-assets = { path = "...", features = ["frontend"] }

# After (3.x): three crates, pinned to tag v3.0.0:
shared-core    = { git = "https://github.com/UberMetroid/shared-assets", tag = "v3.0.0" }
shared-backend = { git = "https://github.com/UberMetroid/shared-assets", tag = "v3.0.0" }
shared-frontend = { git = "https://github.com/UberMetroid/shared-assets", tag = "v3.0.0" }

# Or for local development:
shared-core    = { path = "Assets/shared-assets/shared-rust/shared-core" }
shared-backend = { path = "Assets/shared-assets/shared-rust/shared-backend" }
shared-frontend = { path = "Assets/shared-assets/shared-rust/shared-frontend" }
```

#### New modules

- **`server`** (in `shared-backend`) — Backend server primitives
  - `ServerConfig` — common env-driven config (port, pin, attempts, cookie age, CORS, enable_*, show_*, trust_proxy)
  - `server::serve` — bind + graceful shutdown on SIGINT/SIGTERM
  - `server::ServerError` — `IntoResponse` error type with HTTP status mapping
  - `server::ip::get_client_ip` — trusted-proxy-aware client IP extraction
  - `server::version::CARGO_PKG_VERSION` — re-export of the consuming crate's version

- **`auth`** (in `shared-backend`) — PIN authentication
  - `auth::pin_auth_layer` — axum middleware that gates routes behind a PIN
  - `auth::attempts::{is_locked_out, record_attempt, reset_attempts, lockout_remaining_secs}`
  - `auth::session::issue_cookie` — session cookie helpers

- **`middleware`** (in `shared-backend`) — Shared axum middleware factories
  - `middleware::cors_layer` — CORS layer from `ALLOWED_ORIGINS`
  - `middleware::security_headers_layer` — CSP, X-Frame-Options, etc.
  - `middleware::title_injection_layer` — `{{SITE_TITLE}}` → config
  - `middleware::hsts_layer` — HSTS when HTTPS

#### Removed

- The 2.x single-crate layout. There is no longer a `shared-assets`
  crate; the repository now ships the three crates `shared-core`,
  `shared-backend`, `shared-frontend`. There is no top-level
  `print_unauthorized_console_message` re-export — use
  `shared_backend::security::print_unauthorized_console_message`.

#### Changed

- Bumped edition 2021 → 2024 (let-chains used throughout)
- `web-sys` pinned to `=0.3.98` (matches the Yew 0.23 expected version)
- `ipnet`, `tokio`, `tower-http`, `axum`, `thiserror`, `anyhow`, `dotenvy`,
  `constant_time_eq`, `tracing`, `http-body-util` are now direct dependencies
  of `shared-backend` (consumers don't need to declare them just to use
  the new shared modules)
- `Header` prop API: `disable_print` + `enable_print` collapsed into a
  single `print_disabled: bool`; `on_print` is now `Option<Callback<…>>`

### Added

- 68 unit tests (was 22)
- `cargo clippy` clean, `cargo fmt` clean
- 27 `.rs` files all ≤ 250 lines

## [2.1.1] - 2026-06-25

Last 2.x release. Provides Yew components, theme management, and i18n only.

- `components::Header`, `components::Footer` — Yew UI chrome
- `theme::Theme` — Super Metroid theme enum (Crateria, Brinstar, Norfair, WreckedShip, Maridia, Tourian)
- `theme::mapping::Scheme` — User-facing scheme (light/sepia/dracula/nord) → Theme mapping
- `i18n::Language` — 8-language enum (en/zh/es/de/ja/fr/pt/ru)
- `i18n::strings::lookup` — Centralized UI string lookup
- `security::print_unauthorized_console_message` — anti-shell alert (also re-exported at crate root for 2.x compat)