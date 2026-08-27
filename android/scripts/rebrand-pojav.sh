#!/usr/bin/env bash
# Applies the "Glacier Launcher (Java Edition)" rebrand to the vendored
# PojavLauncher submodule at build time, instead of committing changes
# directly into the submodule (which would pin android/pojavlauncher at a
# commit only this sandbox has — unfetchable by CI or any other clone).
#
# Idempotent: safe to run multiple times (e.g. local rebuilds).
set -euo pipefail

cd "$(dirname "$0")/../pojavlauncher"

sed -i.bak 's/applicationId "net.kdt.pojavlaunch"/applicationId "xyz.glacierclient.launcher.java"/' \
    app_pojavlauncher/build.gradle

sed -i.bak 's/PojavLauncher (Minecraft: Java Edition for Android)/Glacier Launcher (Java Edition)/' \
    app_pojavlauncher/src/main/res/values/strings.xml

find app_pojavlauncher -name '*.bak' -delete

echo "Rebranded pojavlauncher submodule -> xyz.glacierclient.launcher.java"
