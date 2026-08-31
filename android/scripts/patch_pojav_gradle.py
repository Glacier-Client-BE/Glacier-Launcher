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

    # A library's BuildConfig omits VERSION_NAME/VERSION_CODE by default (they
    # aren't meaningful for a library in general — AGP only auto-generates them
    # for applications), but PojavApplication.java and Tools.java reference
    # BuildConfig.VERSION_NAME directly. Recreate it explicitly via the same
    # getVersionName()/getDateSeconds() calls `versionName`/`versionCode` above
    # already use.
    # Guarded because the replacement text itself still contains the string
    # being matched on, so an unguarded re-run (rebrand-pojav.sh documents
    # itself as idempotent, for local rebuilds) appends a second copy of both
    # buildConfigField lines into the same defaultConfig block every time.
    if "buildConfigField 'String', 'VERSION_NAME'" not in text:
        text = text.replace(
            "versionName getVersionName()",
            "versionName getVersionName()\n"
            "        buildConfigField 'String', 'VERSION_NAME', \"\\\"${getVersionName()}\\\"\"\n"
            "        buildConfigField 'int', 'VERSION_CODE', \"${getDateSeconds()}\"",
        )

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


def force_appcompat_theme(path: str) -> None:
    # Pojav's own top-level activities (MainActivity, LauncherActivity,
    # TestStorageActivity, ImportControlActivity, JavaGUILauncherActivity,
    # CustomControlsActivity) declare no android:theme of their own, so they
    # inherit whatever the merged <application> element's theme ends up
    # being. Glacier's own manifest force-overrides that to Theme.Glacier
    # (tools:replace="...,android:theme,...") for its own ComponentActivity
    # UI, which is NOT an AppCompat descendant — but every one of these
    # Pojav activities extends BaseActivity -> AppCompatActivity, and
    # AppCompatDelegateImpl.createSubDecor() hard-crashes
    # ("You need to use a Theme.AppCompat theme") the instant one of them
    # calls findViewById(), which BaseActivity.onCreate() does immediately
    # via Tools.updateWindowSize(). Pojav's own real theme (styles.xml's
    # AppTheme, extending Theme.AppCompat.NoActionBar) still exists as a
    # library resource post-merge — this just points these activities back
    # at it explicitly so they're unaffected by whatever Glacier's own
    # <application> theme is. (FatalErrorActivity/ShowErrorActivity/
    # ExitActivity already declare their own Theme.AppCompat...Dialog and
    # are unaffected either way.)
    text = open(path).read()
    target_names = {
        "MainActivity",
        "LauncherActivity",
        "TestStorageActivity",
        "ImportControlActivity",
        "JavaGUILauncherActivity",
        "CustomControlsActivity",
    }
    found = set()

    # Match each <activity ...> opening tag generically (non-greedy up to
    # its own closing `>`/`/>` — none of these activities' attribute values
    # contain `>`) rather than assuming android:name is the first attribute,
    # since some (e.g. JavaGUILauncherActivity) list android:process first.
    def patch_tag(match: "re.Match[str]") -> str:
        tag = match.group(0)
        name_match = re.search(r'android:name="\.(\w+)"', tag)
        if not name_match or name_match.group(1) not in target_names:
            return tag
        activity_name = name_match.group(1)
        if 'android:theme="@style/AppTheme"' in tag:
            # Already patched by an earlier run of rebrand-pojav.sh against
            # this same working copy — the script documents itself as
            # idempotent (local rebuilds re-run it against an already-patched
            # submodule, unlike CI which always starts from a fresh
            # checkout). Count it as done rather than treating our own
            # previous output as an upstream change and aborting the build.
            found.add(activity_name)
            return tag
        if "android:theme=" in tag:
            raise SystemExit(
                f"force_appcompat_theme: <activity android:name=\".{activity_name}\"> already declares "
                "its own android:theme — Pojav's manifest changed, this patch is no longer needed there."
            )
        found.add(activity_name)
        closing = "/>" if tag.endswith("/>") else ">"
        return tag[: -len(closing)] + f' android:theme="@style/AppTheme"{closing}'

    text = re.sub(r"<activity\b[^>]*?/?>", patch_tag, text)

    missing = target_names - found
    if missing:
        raise SystemExit(
            "force_appcompat_theme: could not find <activity> tag(s) for "
            f"{sorted(missing)} — Pojav's manifest layout changed, update the pattern."
        )
    open(path, "w").write(text)


def redirect_storage_root(path: str) -> None:
    """Points Pojav's storage root at Glacier's own visible folder.

    Upstream getPojavStorageRoot returns getExternalFilesDir(null) on SDK
    29+, i.e. this app's private Android/data sandbox — precisely the
    directory Android 11+ stops file managers from browsing, so every world,
    mod and config landed somewhere the user cannot reach. Below 29 it used
    a PojavLauncher-named folder, which is also wrong for a rebranded app.

    Both become /storage/emulated/0/games/Glacier, with a fallback to the
    old app-private directory when All Files Access is not held: picking the
    shared folder unconditionally would leave the storage root unwritable on
    Android 11+, and Pojav treats an unwritable root as "no storage", which
    breaks Java Edition outright.

    This logic is duplicated (not shared) with GlacierStorage.preferredRoot
    because app_pojavlauncher is a dependency of :app and cannot reference
    it. The two must stay in step — if they disagree, the launcher UI and the
    game runtime read different directories.
    """
    text = open(path).read()

    if "games/Glacier" in text:
        return  # already patched by an earlier run; rebrand-pojav.sh is idempotent

    original = """    private static File getPojavStorageRoot(Context ctx) {
        if(SDK_INT >= 29) {
            return ctx.getExternalFilesDir(null);
        }else{
            return new File(Environment.getExternalStorageDirectory(),"games/PojavLauncher");
        }
    }"""

    replacement = """    private static File getPojavStorageRoot(Context ctx) {
        // Glacier: keep game data in one visible, launcher-branded folder
        // instead of this app's unreachable Android/data sandbox. Kept in
        // step with GlacierStorage.preferredRoot() on the app side.
        if(SDK_INT < 30 || Environment.isExternalStorageManager()) {
            File shared = new File(Environment.getExternalStorageDirectory(), "games/Glacier");
            if(shared.isDirectory() || shared.mkdirs()) return shared;
        }
        // No All Files Access: fall back to the private directory rather
        // than handing back a root we cannot write to.
        if(SDK_INT >= 29) {
            return ctx.getExternalFilesDir(null);
        }else{
            return new File(Environment.getExternalStorageDirectory(),"games/Glacier");
        }
    }"""

    if original not in text:
        raise SystemExit(
            "redirect_storage_root: getPojavStorageRoot does not match the expected "
            "upstream body in " + path + " — Pojav's source changed, update the patch."
        )
    open(path, "w").write(text.replace(original, replacement))


