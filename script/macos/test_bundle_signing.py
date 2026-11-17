#!/usr/bin/env python3

import json
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest


WORKSPACE = Path(__file__).resolve().parents[2]
BUNDLE = WORKSPACE / "script/macos/bundle"
SOURCE = BUNDLE.read_text()
WORKFLOW = (WORKSPACE / ".github/workflows/dev-macos.yml").read_text()
SIGNING_SCRIPT = SOURCE.split("# Defaults for command-line flags.")[0] + SOURCE[
    SOURCE.index('if [[ $READ_PASSWORDS_FROM_ENV != true ]]; then') :
]
MOCK_TOOL = """#!/usr/bin/env python3
import json
import os
from pathlib import Path
import plistlib
import sys

tool = Path(sys.argv[0]).name
args = sys.argv[1:]
state = Path(os.environ["TEST_STATE"])
with (state / "calls").open("a") as calls:
    calls.write(json.dumps([tool, *args]) + "\\n")
if tool == os.environ.get("FAIL_TOOL"):
    sys.exit(1)
if tool == "rustc":
    print('target_arch="aarch64"')
elif tool == "cargo":
    if "--format" not in args or args[args.index("--format") + 1] != "osx":
        print("hdiutil: convert failed - Resource temporarily unavailable", file=sys.stderr)
        sys.exit(1)
elif tool == "codesign" and "--entitlements" in args:
    with open(args[args.index("--entitlements") + 1], "rb") as source:
        (state / "entitlements.json").write_text(json.dumps(plistlib.load(source)))
elif tool == "create-dmg":
    Path(args[-2]).write_text("mock DMG")
elif tool == "xcrun":
    if args[:2] == ["notarytool", "submit"]:
        print('{"id":"submission-1"}')
    elif args[:2] == ["notarytool", "info"]:
        print(json.dumps({"status": os.environ.get("NOTARY_STATUS", "Accepted")}))
    elif args[:2] == ["stapler", "staple"]:
        sys.exit(int(os.environ.get("STAPLE_EXIT", "0")))
elif tool == "plutil":
    print(json.load(sys.stdin)[args[1]])
elif tool == "hdiutil" and args[0] == "attach":
    (Path(args[-1]) / "WarpOss.app").mkdir()
elif tool == "hdiutil" and args[0] == "detach":
    (Path(args[1]) / "WarpOss.app").rmdir()
elif tool == "bundle":
    (state / "has-keychain-password").write_text(
        str(bool(os.environ.get("WARP_CODESIGN_KEYCHAIN_PASSWORD")))
    )
    sys.exit(int(os.environ.get("BUNDLE_EXIT", "0")))
"""


def workflow_script(name):
    step = WORKFLOW.split(f"      - name: {name}\n", 1)[1].split("      - name:", 1)[0]
    return textwrap.dedent(step.split("        run: |\n", 1)[1])


class BundleSigningTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        tools = self.root / "bin"
        tools.mkdir()
        for tool in (
            "rustc", "cargo", "security", "codesign", "create-dmg", "xcrun", "plutil", "hdiutil", "spctl",
        ):
            executable = tools / tool
            executable.write_text(MOCK_TOOL)
            executable.chmod(0o755)
        self.bundle_dir = self.root / "bundle"
        (self.bundle_dir / "WarpOss.app").mkdir(parents=True)
        self.output_dir = self.root / "output"
        self.output_dir.mkdir()
        self.env = dict(os.environ)
        self.env.pop("APPLE_TEAM_ID", None)
        self.env.update(
            PATH=f"{tools}:{os.environ['PATH']}",
            TEST_STATE=str(self.root),
            BUNDLE_DIR=str(self.bundle_dir),
            OUT_DIR=str(self.output_dir),
            DMG_DIR=str(self.root / "dmg"),
            DMG_NAME="WarpOss.dmg",
            FINAL_DMG_NAME="WarpOss-dev.dmg",
            WARP_APP_NAME="WarpOss",
            ARTIFACT="app",
            CODESIGN="true",
            SELFSIGN="false",
            READ_PASSWORDS_FROM_ENV="true",
            OPEN_AFTER_BUNDLE="false",
            GITHUB_ACTIONS="false",
            WARP_NOTARIZATION_APPLE_ID="test@example.invalid",
            WARP_NOTARIZATION_PASSWORD="test-password",
            WARP_DEVELOPER_ID_CERT="dGVzdA==",
            WARP_DEVELOPER_ID_CERT_PASSWORD="test-password",
            WARP_CODESIGN_KEYCHAIN_PASSWORD="test-password",
            GITHUB_OUTPUT=str(self.root / "github-output"),
            GITHUB_STEP_SUMMARY=str(self.root / "github-summary"),
            GITHUB_SHA="0123456789abcdef",
        )

    def run_signing(self):
        (self.root / "calls").write_text("")
        return subprocess.run(
            ["bash", "-c", SIGNING_SCRIPT], cwd=WORKSPACE, env=self.env,
            capture_output=True, text=True,
        )

    def calls(self):
        return [json.loads(line) for line in (self.root / "calls").read_text().splitlines()]

    def test_cargo_bundle_only_creates_app_before_custom_dmg_packaging(self):
        build_script = SOURCE.split('if [[ "$ARTIFACT" == "app" ]]; then', 1)[1].split(
            '  "$WORKSPACE_ROOT_DIR/script/macos/add_framework_rpath"', 1
        )[0]
        (self.root / "app").mkdir()
        self.env.update(
            DEFAULT_TARGET="aarch64-apple-darwin", RELEASE_CHANNEL="oss",
            BUNDLE_ID="dev.warp.WarpOss", CARGO_PROFILE="release-lto",
            WARP_BIN="warp-oss", FEATURES="release_bundle,extern_plist",
        )
        for build_binary in ("true", "false"):
            with self.subTest(build_binary=build_binary):
                (self.root / "calls").write_text("")
                self.env["BUILD_BINARY"] = build_binary
                result = subprocess.run(
                    ["bash", "-e", "-c", build_script], cwd=self.root, env=self.env,
                    capture_output=True, text=True,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(self.calls(), [[
                    "cargo", "bundle", "--format", "osx", "--profile", "release-lto",
                    "--bin", "warp-oss", "--target", "aarch64-apple-darwin",
                    "--features", "release_bundle,extern_plist",
                ]])

    def test_adhoc_sign_uses_requested_command_without_developer_identity(self):
        signing_script = SOURCE.split("## Step 3: Codesign the app ##", 1)[1].split(
            "## Step 4: Create DMG ##", 1
        )[0]
        self.env.update(
            ADHOC_SIGN="true", SELFSIGN="false", CODESIGN="false", ARTIFACT="app",
            BUNDLE_DIR=str(self.bundle_dir), WARP_APP_NAME="WarpOss",
        )
        result = subprocess.run(
            ["bash", "-e", "-c", signing_script], cwd=WORKSPACE, env=self.env,
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.calls(), [[
            "codesign", "--force", "--deep", "-s", "-", str(self.bundle_dir / "WarpOss.app"),
        ]])

    def run_workflow_step(self, name):
        (self.root / "calls").write_text("")
        return subprocess.run(
            ["bash", "-eo", "pipefail", "-c", workflow_script(name)],
            cwd=self.root, env=self.env, capture_output=True, text=True,
        )

    def test_workflow_signing_mode_for_every_credential_combination(self):
        credentials = (
            "APPLE_TEAM_ID", "WARP_DEVELOPER_ID_CERT", "WARP_DEVELOPER_ID_CERT_PASSWORD",
            "WARP_NOTARIZATION_APPLE_ID", "WARP_NOTARIZATION_PASSWORD",
        )
        for combination in range(1 << len(credentials)):
            with self.subTest(combination=combination):
                for index, name in enumerate(credentials):
                    self.env[name] = "credential-fixture" if combination & (1 << index) else ""
                output = Path(self.env["GITHUB_OUTPUT"])
                summary = Path(self.env["GITHUB_STEP_SUMMARY"])
                output.write_text("")
                summary.write_text("")
                result = self.run_workflow_step("Select signing mode")
                if combination in (0, (1 << len(credentials)) - 1):
                    mode = "unnotarized" if combination == 0 else "notarized"
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(output.read_text(), f"mode={mode}\n")
                    self.assertIn(mode, summary.read_text())
                    self.assertEqual("Open Anyway" in summary.read_text(), combination == 0)
                else:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("::error::", result.stdout)
                    self.assertEqual(output.read_text(), "")
                self.assertNotIn("credential-fixture", result.stdout + result.stderr)

    def test_workflow_build_flags_and_failure_do_not_downgrade(self):
        bundle = self.root / "script/macos/bundle"
        bundle.parent.mkdir(parents=True)
        bundle.write_text(MOCK_TOOL)
        bundle.chmod(0o755)
        self.env.pop("WARP_CODESIGN_KEYCHAIN_PASSWORD")
        for mode in ("unnotarized", "notarized"):
            for exit_code in (0, 1):
                with self.subTest(mode=mode, exit_code=exit_code):
                    self.env.update(SIGNING_MODE=mode, BUNDLE_EXIT=str(exit_code))
                    result = self.run_workflow_step("Build OSS installer")
                    self.assertEqual(result.returncode, exit_code, result.stderr)
                    calls = self.calls()
                    self.assertEqual(len(calls), 1)
                    self.assertEqual("--adhoc-sign" in calls[0], mode == "unnotarized")
                    self.assertEqual("--read-passwords-from-env" in calls[0], mode == "notarized")
                    self.assertIn(f"dev-aarch64-01234567-{mode}", calls[0])
                    self.assertEqual(
                        (self.root / "has-keychain-password").read_text(),
                        str(mode == "notarized"),
                    )

    def test_workflow_verification_depends_on_signing_mode(self):
        dmg = self.root / "WarpOss test.dmg"
        dmg.write_text("mock DMG")
        checksum = Path(f"{dmg}.sha256")
        for mode in ("unnotarized", "notarized"):
            for failure in ("", "hdiutil", "codesign", "xcrun", "spctl"):
                with self.subTest(mode=mode, failure=failure):
                    if checksum.exists():
                        checksum.unlink()
                    self.env.update(SIGNING_MODE=mode, DMG_PATH=str(dmg), FAIL_TOOL=failure)
                    result = self.run_workflow_step("Verify and checksum installer")
                    should_pass = not failure or (
                        mode == "unnotarized" and failure in ("xcrun", "spctl")
                    )
                    self.assertEqual(result.returncode == 0, should_pass, result.stderr)
                    self.assertEqual(checksum.exists(), should_pass)
                    calls = self.calls()
                    if mode == "unnotarized":
                        self.assertFalse(any(call[0] in ("xcrun", "spctl") for call in calls))
                    elif not failure:
                        self.assertTrue(
                            any(call[:3] == ["xcrun", "stapler", "validate"] for call in calls)
                        )
                        self.assertEqual(sum(call[0] == "spctl" for call in calls), 2)
                    if failure != "hdiutil":
                        self.assertEqual(calls[-1][:2], ["hdiutil", "detach"])

    def test_upstream_and_fork_teams(self):
        for team in ("2BBY89MBSN", "FORK123456"):
            with self.subTest(team=team):
                if team != "2BBY89MBSN":
                    self.env["APPLE_TEAM_ID"] = team
                result = self.run_signing()
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                calls = self.calls()
                signatures = [call for call in calls if call[0] == "codesign"]
                self.assertEqual(len(signatures), 2)
                for call in signatures:
                    self.assertEqual(call[call.index("-s") + 1], team)
                    self.assertIn("--timestamp", call)
                submissions = [
                    call for call in calls if call[:3] == ["xcrun", "notarytool", "submit"]
                ]
                self.assertEqual(len(submissions), 1)
                self.assertEqual(submissions[0][submissions[0].index("--team-id") + 1], team)
                self.assertTrue(
                    any(call[:3] == ["xcrun", "stapler", "validate"] for call in calls)
                )
                self.assertTrue((self.output_dir / "WarpOss-dev.dmg").is_file())
                entitlements = json.loads((self.root / "entitlements.json").read_text())
                self.assertEqual(
                    "com.apple.security.application-groups" in entitlements, team == "2BBY89MBSN"
                )
                self.assertNotIn("com.apple.security.get-task-allow", entitlements)
                self.assertEqual(calls[-1][:2], ["security", "delete-keychain"])
                if team != "2BBY89MBSN":
                    signing_file = Path(
                        signatures[0][signatures[0].index("--entitlements") + 1]
                    )
                    self.assertFalse(signing_file.parent.exists())

    def test_selfsign_remains_explicitly_unnotarized(self):
        self.env.update(CODESIGN="false", SELFSIGN="true", READ_PASSWORDS_FROM_ENV="false")
        result = self.run_signing()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse(any(call[0] == "xcrun" for call in self.calls()))
        signatures = [call for call in self.calls() if call[0] == "codesign"]
        self.assertEqual(signatures[0][signatures[0].index("--sign") + 1], "-")

    def test_rejection_and_stapling_failure_prevent_publication(self):
        for overrides in ({"NOTARY_STATUS": "Invalid"}, {"STAPLE_EXIT": "1"}):
            with self.subTest(overrides=overrides):
                self.env.pop("NOTARY_STATUS", None)
                self.env.pop("STAPLE_EXIT", None)
                self.env.update(overrides)
                result = self.run_signing()
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((self.output_dir / "WarpOss-dev.dmg").exists())
                self.assertEqual(self.calls()[-1][:2], ["security", "delete-keychain"])

    def test_missing_credentials_fail_before_build(self):
        for name in (
            "WARP_NOTARIZATION_APPLE_ID", "WARP_NOTARIZATION_PASSWORD", "WARP_DEVELOPER_ID_CERT",
            "WARP_DEVELOPER_ID_CERT_PASSWORD", "WARP_CODESIGN_KEYCHAIN_PASSWORD",
        ):
            with self.subTest(name=name):
                environment = dict(self.env, **{name: ""})
                result = subprocess.run(
                    [
                        "bash", str(BUNDLE), "--channel", "oss", "--arch", "aarch64",
                        "--read-passwords-from-env",
                    ],
                    cwd=WORKSPACE, env=environment, capture_output=True, text=True,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f"{name} must be set for code signing", result.stderr)


if __name__ == "__main__":
    unittest.main()
