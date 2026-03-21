# Contributing

## Branch Strategy

- `main` is the release-ready branch.
- `develop` is the integration branch for completed work.
- Create topic branches from `develop` using focused names such as `feat/...`, `fix/...`, or `chore/...`.
- Merge changes through pull requests. Do not push directly to `main` or `develop`.

## Pull Requests

- Keep each pull request scoped to a single objective.
- Use the pull request template and fill in `Why`, `What Changed`, `Validation`, `Risks`, and `Follow-ups`.
- Prefer conventional commit style for the pull request title when it accurately describes the change.
- Avoid unrelated refactors, formatting churn, or dependency changes unless they are part of the stated objective.

## Validation

Run the same checks locally before opening or updating a pull request:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-targets --all-features
cargo deny check
cargo machete
```

If behavior changes, add or update tests in the same pull request.

## Repository Hygiene

- Keep generated files, local editor settings, and temporary artifacts out of version control.
- Do not commit planning-only or local AI-assistance files.
- Update documentation when workflows, developer setup, or emulator behavior changes.

## Commit Quality

- Make small, reviewable commits with clear intent.
- Prefer root-cause fixes over narrow patches.
- Preserve the existing code style and project structure unless the change requires otherwise.