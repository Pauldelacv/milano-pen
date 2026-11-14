# Releasing

## Cutting a release

1. Bump the version in **all three** places (CI checks they agree):
   `package.json`, `src-tauri/tauri.conf.json`, and `[workspace.package]` in
   `Cargo.toml`. Run `npm install` and `cargo check` so the lockfiles follow.
2. Merge that change to `main` with CI green.
3. Tag and push:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```
4. The **Release** workflow then:
   - runs the full CI workflow,
   - checks that the tag equals the app version,
   - builds `aarch64-apple-darwin` and `x86_64-apple-darwin` on a macOS 26
     runner (Intel is cross-compiled), producing a `.dmg` and a zipped `.app`
     for each,
   - verifies the code signature (and notarization when configured),
   - computes `SHA256SUMS.txt` and creates a **draft** GitHub release with
     generated notes and the signing status.
   If anything fails, nothing is published.
5. Review the draft release, edit the notes, and publish it.

Add a section to `CHANGELOG.md` in the same pull request as the version bump.

### Pre-releases (beta, release candidate)

Use a SemVer suffix, e.g. `0.1.0-beta.1` or `1.0.0-rc.1`, in all three
version fields and in the tag (`v0.1.0-beta.1`). Any tag containing `-` is
published as a GitHub **pre-release**, is never marked "Latest", and its notes
start with a reminder to back up before upgrading. Increment the last number
for each new beta (`-beta.2`, …); the final release drops the suffix.

## Code signing and notarization (optional)

Without credentials, builds get an ad-hoc signature (required to run on Apple
Silicon) and are **not notarized**; the release notes say so and the README
explains how users can open them. To ship signed and notarized builds, add
these repository secrets (**Settings → Secrets and variables → Actions**):

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64 of your *Developer ID Application* certificate exported as `.p12` (`base64 -i cert.p12 \| pbcopy`) |
| `APPLE_CERTIFICATE_PASSWORD` | The `.p12` export password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | Apple ID email used for notarization |
| `APPLE_PASSWORD` | An [app-specific password](https://support.apple.com/102654) for that Apple ID |
| `APPLE_TEAM_ID` | Your 10-character team ID |

Signing turns on when the first three exist; notarization when all six exist.
Tauri imports the certificate into a temporary keychain, signs, submits to
Apple's notary service and staples the ticket. The workflow then checks the
result with `codesign`, `stapler validate` and `spctl` and fails if it isn't
what the secrets promised.

## Runners

GitHub-hosted runner labels change over time. The workflows use `ubuntu-24.04`
and `macos-26` (Apple Silicon). When GitHub retires an image, update the
labels in `.github/workflows/*.yml` in one pull request.
