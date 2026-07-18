# personal-kanban

A personal kanban CLI. See [SPEC.md](SPEC.md) for the implementation spec.

## Development

The toolchain is pinned by the Nix flake devShell (Rust + clippy + rustfmt +
`just`). Enter it with direnv (`.envrc` runs `use flake`) or `nix develop`.

```
just            # list recipes
just validate   # full gate: fmt + lint + test + build (CI equivalent)
just test
just lint
```

Every tool the `Justfile` recipes call is provided by the devShell, so
validation never depends on host-installed tooling.
