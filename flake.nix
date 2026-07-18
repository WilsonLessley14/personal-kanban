{
  description = "personal-kanban — TODO one-line description";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {inherit system overlays;};
      # The devShell toolchain. clippy + rustfmt are included so `just lint`
      # and `just format` work with no `rustup` in sight — this is the whole
      # point of pinning the toolchain in the flake rather than the host.
      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = ["rust-src" "clippy" "rustfmt"];
      };
    in {
      # `nix build` requires Cargo.lock (run `cargo build` once in the devShell
      # to generate it, then commit it).
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "personal-kanban";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        # Add native build deps here (pkg-config + libs your crates link).
        nativeBuildInputs = [pkgs.pkg-config];
        buildInputs = [];

        meta = {
          description = "TODO";
          license = pkgs.lib.licenses.mit;
          mainProgram = "personal-kanban";
        };
      };

      devShells.default = pkgs.mkShell {
        # Everything the Justfile recipes call must be listed here. If a
        # validation command fails with "command not found", the fix is to add
        # the package to this list — never to `rustup component add` at runtime.
        buildInputs = [
          rustToolchain
          pkgs.just
          pkgs.alejandra # nix formatter
          pkgs.statix # nix linter
          pkgs.cargo-tarpaulin # coverage (optional; used by `just coverage`)
          pkgs.pkg-config
        ];
      };

      formatter = pkgs.alejandra;
    });
}
