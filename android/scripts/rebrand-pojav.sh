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

# Glacier's own manifest force-overrides the merged <application>'s theme to
# Theme.Glacier (a plain android:Theme.Material descendant, for its own
# ComponentActivity UI) — but every one of Pojav's own top-level activities
# extends AppCompatActivity and crashes instantly ("You need to use a
# Theme.AppCompat theme") the moment it inherits that instead of its own
# real AppTheme. Point them back at their own theme explicitly.
python3 "$SCRIPT_DIR/patch_pojav_gradle.py" app_pojavlauncher/src/main/AndroidManifest.xml force-appcompat-theme

# GamepadMapperAdapter.java references 7 drawables (stick_left/right(+click),
# dpad_up/down/left/right) that never existed anywhere in Pojav's own git
# history — a real, pre-existing bug in upstream's final "Discontinued"
# commit that only surfaces now that the module actually gets compiled
# (previously nobody was building this exact submodule commit as a library
# dependency of another app). Generate simple stand-in vector drawables so
# compilation succeeds; see patch_pojav_gradle.py for details.
python3 "$SCRIPT_DIR/patch_pojav_gradle.py" app_pojavlauncher/src/main/res/drawable add-missing-gamepad-drawables

# AGP never generates `final int` R fields for a library module (only for an
# application, where resource IDs get inlined as constants) — `case
# R.id.foo:` is a hard javac error there regardless of the nonFinalResIds
# gradle.properties flag, which only ever affects applications.
# JavaGUILauncherActivity.onTouch() is the one place in this codebase that
# switches on a view ID; rewrite to an if/else-if chain instead.
python3 "$SCRIPT_DIR/patch_pojav_gradle.py" app_pojavlauncher/src/main/java/net/kdt/pojavlaunch/JavaGUILauncherActivity.java fix-nonfinal-resid-switch

find . -name '*.bak' -delete

echo "Converted pojavlauncher submodule into a library module (xyz.glacierclient.launcher process, no separate install)"
