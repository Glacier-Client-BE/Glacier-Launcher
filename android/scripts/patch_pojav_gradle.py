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


ACTIONS = {
    "strip-git-version-block": strip_git_version_block,
    "patch-app-module": patch_app_module,
    "strip-launcher-intent-filter": strip_launcher_intent_filter,
}

if __name__ == "__main__":
    file_path, action = sys.argv[1], sys.argv[2]
    ACTIONS[action](file_path)
