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

## Synthetic Test Workflow

Phase 7 standardizes ROM-free synthetic coverage around the shared helpers in `tests/support`.

- `tests/support/runtime_script.rs` drives generated-cartridge runtime sessions, frame stepping, and scripted input.
- `tests/support/assertions.rs` provides compact framebuffer and audio mismatch summaries.
- `tests/support/save_state.rs` creates temp-path ROM and slot fixtures for future persistence tests.

When extending milestone-critical synthetic coverage, prefer those helpers over ad hoc temp-path, framebuffer, or audio fixture code.

Quick loop for runtime and video-heavy synthetic work:

```bash
cargo test --test ppu_timing --test smb_video --test runtime_session --test runtime_view
```

Phase 7 plan slices also use these focused commands while implementing specific areas:

```bash
cargo test --test runtime_session --test runtime_view --test shell_loader
cargo test --test cpu_vectors --test ppu_timing --test ppu_background --test ppu_sprite0 --test smb_video --test apu_audio
cargo test --test runtime_input --test runtime_controls --test runtime_session --test runtime_view --test audio_runtime --test shell_loader --test save_state --test desktop_product
```

Full synthetic gate:

```bash
cargo nextest run --all-targets --all-features
```

Committed tests must stay ROM-free. When a test needs a path-based seam, write generated ROM bytes to a temp file through `tests/support` helpers instead of reading from local `roms/` or `nestest/` directories.

## Release Smoke Workflow

Phase 5 desktop-finish work adds a repeatable release smoke path for the desktop product surface.

Run the full release smoke workflow from the repo root with:

```bash
sh scripts/smoke-release-desktop.sh
```

The script wraps these exact commands:

```bash
sh scripts/check-no-real-rom-tests.sh
cargo build --release
cargo test --release --test desktop_product --test runtime_view --test shell_diagnostics --test trace_cli
```

This workflow stays ROM-free and headless-friendly: it uses generated-ROM tests, shell diagnostics, and the release trace CLI suite instead of checked-in game assets or manual GUI launching.

## CI

The workflow in `.github/workflows/ci.yml` mirrors the local checks and installs the Cargo helper binaries with pinned GitHub Actions.

## License Policy Note

`cargo-deny` is configured to ignore licensing for unpublished private workspace crates. The manifest currently sets `publish = false`, which keeps the policy green until the project chooses and records a repository license.
