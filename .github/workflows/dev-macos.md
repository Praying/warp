# Fork macOS installers

`dev-macos.yml` builds the fork's `dev` branch on every push, using native GitHub-hosted
macOS runners for Apple Silicon (`aarch64`) only. A newer push cancels
an older in-progress build. No Apple credentials are required by default: the workflow
produces a self-signed, **unnotarized** installer. To enable Developer ID signing and Apple
notarization, configure your own credentials below; Warp organization credentials are not needed.

The Git branch and application release channel are separate: these builds compile code
from `dev` using the public `oss` channel, producing **WarpOss.app**, not WarpDev.app.
The OSS channel does not require the private channel configuration generator, disables
telemetry/crash reporting, and has no automatic updater. Its application identity and
settings location differ from the official Warp app.

## Enable and download

1. Enable GitHub Actions in the fork's **Actions** tab. Signing secrets are optional.
2. Commit this workflow to `dev` and push it. Open **Actions → Build dev macOS installers**.
3. After a successful run, download the Apple Silicon artifact from the run's
   **Artifacts** section. The ZIP contains a DMG and its SHA-256 checksum file.
4. Extract the ZIP, optionally verify it with `shasum -a 256 -c <filename>.dmg.sha256`,
   then open the DMG and drag WarpOss into Applications.
5. For an **unnotarized** build, attempt to open WarpOss, then use **System Settings →
   Privacy & Security → Open Anyway** only if you trust the source and build. This first-launch
   approval is expected; do not disable Gatekeeper globally.

Both the artifact name and DMG filename include `unnotarized` or `notarized`. The workflow
summary also identifies the mode and explains first-launch approval for unnotarized builds.

Artifacts are retained for 14 days; download them before expiry. This workflow does not
create GitHub Releases or publish updates. Re-run an existing run to rebuild it.
For the **Run workflow** button to appear, the workflow must also exist on the default
branch (`master` in this fork); select `dev` when dispatching it. Other branches are skipped.

## Optional signing and notarization

With none of these secrets configured, the workflow builds without notarization. To enable
signing and notarization, add **all five** repository secrets in
**Settings → Secrets and variables → Actions**:

| Secret | Value |
| --- | --- |
| `APPLE_TEAM_ID` | Your Apple Developer team's ID, matching the signing certificate. |
| `WARP_DEVELOPER_ID_CERT` | Base64-encoded `.p12` containing your **Developer ID Application** certificate and its private key. Do not use an Apple Development or Developer ID Installer certificate. |
| `WARP_DEVELOPER_ID_CERT_PASSWORD` | Password protecting that `.p12` export. |
| `WARP_NOTARIZATION_APPLE_ID` | Apple Account email with access to that developer team. |
| `WARP_NOTARIZATION_PASSWORD` | An app-specific password for that Apple Account, not its login password. |

Creating a Developer ID certificate requires Apple Developer Program membership. Export
the certificate **with its private key** from Keychain Access as a password-protected `.p12`.
On macOS, copy its encoded contents using `base64 -i DeveloperID.p12 | pbcopy`, then paste
into the secret. Never commit the certificate, private key, or passwords to the repository.
The temporary signing keychain password is generated and masked per build; it needs no secret.

If only some secrets are configured, the workflow fails before installing dependencies or
compiling. Complete the configuration or remove all five to return to unnotarized builds.
Once signing is enabled, signing or notarization failures stop the build without silently
falling back to self-signing. The bundler uses the supplied team ID for
signing and notarization, retaining the upstream default for existing upstream workflows.
For a different team, it omits Warp's private App Group entitlement from a temporary copy
of the release entitlements; the source plist is unchanged. OSS has no automatic updater,
and the app's existing storage fallback handles an unavailable Warp App Group.

## Verification and existing downloads

The previous workflow passed `--selfsign`, which falls back to ad-hoc signing on hosted
runners. That skips Developer ID signing and notarization, causing the “Apple cannot check
it for malicious software” first-launch warning. A valid DMG checksum or a successful
`hdiutil verify` does not establish Gatekeeper trust.

Both modes verify DMG integrity, mount it read-only, verify the enclosed app's signature,
and generate a SHA-256 checksum. Unnotarized builds skip notarization and Gatekeeper trust
assessments because their self-signature does not establish distribution trust.

With all credentials configured, the workflow signs with hardened runtime and a secure
timestamp, submits the DMG to Apple, waits for acceptance, and staples the notarization
ticket. Before uploading, it also checks the DMG signature and ticket and assesses both
the DMG and app with Gatekeeper. A failure prevents upload.

Old downloads do not acquire a signature or notarization ticket after this change. To get
a notarized build, configure the secrets and download a new successful build. Without Apple
Developer credentials, there is no packaging-only change that makes an ad-hoc build trusted
by Gatekeeper; unnotarized builds remain available with manual first-launch approval.
For an old build whose source and origin you trust, use the per-app **Open Anyway** option
in **System Settings → Privacy & Security** after attempting to launch it. Do not disable
Gatekeeper globally. Manual local builds can still use `--selfsign`.

The packaging regression tests run with `python3 script/macos/test_bundle_signing.py`.
They mock Apple tools and do not prove real notarization or interactive launch.
The workflow's trust checks also do not test interactive launch or UI behavior. Initial builds
can be slow; dependency caching accelerates later runs. Jobs are capped at 180 minutes
and two Cargo build workers to limit memory pressure on standard hosted runners.
