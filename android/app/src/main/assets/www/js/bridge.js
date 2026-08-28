// Wraps the native Android JS interface (see MainActivity.kt's AndroidBridge)
// with a browser-safe fallback so this page can also be opened directly in a
// desktop browser for layout iteration without a device.
const Bridge = (() => {
    const native = window.AndroidBridge;
    const hasNative = typeof native !== "undefined";

    return {
        isRootAvailable: () => (hasNative ? native.isRootAvailable() : false),
        attemptInject: (path) => (hasNative ? native.attemptInject(path) : "No native bridge (browser preview)."),
        isJavaEditionInstalled: () => (hasNative ? native.isJavaEditionInstalled() : false),
        launchJavaEdition: () => { if (hasNative) native.launchJavaEdition(); },
        hasBundledJavaEditionInstaller: () => (hasNative ? native.hasBundledJavaEditionInstaller() : false),
        installJavaEditionCompanion: () => { if (hasNative) native.installJavaEditionCompanion(); },
        launchBedrock: () => { if (hasNative) native.launchBedrock(); },
        getSettingsJson: () => (hasNative ? native.getSettingsJson() : "{}"),
        saveSettingsJson: (json) => { if (hasNative) native.saveSettingsJson(json); },
        curseForgeApiKey: () => (hasNative ? native.curseForgeApiKey() : ""),
        appVersionName: () => (hasNative ? native.appVersionName() : "0.0.0-dev"),
        openUrl: (url) => { if (hasNative) { native.openUrl(url); } else { window.open(url, "_blank"); } },
    };
})();
