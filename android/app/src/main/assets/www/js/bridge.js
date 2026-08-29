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

        // Self-update (LauncherUpdateService.kt)
        downloadAndInstallUpdate: (url, tag) => { if (hasNative) native.downloadAndInstallUpdate(url, tag); },

        // Bedrock shared-storage panels (BedrockStorageService.kt/BedrockBackupService.kt)
        hasBedrockStorageAccess: () => (hasNative ? native.hasBedrockStorageAccess() : false),
        requestBedrockStorageAccess: () => { if (hasNative) native.requestBedrockStorageAccess(); },
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

        // Exit the app — desktop's .window-controls close button; the
        // minimize/maximize/fullscreen ones alongside it don't apply here.
        closeApp: () => { if (hasNative) native.closeApp(); },
    };
})();