def drop_localized_app_names(res_dir: str) -> None:
    """Removes app_name/app_short_name from every localized values-*/strings.xml.

    Android resolves a resource to the *best config match*, and only then
    falls back to the app-over-library override. app_name is declared both
    by :app's own values/strings.xml ("Glacier Launcher") and by 48 of this
    library's values-<locale>/strings.xml translations. On a device whose
    locale matches any of those translations, values-de (say) is a strictly
    better config match than :app's unqualified values/, so the library's
    German "PojavLauncher (Minecraft: Java Edition fuer Android)" wins and
    the launcher label reverts to PojavLauncher — even though the icon,
    which has no localized variant, stays Glacier's. Only an en-US-ish
    device that falls all the way back to values/ ever saw the right name.

    Deleting the two names from the translations (rather than rewriting each
    to "Glacier Launcher") makes every locale fall back to the rebranded
    default in values/strings.xml, so there is exactly one place the app
    name is defined. Every other translated string is left untouched.
    """
    import glob
    import os

    name_re = re.compile(
        r'^[ \t]*<string\s+name="(?:app_name|app_short_name)"[^>]*>.*?</string>[ \t]*\r?\n',
        re.MULTILINE | re.DOTALL,
    )
    patched = 0
    for path in sorted(glob.glob(os.path.join(res_dir, "values-*", "strings.xml"))):
        text = open(path, encoding="utf-8").read()
        stripped, count = name_re.subn("", text)
        if count:
            open(path, "w", encoding="utf-8").write(stripped)
            patched += 1
    print(f"drop-localized-app-names: cleared app name overrides in {patched} translation(s)")


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


def fix_nonfinal_resid_switch(path: str) -> None:
    """AGP never generates `final int` R fields for a library module (unlike
    an application, where they're inlined constants) — `case R.id.foo:` is a
    hard javac error ("constant expression required") there, regardless of
    the nonFinalResIds gradle.properties flag, which only ever applied to
    applications. JavaGUILauncherActivity.onTouch() switches on v.getId()
    twice; rewrite both to if/else-if chains, which work with any int value
    known only at runtime."""
    text = open(path).read()

    # Already rewritten by an earlier run against this same working copy.
    # rebrand-pojav.sh documents itself as idempotent (a local rebuild
    # re-runs it over an already-patched submodule, where CI always starts
    # from a fresh checkout), so recognise our own previous output instead
    # of reporting it as an upstream source change and aborting the build.
    if "int vId = v.getId();" in text:
        return

    block_re = re.compile(
        r"[ \t]*switch \(v\.getId\(\)\) \{.*?\n[ \t]*\}\n"
        r"[ \t]*if\(isDown\) switch\(v\.getId\(\)\) \{.*?\n[ \t]*\}",
        re.DOTALL,
    )

    new_block_1 = """        int vId = v.getId();
        if (vId == R.id.installmod_mouse_pri) {
            AWTInputBridge.sendMousePress(AWTInputEvent.BUTTON1_DOWN_MASK, isDown);
        } else if (vId == R.id.installmod_mouse_sec) {
            AWTInputBridge.sendMousePress(AWTInputEvent.BUTTON3_DOWN_MASK, isDown);
        }
        if (isDown) {
            if (vId == R.id.installmod_window_moveup) {
                AWTInputBridge.nativeMoveWindow(0, -10);
            } else if (vId == R.id.installmod_window_movedown) {
                AWTInputBridge.nativeMoveWindow(0, 10);
            } else if (vId == R.id.installmod_window_moveleft) {
                AWTInputBridge.nativeMoveWindow(-10, 0);
            } else if (vId == R.id.installmod_window_moveright) {
                AWTInputBridge.nativeMoveWindow(10, 0);
            }
        }"""

    new_text, count = block_re.subn(new_block_1, text, count=1)
    if count != 1:
        raise SystemExit(
            f"fix_nonfinal_resid_switch: expected switch block not found in {path} "
            "(upstream source likely changed — update the patch)"
        )
    open(path, "w").write(new_text)


ACTIONS = {
    "strip-git-version-block": strip_git_version_block,
    "patch-app-module": patch_app_module,
    "strip-launcher-intent-filter": strip_launcher_intent_filter,
    "force-appcompat-theme": force_appcompat_theme,
    "add-missing-gamepad-drawables": add_missing_gamepad_drawables,
    "fix-nonfinal-resid-switch": fix_nonfinal_resid_switch,
    "drop-localized-app-names": drop_localized_app_names,
    "redirect-storage-root": redirect_storage_root,
}

if __name__ == "__main__":
    file_path, action = sys.argv[1], sys.argv[2]
    ACTIONS[action](file_path)
