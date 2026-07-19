# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.12] - 2026-07-19

### Fixed
- **Warning fix**: Removed unused mutable keyword on server command spawn to prevent warning compilation failures in CI runners.


## [1.0.11] - 2026-07-19

### Changed
- **Release build bump**: Preparing new version release to trigger automated package container compilation on GHCR.


## [1.0.10] - 2026-07-19

### Changed
- **Slim Branding Banner**: Replaced the repository header banner with a slim, flat-art twilight landscape of Cheney, WA (home of the server) featuring rolling hills, Ponderosa pines, and a soaring neon eagle.


## [1.0.9] - 2026-07-19

### Changed
- **Containerized Admin Console integration**: Named the admin tool after the application (`beam`) and copied it to the container's system path `/usr/local/bin/beam`. The TUI can now be launched by simply executing `beam tui` (or `beam`) inside the container shell.
- **Documentation Modernization**: Rewrote `README.md` to remove CI details, format CLI commands as tables, and purge local development guides.


## [1.0.8] - 2026-07-19

### Changed
- **Containerized Admin Console integration**: Named the admin tool after the application (`beam`) and copied it to the container's system path `/usr/local/bin/beam`. The TUI can now be launched by simply executing `beam tui` (or `beam`) inside the container shell.
- **Documentation Modernization**: Rewrote `README.md` to remove CI details, format CLI commands as tables, and purge local development guides.


## [1.0.7] - 2026-07-19

### Changed
- Update README, clean file tree, and remove contributing/license files.


