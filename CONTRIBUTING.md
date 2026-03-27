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
- Use feature-oriented language in public metadata. Avoid internal planning terms such as `phase`, `milestone`, or similar labels in branch names, pull request titles, and squash commit titles.
- Avoid unrelated refactors, formatting churn, or dependency changes unless they are part of the stated objective.

## Merge Strategy

- Merge short-lived topic branches into `develop` through pull requests.
- Prefer `squash merge` for topic branches so each pull request lands as one reviewable change on `develop`.
- Promote `develop` into `main` through a dedicated pull request.
- Prefer `rebase merge` for `develop` to `main` promotions so `main` stays aligned with `develop` without creating duplicate branch history.
- Avoid `squash merge` for `develop` to `main` promotions unless you intentionally want `main` and `develop` to diverge in commit history.

## Validation

Run the same checks locally before opening or updating a pull request:

```sh
sh scripts/check-no-real-rom-tests.sh
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
- Do not commit real ROM assets or tests that depend on checked-in or local third-party ROM files. Automated tests must use synthetic fixtures or generated ROM bytes instead.
- Update documentation when workflows, developer setup, or emulator behavior changes.

## Commit Quality

- Make small, reviewable commits with clear intent.
- Prefer root-cause fixes over narrow patches.
- When a change becomes part of public history, describe the user-visible feature or engineering concern rather than internal delivery phases.
- Preserve the existing code style and project structure unless the change requires otherwise.
