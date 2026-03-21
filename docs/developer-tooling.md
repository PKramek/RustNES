# Developer Tooling

This repository uses a two-stage local hook pipeline and a matching GitHub Actions workflow.

## Included Tools

- `rustfmt` for formatting.
- `clippy` for linting with warnings treated as errors.
- `cargo-machete` for unused direct dependency checks.
- `cargo-nextest` for fast isolated test execution.
- `cargo-deny` for advisories, licensing, crate bans, and source validation.
- `pre-commit` to orchestrate fast checks on commit and heavier checks on push.

## Local Bootstrap

1. Ensure the stable Rust toolchain is installed. The repository pins `clippy` and `rustfmt` in `rust-toolchain.toml`.
2. Install the Cargo subcommands:

   ```bash
   cargo install --locked cargo-nextest@0.9.132 cargo-deny@0.19.0 cargo-machete@0.9.1
   ```

3. Install the Git hooks:

   ```bash
   pre-commit install --hook-type pre-commit --hook-type pre-push
   ```

## Hook Stages

- `pre-commit`: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo machete`
- `pre-push`: `cargo nextest run --all-targets --all-features`, `cargo deny check`

## Manual Validation

Run the same checks without waiting for Git hooks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo machete
cargo nextest run --all-targets --all-features
cargo deny check
```

To run the full hook pipeline directly:

```bash
pre-commit run --all-files
pre-commit run --hook-stage pre-push --all-files
```

## CI

The workflow in `.github/workflows/ci.yml` mirrors the local checks and installs the Cargo helper binaries with pinned GitHub Actions.

## License Policy Note

`cargo-deny` is configured to ignore licensing for unpublished private workspace crates. The manifest currently sets `publish = false`, which keeps the policy green until the project chooses and records a repository license.