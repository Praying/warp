# Fork macOS installers

`dev-macos.yml` builds the fork's `dev` branch on every push, using native GitHub-hosted
macOS runners for Apple Silicon (`aarch64`) only. A newer push cancels
an older in-progress build. No Warp organization secrets or Apple credentials are required.

The Git branch and application release channel are separate: these builds compile code
from `dev` using the public `oss` channel, producing **WarpOss.app**, not WarpDev.app.
The OSS channel does not require the private channel configuration generator, disables
telemetry/crash reporting, and has no automatic updater. Its application identity and
settings location differ from the official Warp app.

## Enable and download

1. Enable GitHub Actions in the fork's **Actions** tab if disabled.
2. Commit this workflow to `dev` and push it. Open **Actions → Build dev macOS installers**.
3. After a successful run, download the Apple Silicon artifact from the run's
   **Artifacts** section. The ZIP contains a DMG and its SHA-256 checksum file.
4. Extract the ZIP, optionally verify it with `shasum -a 256 -c <filename>.dmg.sha256`,
   then open the DMG and drag WarpOss into Applications.

Artifacts are retained for 14 days; download them before expiry. This workflow does not
create GitHub Releases or publish updates. Re-run an existing run to rebuild it.
For the **Run workflow** button to appear, the workflow must also exist on the default
branch (`master` in this fork); select `dev` when dispatching it. Other branches are skipped.

## Signing and verification limits

The existing packaging script uses ad-hoc signing when no Apple Development certificate
is present. This is **not** Developer ID signing or Apple notarization: macOS may block
first launch. For a build whose source and origin you trust, use the per-app **Open Anyway**
option in System Settings → Privacy & Security rather than disabling Gatekeeper globally.

Each job builds the app and bundled resources, verifies the DMG with `hdiutil`, and
generates a checksum. This does not test interactive launch or UI behavior. Initial builds
can be slow; dependency caching accelerates later runs. Jobs are capped at 180 minutes
and two Cargo build workers to limit memory pressure on standard hosted runners.
