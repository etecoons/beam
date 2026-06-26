{
  description = "Minimalist Nix-built container for Beam";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    shared-assets = {
      url = "github:UberMetroid/shared-assets";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, shared-assets, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustVersion = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
        rustPlatform = pkgs.makeRustPlatform {
          rustc = rustVersion;
          cargo = rustVersion;
        };

        # 1. Build the WASM frontend
        frontend = rustPlatform.buildRustPackage {
          pname = "beam-frontend";
          version = "2.1.1";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [
            rustVersion
            pkgs.wasm-bindgen-cli
            pkgs.trunk
          ];

          buildPhase = ''
            export HOME=$TMPDIR
            mkdir -p frontend/Assets/shared-assets
            cp -r ${shared-assets}/* frontend/Assets/shared-assets/
            cd frontend
            trunk build --release
          '';

          installPhase = ''
            mkdir -p $out/dist
            cp -r dist/* $out/dist/
          '';
        };

        # 2. Build the Axum backend
        backend = rustPlatform.buildRustPackage {
          pname = "beam-backend";
          version = "2.1.1";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];

          doCheck = false;

          buildPhase = ''
            mkdir -p frontend/Assets/shared-assets
            cp -r ${shared-assets}/* frontend/Assets/shared-assets/
            cargo build --release --bin backend --bin sh
          '';

          installPhase = ''
            mkdir -p $out/bin
            cp target/release/backend $out/bin/backend
            cp target/release/sh $out/bin/sh
          '';
        };

        # 3. Create the layered Docker container image
        dockerImage = pkgs.dockerTools.buildLayeredImage {
          name = "beam-nix";
          tag = "latest";
          
          # Run under the nobody user (UID 65534)
          config = {
            Cmd = [ "${backend}/bin/backend" ];
            WorkingDir = "/app";
            Env = [
              "PORT=4401"
              "UPLOAD_DIR=/app/uploads"
            ];
            ExposedPorts = {
              "4401/tcp" = {};
            };
            User = "65534:65534";
          };

          # Create /app directory structure inside the container
          extraCommands = ''
            mkdir -p app/data
            mkdir -p app/uploads
            mkdir -p app/frontend
            cp -r ${frontend}/dist app/frontend/dist
                      mkdir -p bin
            cp ${backend}/bin/sh bin/sh
            cp ${backend}/bin/sh bin/bash
'';
        };

      in {
        packages = {
          inherit frontend backend dockerImage;
          default = dockerImage;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustVersion
            pkgs.trunk
            pkgs.wasm-bindgen-cli
          ];
        };
      }
    );
}
