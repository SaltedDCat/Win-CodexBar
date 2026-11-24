# CI budget glossary

The [Blacksmith Windows workflow](.github/workflows/pr-check.yml) runs on
manual dispatch. The [interaction guard](.github/workflows/interaction-guard.yml)
runs on GitHub-hosted Ubuntu. Set the GitHub Actions repository variable
`CI_BUDGET_MODE` to `off` to skip these checks. Unset, `normal` and `thin`
allow the checks.

## Local check slice

The manual Blacksmith workflow and local validation use this command from the repo root:

```powershell
.\scripts\local-check.ps1 -Slice ci
```

It runs these steps in order:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `pnpm --dir apps/desktop-tauri install --frozen-lockfile`
5. `pnpm --dir apps/desktop-tauri run notices:check` (third-party notices are current)
6. `pnpm --dir apps/desktop-tauri run lint` (oxlint)
7. `pnpm --dir apps/desktop-tauri run check:anti-slop`
8. `pnpm --dir apps/desktop-tauri run test:anti-slop`
9. `pnpm --dir apps/desktop-tauri test`
10. `pnpm --dir apps/desktop-tauri run build`
11. `node --test .github/scripts/interaction-guard.test.mjs`

The slice doesn't build the installer or release assets, and it doesn't prove UI behavior.

For tray, settings, float bar and other UI changes, CI is not enough. Rebuild on Windows and prove the change in the real window with CUA, or attach equivalent manual proof. `AGENTS.md` and `docs/WINDOWS_PROOF.md` describe the proof.
