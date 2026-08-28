#!/usr/bin/env bash
# Converts the vendored PojavLauncher submodule from a standalone app into a
# library module built directly into the main Glacier app — one process, one
# APK, no separate "Java Edition companion app" install step — instead of
# committing changes directly into the submodule (which would pin
# android/pojavlauncher at a commit only this sandbox has, unfetchable by CI
# or any other clone).
#
# Idempotent: safe to run multiple times (e.g. local rebuilds).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/../pojavlauncher"

sed -i.bak 's/PojavLauncher (Minecraft: Java Edition for Android)/Glacier Launcher (Java Edition)/' \
    app_pojavlauncher/src/main/res/values/strings.xml

# jre_lwjgl3glfw / arc_dns_injector / forge_installer are plain `java`/
# `java-library` modules (no AGP dependency at all) whose jar tasks write
# straight into app_pojavlauncher's own assets/ folder — safe to include as
# subprojects of the main app's Gradle build directly. Their "write a
# version marker file" step calls gitUsed()/getGitHash(), helper methods
# only defined in pojavlauncher's OWN root build.gradle, which the main
# build never applies (it wires these directories in as its own
# subprojects, bypassing pojavlauncher's root entirely) — so that dead-code
# version-marker step is stripped rather than reimplementing those helpers.
for module in jre_lwjgl3glfw arc_dns_injector forge_installer; do
    python3 "$SCRIPT_DIR/patch_pojav_gradle.py" "$module/build.gradle" strip-git-version-block
done

# app_pojavlauncher: com.android.application -> com.android.library, so it
# builds as a dependency of :app instead of its own installable APK.
python3 "$SCRIPT_DIR/patch_pojav_gradle.py" app_pojavlauncher/build.gradle patch-app-module

# Drop the LAUNCHER intent-filter from TestStorageActivity — once merged in,
# it would show as a second home-screen icon alongside Glacier's own. The
# real Java-version launch path (JavaEditionBridge.launch()) already targets
# net.kdt.pojavlaunch.MainActivity directly, which is the actual JVM/GLFW
# game surface (see android/README.md), not this setup/version-picker flow.
python3 "$SCRIPT_DIR/patch_pojav_gradle.py" app_pojavlauncher/src/main/AndroidManifest.xml strip-launcher-intent-filter

find . -name '*.bak' -delete

echo "Converted pojavlauncher submodule into a library module (xyz.glacierclient.launcher process, no separate install)"
