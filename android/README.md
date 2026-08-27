# Glacier Launcher — Android

A native Kotlin/Jetpack Compose recreation of the desktop Glacier Launcher's
UI and workflows (client management, versions, worlds/packs/backups browsing,
settings, theming), built as a standalone Android Studio Gradle project.

## Why this isn't a literal 1:1 port

The desktop launcher's headline feature is **DLL injection** into the Windows
`Minecraft.Windows.exe` / UWP process (Latite, Flarial, OderSo clients), plus
Windows-only integrations: a native system tray icon, `%APPDATA%` file layout,
Discord native RPC, and Java Edition desktop launching.

None of these exist on Android in the same form:

- **No DLL injection equivalent.** Android sandboxes every app by UID; there
  is no supported API to load a foreign native library into another app's
  process. `service/ClientInjectionService.kt` implements the closest
  best-effort analogue (root-only staging of a native lib into Minecraft's
  private storage, à la community Bedrock-Android mod loaders) and is
  explicit in the UI when root is unavailable rather than pretending to work.
- **No system tray** — Android has no desktop shell concept; omitted.
- **Java Edition desktop launching** doesn't apply on Android; Bedrock is
  Mojang's only Android SKU, so the Android app targets Bedrock only.
- File layout uses Android's per-app sandbox (`context.filesDir`) instead of
  `%USERPROFILE%\.glacier`.

Everything else — UI structure, panels (Home, Clients, Worlds, Packs,
Backups, Settings), the Glacier Client manifest/download pipeline, settings
persistence, theming — is recreated to match the desktop app as closely as
the platform allows.

## Building locally

```
cd android
./gradlew :app:assembleDebug
```

(No wrapper jar is committed; CI provisions Gradle via
`gradle/actions/setup-gradle`. Locally, run `gradle wrapper` once, or install
Gradle 8.9 and use the `gradle` command directly.)

## CI

`.github/workflows/android-release.yml` builds debug + release APKs on every
push to `main` that touches `android/**`, uploads them as build artifacts,
and — on commits prefixed `hotfix:`/`update:` — tags and publishes a GitHub
Release, mirroring the desktop launcher's `release.yml` versioning scheme.
