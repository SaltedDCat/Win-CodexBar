# Windows validation and release builds

Run `.\scripts\local-check.ps1 -Slice ci` on Windows to validate the workspace.
The manual [Blacksmith workflow](../../.github/workflows/pr-check.yml) runs the
same command.

Build Windows installer, portable, and CLI artifacts with
`.\scripts\windows-release-build.ps1 -Ref vX.Y.Z`. Use `-SmokeInstall` to
test installation and uninstallation. The builder writes SHA-256 sidecars
for the release artifacts. See [Build a release on Windows](../BUILDING.md#build-a-release-on-windows).
