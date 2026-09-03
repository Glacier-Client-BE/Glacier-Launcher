# ❄️ Glacier Launcher

**Glacier Launcher** is a sleek, frameless Minecraft launcher for **Bedrock Edition** and **Java
Edition**. On Bedrock it manages and injects **DLL clients** such as Latite, Flarial, OderSo and
LeviLamina; on Java it manages instances, versions, mods and worlds.

---

## ✨ Features

- 🖥️ **Frameless modern UI** — custom window controls, rounded corners, view transitions
- 🧩 **DLL client support** — install, update and inject Latite, Flarial, OderSo and LeviLamina
- 📦 **Bedrock version management** — sideload specific versions, or install the current build from
  the Microsoft Store
- 🌍 **Worlds, packs, backups and screenshots** — including a level.dat editor
- ☕ **Java Edition** — instances, version browser, Fabric/Forge/NeoForge mods, world backups,
  datapacks and skin management
- 🎨 **Theme Studio** — build, import and export custom themes
- 🪄 **VanillaTweaks / BedrockTweaks** — pick tweaks and have them built and installed for you
- 🎮 **Discord Rich Presence**
- 💾 **Persistent settings** — atomic writes, on-disk-compatible with the previous version

---

## 🏗️ Architecture

Glacier Launcher is a **Rust + Tauri v2** desktop app with a **Svelte 5** frontend.

```
Cargo.toml            workspace root
crates/gl-*           library crates (bedrock, java, clients, download, injection,
                      integrations, launch, settings, persistence, nbt, …)
src-tauri/            the `glacier-launcher` binary — thin #[tauri::command]
                      wrappers over the crates, plus window chrome and the tray
frontend/             Svelte 5 (runes) + Vite + TypeScript, plain CSS
```

Two conventions matter if you're changing anything:

- Every new `#[tauri::command]` must be registered in `tauri::generate_handler!` in
  `src-tauri/src/lib.rs`, **and** any new plugin permission added to
  `src-tauri/capabilities/default.json`. A missing ACL entry makes the JS call silently do nothing
  rather than error.
- `frontend/src/app.css` and `frontend/src/interop.js` are verbatim ports. Grep before inventing a
  class name, and don't add a scoped `<style>` that shadows a real global class.

`PARITY.md` is the working record of what has been ported, what deviates, and why.

---

## 🚀 Development

Requirements: Rust (stable), Node 22+, MSVC build tools, and the WebView2 runtime. Injection testing
needs an elevated shell.

```bash
npm run dev
```

Other useful commands:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cd frontend && npx svelte-check --tsconfig ./tsconfig.json
npm run build
```

If a dev run fails to bind port 1420, kill strays first:

```bash
taskkill //F //IM node.exe
```

---

## ⚠️ Status

The Rust/Tauri rewrite replaces an earlier .NET 10 + WPF + Blazor WebView version of this app. Parity
work is tracked in `PARITY.md`, including the features that ship as visibly disabled controls because
their backend isn't ported yet — those are marked in the UI with an explanatory tooltip rather than
faked.

---

❄️ Glacier Launcher makes managing and launching Minecraft clients simple, fast, and clean.
