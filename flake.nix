{
  description = "personal-kanban — a personal kanban board CLI and TUI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    crane,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
      lib = pkgs.lib;
      craneLib = (crane.mkLib pkgs).overrideToolchain (
        (pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "clippy" "rustfmt"];
        }).out
      );

      # Common sources
      src = ./.;

      # Darwin-specific build inputs
      darwinBuildInputs = lib.optionals pkgs.stdenv.isDarwin [
        pkgs.libiconv
      ];

      # Dev shell toolchain with rust-analyzer
      rustToolchain = (pkgs.rust-bin.stable.latest.default.override {
        extensions = ["rust-src" "clippy" "rustfmt" "rust-analyzer"];
      }).out;

      kanbanPackage = craneLib.buildPackage {
        inherit src;
        nativeBuildInputs = [pkgs.pkg-config];
        buildInputs = darwinBuildInputs;

        # Create the pk symlink in postInstall
        postInstall = ''
          ln -s $out/bin/kanban $out/bin/pk
        '';

        meta = {
          description = "A personal kanban board CLI and TUI";
          license = lib.licenses.mit;
          mainProgram = "kanban";
        };
      };
    in {
      packages.default = kanbanPackage;

      devShells.default = pkgs.mkShell {
        buildInputs = [
          rustToolchain
          (pkgs.cargo-watch.overrideAttrs (old: {
            cargoBuildNoDefaultFeatures = true;
          }))
          pkgs.pkg-config
          pkgs.sqlite
          pkgs.just
          pkgs.alejandra
          pkgs.statix
          pkgs.cargo-tarpaulin
        ] ++ darwinBuildInputs;
      };

      formatter = pkgs.alejandra;
    });
}
