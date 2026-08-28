#!/usr/bin/env python3
"""Structural (brace-depth-aware) patches for the vendored PojavLauncher
submodule, applied at build time by rebrand-pojav.sh. Regex alone is too
fragile for removing whole `{ ... }` blocks from Groovy build scripts (a
lazy regex can't tell a nested closing brace from the block's real one) —
this walks brace depth line by line instead, so a removed block is exactly
the block, nothing more or less.
"""
import re
import sys


def remove_brace_block(text: str, start_pattern: str) -> str:
    """Removes the first `<indent><start_pattern> {` ... matching `}` block,
    found by tracking brace depth from the opening line."""
    lines = text.split("\n")
    start_re = re.compile(start_pattern)
    out = []
    i = 0
    removed = False
    while i < len(lines):
        line = lines[i]
        if not removed and start_re.search(line) and line.rstrip().endswith("{"):
            depth = line.count("{") - line.count("}")
            i += 1
            while i < len(lines) and depth > 0:
                depth += lines[i].count("{") - lines[i].count("}")
                i += 1
            removed = True
            continue
        out.append(line)
        i += 1
    return "\n".join(out)


def remove_matching_lines(text: str, pattern: str) -> str:
    regex = re.compile(pattern)
    return "\n".join(line for line in text.split("\n") if not regex.search(line))


def strip_git_version_block(path: str) -> None:
    text = open(path).read()
    text = remove_brace_block(text, r"if \(gitUsed\(\)\)")
    open(path, "w").write(text)


def patch_app_module(path: str) -> None:
    text = open(path).read()

    text = text.replace(
        "id 'com.android.application' version '8.7.2'",
        "id 'com.android.library'",
    )

    # Libraries can't declare applicationId or applicationIdSuffix.
    text = remove_matching_lines(text, r'^\s*applicationId "net\.kdt\.pojavlaunch"\s*$')
    text = remove_matching_lines(text, r"^\s*applicationIdSuffix '\.debug'\s*$")

    # These package-shaped resValues back FileProvider authorities
    # (storageProviderAuthorities is the real manifest <provider
    # android:authorities>) — they must track the real applicationId
    # (xyz.glacierclient.launcher) once merged in, not the old standalone
    # one, or they risk colliding with any other Pojav-based app installed
    # on the same device. Only touches resValue lines — the module's own
    # `namespace 'net.kdt.pojavlaunch'` (its R-class package, unrelated to
    # applicationId) must NOT change, or it'll collide with the main app's
    # own namespace once merged in as a library.
    text = "\n".join(
        line.replace("net.kdt.pojavlaunch", "xyz.glacierclient.launcher")
        if "resValue" in line and "net.kdt.pojavlaunch" in line
        else line
        for line in text.split("\n")
    )

    # `bundle { ... }` (AAB split config) and `signingConfigs { ... }`
    # (referencing debug.keystore/upload.jks, which don't exist here) are
    # application-only DSL — a library module errors on either.
    text = remove_brace_block(text, r"^\s*bundle \{")
    text = remove_brace_block(text, r"^\s*signingConfigs \{")
    # The `signingConfig signingConfigs.*` assignments inside buildTypes
    # that referenced the now-removed signingConfigs block.
    text = remove_matching_lines(text, r"^\s*signingConfig signingConfigs\.\w+\s*$")
    # "Resource shrinker cannot be used for libraries." — shrinkResources
    # only applies at final APK packaging time, which a library module
    # never does itself.
    text = remove_matching_lines(text, r"^\s*shrinkResources (?:true|false)\s*$")

    open(path, "w").write(text)


def strip_launcher_intent_filter(path: str) -> None:
    text = open(path).read()
    text = re.sub(
        r'(<activity\s+android:name="\.TestStorageActivity"[^>]*>)\s*'
        r"<intent-filter>\s*"
        r'<action android:name="android\.intent\.action\.MAIN" />\s*'
        r'<category android:name="android\.intent\.category\.LAUNCHER" />\s*'
        r"</intent-filter>",
        r"\1",
        text,
    )
    open(path, "w").write(text)


_VECTOR_DRAWABLE_TEMPLATE = """<?xml version="1.0" encoding="utf-8"?>
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="24dp"
    android:height="24dp"
    android:viewportWidth="24"
    android:viewportHeight="24"
    android:tint="?attr/colorControlNormal">
    <path
        android:fillColor="#FFFFFFFF"
        android:pathData="{path}" />
</vector>
"""

# GamepadMapperAdapter.java (controller-remap settings screen) references
# these 7 drawables, but none of them ever existed anywhere in Pojav's own
# git history (checked via `git log --all -- '**/<name>*'`) — a real,
# pre-existing bug in upstream's final "Discontinued" commit, unrelated to
# this merge. Stand in with plain glyph vectors (a filled circle for the
# thumbsticks, a triangle per D-pad direction) so the controller-remap
# screen still renders something recognizable instead of failing to
# compile at all.
_MISSING_GAMEPAD_DRAWABLES = {
    "stick_left": "M12,6 C8.7,6 6,8.7 6,12 C6,15.3 8.7,18 12,18 C15.3,18 18,15.3 18,12 C18,8.7 15.3,6 12,6 Z",
    "stick_right": "M12,6 C8.7,6 6,8.7 6,12 C6,15.3 8.7,18 12,18 C15.3,18 18,15.3 18,12 C18,8.7 15.3,6 12,6 Z",
    "stick_left_click": "M12,4 C7.6,4 4,7.6 4,12 C4,16.4 7.6,20 12,20 C16.4,20 20,16.4 20,12 C20,7.6 16.4,4 12,4 Z M12,8 C14.2,8 16,9.8 16,12 C16,14.2 14.2,16 12,16 C9.8,16 8,14.2 8,12 C8,9.8 9.8,8 12,8 Z",
    "stick_right_click": "M12,4 C7.6,4 4,7.6 4,12 C4,16.4 7.6,20 12,20 C16.4,20 20,16.4 20,12 C20,7.6 16.4,4 12,4 Z M12,8 C14.2,8 16,9.8 16,12 C16,14.2 14.2,16 12,16 C9.8,16 8,14.2 8,12 C8,9.8 9.8,8 12,8 Z",
    "dpad_up": "M12,5 L18,13 L14,13 L14,19 L10,19 L10,13 L6,13 Z",
    "dpad_down": "M12,19 L6,11 L10,11 L10,5 L14,5 L14,11 L18,11 Z",
    "dpad_left": "M5,12 L13,6 L13,10 L19,10 L19,14 L13,14 L13,18 Z",
    "dpad_right": "M19,12 L11,18 L11,14 L5,14 L5,10 L11,10 L11,6 Z",
}


def add_missing_gamepad_drawables(res_drawable_dir: str) -> None:
    import os

    os.makedirs(res_drawable_dir, exist_ok=True)
    for name, path_data in _MISSING_GAMEPAD_DRAWABLES.items():
        dest = os.path.join(res_drawable_dir, f"{name}.xml")
        if not os.path.exists(dest):
            open(dest, "w").write(_VECTOR_DRAWABLE_TEMPLATE.format(path=path_data))


ACTIONS = {
    "strip-git-version-block": strip_git_version_block,
    "patch-app-module": patch_app_module,
    "strip-launcher-intent-filter": strip_launcher_intent_filter,
    "add-missing-gamepad-drawables": add_missing_gamepad_drawables,
}

if __name__ == "__main__":
    file_path, action = sys.argv[1], sys.argv[2]
    ACTIONS[action](file_path)
