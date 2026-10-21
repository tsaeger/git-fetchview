# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2026 Tom Saeger <tom.saeger@gmail.com>

{
  description = "git-fetchview: pinned toolchain and tools for development and CI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      # x86_64-darwin is not here: nixpkgs 26.11 dropped it.
      systems = [ "aarch64-darwin" "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system:
        f (import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; }));
    in
    {
      devShells = forAllSystems (pkgs: {
        # Everything `cargo xtask ci` and the changelog task need. The Rust
        # version comes from rust-toolchain.toml, so rustup and Nix agree.
        default = pkgs.mkShell {
          packages = [
            (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
            pkgs.cargo-deny
            pkgs.cargo-audit
            pkgs.git-cliff
            pkgs.git
            pkgs.nodejs # for backlog.md: `npm ci`, then `npx backlog`
          ];
        };

        # The minimum supported Rust version, to check that `rust-version` in
        # Cargo.toml is true. Keep the version in step with it.
        msrv = pkgs.mkShell {
          packages = [ pkgs.rust-bin.stable."1.88.0".minimal pkgs.git ];
        };
      });
    };
}
