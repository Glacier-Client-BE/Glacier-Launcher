# Glacier Launcher

A frameless Minecraft launcher that injects Latite Client, built with .NET 10 / WPF / Blazor WebView.

---

## Building

```
dotnet build
```

---

## Adding your own images

Place the following files in `wwwroot/images/` (create the folder if it doesn't exist):

| File             | Usage                                           | Recommended size |
|------------------|-------------------------------------------------|-----------------|
| `bg.jpg`         | Background panorama shown behind the UI         | 460 × 370 px    |
| `icon.png`       | Small logo in the top-left corner               | 28 × 28 px      |
| `avatar.png`     | User avatar shown in the bottom-left footer     | 34 × 34 px      |

Images are referenced from `wwwroot/css/app.css` (`.window-bg`) and `Pages/Home.razor`.
If a file is missing the element simply shows its fallback background colour — no crash.

---

## Discord Rich Presence

1. Go to https://discord.com/developers/applications and create a new application.
2. Copy the **Application ID**.
3. Open `Services/DiscordRpcService.cs` and replace the `ApplicationId` constant with your ID.
4. Upload image assets named `glacier_logo` and `minecraft_icon` in the **Rich Presence → Art Assets** tab of your application.

The launcher will show:
- **"In the Launcher / Selecting a version"** while idle.
- **"Playing Minecraft / Using Latite vX.X"** after a successful injection.

Toggle Discord RPC on/off from the Settings panel inside the launcher.

---

## Clients directory

Downloaded Latite DLLs are stored in `clients/` next to the executable, named `Latite_<tag>.dll`.
You can delete old versions from here at any time.

---

## File structure (source only)

```
GlacierLauncher/
├── Models/
│   ├── LauncherSettings.cs   – Settings data model
│   └── MinecraftVersion.cs   – Version list item model
├── Pages/
│   └── Home.razor            – Full UI (main view, Settings panel, Versions panel)
├── Services/
│   ├── DiscordRpcService.cs  – Discord Rich Presence wrapper
│   ├── GameLauncher.cs       – GitHub release fetching, downloading, injection
│   └── SettingsService.cs    – Load/save glacier-settings.json
├── wwwroot/
│   ├── css/app.css           – All styling
│   ├── images/               – YOUR images go here (see above)
│   ├── js/interop.js         – Window drag + close via WebView2 postMessage
│   └── index.html
├── MainWindow.xaml           – Frameless WPF window with rounded corners + shadow
├── MainWindow.xaml.cs        – WebView2 message handler, Discord RPC init
└── MinecraftLauncher.csproj
```
