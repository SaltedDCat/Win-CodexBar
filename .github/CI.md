# CI — Win-CodexBar

The manual Blacksmith Windows workflow runs the local check slice. The
interaction guard runs on GitHub-hosted Ubuntu.

## Windows checks

Dispatch [PR check](workflows/pr-check.yml) or run this command from the
repository root on Windows:

```powershell
.\scripts\local-check.ps1 -Slice ci
```

See [CONTEXT.md](../CONTEXT.md) for the check list and budget setting.

## Fork pull requests

1. Read the diff, including build scripts and package manifests.
2. Use a disposable, credential-free Windows environment, such as Windows
   Sandbox or a throwaway VM.
3. Fetch and check out the PR head there. Run the steps listed in the trusted
   base's copy of `scripts/local-check.ps1` one by one.
4. Record the results on the PR.

## Code review

CodeRabbit reviews every PR, drafts included, with the settings in
.coderabbit.yaml. It uses AGENTS.md as its guidelines and runs blocking
pre-merge checks for provider data siloing, secret handling and new
dependencies. It approves a PR once its comments are resolved; maintainers
still merge. It does not build or test anything, and it does not replace UI
proof on a fresh Windows build.
