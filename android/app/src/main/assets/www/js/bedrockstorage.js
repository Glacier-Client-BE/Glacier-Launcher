// Bridges to BedrockStorageService.kt/BedrockNbt.kt — reads Bedrock's
// shared-storage world data (levelname.txt + level.dat's NBT) through a
// one-time Storage Access Framework grant, since scoped storage means
// there's no plain file path this WebView can read directly. Mirrors the
// read half of Pages/Home.Worlds.cs.
const BedrockStorage = {
    _pending: null,

    hasAccess() {
        return !!(Bridge.hasBedrockStorageAccess && Bridge.hasBedrockStorageAccess());
    },

    requestAccess() {
        if (!Bridge.requestBedrockStorageAccess) {
            return Promise.reject(new Error("Storage access isn't available in this preview (no native bridge)."));
        }
        return new Promise((resolve) => {
            this._pending = resolve;
            Bridge.requestBedrockStorageAccess();
        });
    },

    // Called from native (MainActivity.kt's requestBedrockStorageAccess) once
    // the SAF folder picker returns.
    _onAccessResult(granted) {
        if (!this._pending) return;
        const resolve = this._pending;
        this._pending = null;
        resolve(granted);
    },

    listWorlds() {
        if (!Bridge.listBedrockWorlds) return [];
        try { return JSON.parse(Bridge.listBedrockWorlds() || "[]"); } catch (e) { return []; }
    },

    listPacks(kind) {
        if (!Bridge.listBedrockPacks) return [];
        try { return JSON.parse(Bridge.listBedrockPacks(kind) || "[]"); } catch (e) { return []; }
    },

    listBackups() {
        if (!Bridge.listBedrockBackups) return [];
        try { return JSON.parse(Bridge.listBedrockBackups() || "[]"); } catch (e) { return []; }
    },

    // Writing the backup zip needs no SAF grant (it's saved to this app's
    // own storage), but reading the worlds/packs it zips does — same
    // hasAccess()/requestAccess() gate as listWorlds()/listPacks().
    createBackup() {
        if (!Bridge.createBedrockBackup) return { success: false, message: "Backups aren't available in this preview (no native bridge)." };
        try { return JSON.parse(Bridge.createBedrockBackup()); } catch (e) { return { success: false, message: "Backup failed." }; }
    },

    deleteBackup(fileName) {
        return !!(Bridge.deleteBedrockBackup && Bridge.deleteBedrockBackup(fileName));
    },

    listScreenshots() {
        if (!Bridge.listBedrockScreenshots) return [];
        try { return JSON.parse(Bridge.listBedrockScreenshots() || "[]"); } catch (e) { return []; }
    },
};

function formatBytes(bytes) {
    if (!bytes) return "0 B";
    const units = ["B", "KB", "MB", "GB"];
    let i = 0, n = bytes;
    while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
    return `${n.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function formatRelativeTime(unixSeconds) {
    if (!unixSeconds) return "Never played";
    const diffMs = Date.now() - unixSeconds * 1000;
    const mins = Math.floor(diffMs / 60000);
    if (mins < 1) return "Just now";
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days}d ago`;
    return new Date(unixSeconds * 1000).toLocaleDateString();
}
