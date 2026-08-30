// Wraps the native Android JS interface (see MainActivity.kt's AndroidBridge)
// with a browser-safe fallback so this page can also be opened directly in a
// desktop browser for layout iteration without a device.
const Bridge = (() => {
    const native = window.AndroidBridge;
    const hasNative = typeof native !== "undefined";

    return {
        isRootAvailable: () => (hasNative ? native.isRootAvailable() : false),
        attemptInject: (path) => (hasNative ? native.attemptInject(path) : "No native bridge (browser preview)."),
        launchJavaEdition: () => { if (hasNative) native.launchJavaEdition(); },
        launchJavaEditionVersion: (versionId) => { if (hasNative) native.launchJavaEditionVersion(versionId); },
        signInMicrosoft: hasNative ? () => native.signInMicrosoft() : null,
        signInDiscord: hasNative ? () => native.signInDiscord() : null,
        launchBedrock: () => { if (hasNative) native.launchBedrock(); },
        getSettingsJson: () => (hasNative ? native.getSettingsJson() : "{}"),
        saveSettingsJson: (json) => { if (hasNative) native.saveSettingsJson(json); },
        curseForgeApiKey: () => (hasNative ? native.curseForgeApiKey() : ""),
        appVersionName: () => (hasNative ? native.appVersionName() : "0.0.0-dev"),
        openUrl: (url) => { if (hasNative) { native.openUrl(url); } else { window.open(url, "_blank"); } },

        // level.dat editor (LevelDatService.kt).
        levelDatSummary: (worldId) => (hasNative ? native.levelDatSummary(worldId) : '{"ok":false,"error":"No native bridge."}'),
        saveLevelDat: (worldId, patchJson) => (hasNative ? native.saveLevelDat(worldId, patchJson) : '{"ok":false,"error":"No native bridge."}'),

        // Skin Library "Add PNG" import.
        pickSkinPng: () => { if (hasNative) native.pickSkinPng(); },

        // Java Tools (desktop's JavaInstanceService.cs). Each returns the
        // produced path, or "" when there was nothing to do.
        listJavaAssetFolders: () => (hasNative ? native.listJavaAssetFolders() : "[]"),
        openJavaAssetFolder: (folder) => { if (hasNative) native.openJavaAssetFolder(folder); },
        backupJavaSaves: () => (hasNative ? native.backupJavaSaves() : ""),
        exportJavaModpack: () => (hasNative ? native.exportJavaModpack() : ""),
        duplicateJavaInstance: (id) => (hasNative ? native.duplicateJavaInstance(id) : ""),

        // Custom wallpaper (desktop's PickWallpaper/ResetWallpaper).
        pickWallpaper: () => { if (hasNative) native.pickWallpaper(); },
        customBackgroundUrl: () => (hasNative ? native.customBackgroundUrl() : ""),
        resetWallpaper: () => { if (hasNative) native.resetWallpaper(); },

        // Glacier storage folder (GlacierStorage.kt).
        glacierStoragePath: () => (hasNative ? native.glacierStoragePath() : "/storage/emulated/0/games/Glacier"),
        glacierStorageIsShared: () => (hasNative ? native.glacierStorageIsShared() : true),
        requestAllFilesAccess: () => (hasNative ? native.requestAllFilesAccess() : false),

        // Self-update (LauncherUpdateService.kt)
        downloadAndInstallUpdate: (url, tag) => { if (hasNative) native.downloadAndInstallUpdate(url, tag); },

        // Bedrock shared-storage panels (BedrockStorageService.kt/BedrockBackupService.kt)
        hasBedrockStorageAccess: () => (hasNative ? native.hasBedrockStorageAccess() : false),
        requestBedrockStorageAccess: () => { if (hasNative) native.requestBedrockStorageAccess(); },
        bedrockStorageBlockedByPlatform: () => (hasNative ? native.bedrockStorageBlockedByPlatform() : false),
        listBedrockWorlds: () => (hasNative ? native.listBedrockWorlds() : "[]"),
        listBedrockPacks: (kind) => (hasNative ? native.listBedrockPacks(kind) : "[]"),
        listBedrockScreenshots: () => (hasNative ? native.listBedrockScreenshots() : "[]"),
        openBedrockFolder: (name) => { if (hasNative) native.openBedrockFolder(name); },
        listBedrockBackups: () => (hasNative ? native.listBedrockBackups() : "[]"),
        createBedrockBackup: () => (hasNative ? native.createBedrockBackup() : '{"success":false,"message":"No native bridge (browser preview)."}'),
        deleteBedrockBackup: (fileName) => (hasNative ? native.deleteBedrockBackup(fileName) : false),

        // Java multi-instance management (JavaInstanceService.kt)
        listJavaInstances: () => (hasNative ? native.listJavaInstances() : "[]"),
        createJavaInstance: (name, versionId) => (hasNative ? native.createJavaInstance(name, versionId) : "null"),
        renameJavaInstance: (id, newName) => (hasNative ? native.renameJavaInstance(id, newName) : false),
        deleteJavaInstance: (id) => (hasNative ? native.deleteJavaInstance(id) : false),
        setActiveJavaInstance: (id) => (hasNative ? native.setActiveJavaInstance(id) : false),

        // Modpack install (ModpackInstallService.kt)
        installModrinthPack: (url, packName) => (hasNative ? native.installModrinthPack(url, packName) : '{"success":false,"message":"No native bridge (browser preview)."}'),

        // Custom Bedrock client .so picker
        pickCustomDllFile: () => { if (hasNative) native.pickCustomDllFile(); },

        // Side-loaded Bedrock build management (BedrockVersionService.kt)
        pickBedrockApkFile: () => { if (hasNative) native.pickBedrockApkFile(); },
        listBedrockApkBuilds: () => (hasNative ? native.listBedrockApkBuilds() : "[]"),
        downloadBedrockApk: (url) => { if (hasNative) native.downloadBedrockApk(url); },
        backupCurrentBedrockApk: () => (hasNative ? native.backupCurrentBedrockApk() : ""),
        installBedrockApkBuild: (fileName) => (hasNative ? native.installBedrockApkBuild(fileName) : '{"success":false,"message":"No native bridge (browser preview)."}'),
        deleteBedrockApkBuild: (fileName) => (hasNative ? native.deleteBedrockApkBuild(fileName) : false),

        // Logs & Crashes panel (LogService.kt)
        listJavaLogs: () => (hasNative ? native.listJavaLogs() : "[]"),
        readJavaLog: (path) => (hasNative ? native.readJavaLog(path) : "No native bridge (browser preview)."),

        // Exit the app — desktop's .window-controls close button; the
        // minimize/maximize/fullscreen ones alongside it don't apply here.
        closeApp: () => { if (hasNative) native.closeApp(); },
    };
})();
