# ❄️ Glacier Launcher

**Glacier Launcher** is a sleek, frameless Minecraft launcher built for **Minecraft Bedrock Edition (MCBE)**. It focuses on managing and injecting **DLL clients** such as Latite, Flarial, and Oderso, with a smooth and modern desktop experience.

---

## ✨ Features

- 🖥️ Frameless Modern UI  
  Clean design with custom window controls and rounded corners

- 🧩 DLL Client Support  
  Easily manage and inject clients like Latite, Flarial, and Oderso

- 🔄 Auto Updates  
  Launcher and clients stay up to date automatically

- 📦 CurseForge Integration  
  Fetch and manage content directly from CurseForge

- 🎮 Discord Rich Presence  
  Show your activity while using the launcher or playing

- 🎨 Customizable Theming  
  Set your own background, color, and more in the settings

- 💾 Persistent Settings  
  Your preferences and selections are saved locally

- 📚 Version Management  
  Browse, install, and clean up different client versions

---

## 📦 What's Included

- ⚙️ Modern Architecture  
  Built with .NET 10, WPF, and Blazor WebView

- 🌐 GitHub + CurseForge Integration  
  Automatic fetching of releases and content

- 🔧 Client Injection System  
  Simple and fast DLL injection workflow

- 💬 Rich Presence Integration  
  Displays your current activity on Discord

---

❄️ Glacier Launcher makes managing and launching MCBE DLL clients simple, fast, and clean.

---

## 📱 Android

An Android recreation of the launcher lives in [`android/`](android/): a
WebView shell reusing the desktop app's actual `wwwroot/css/app.css` and
image assets for pixel-identical styling, with a thin Kotlin bridge for
native-only pieces, plus a rebranded
[PojavLauncher](https://github.com/PojavLauncherTeam/PojavLauncher)
companion app for Java Edition — both built by CI in
`.github/workflows/android-release.yml`. DLL injection and native Discord
RPC are Windows-only mechanisms with no direct Android equivalent — see
[`android/README.md`](android/README.md) for what is and isn't ported.