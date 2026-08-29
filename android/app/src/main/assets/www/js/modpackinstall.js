// Partial Android analogue of Services/ModpackInstallService.cs — Modrinth
// modpacks only, and doesn't install a mod loader (Fabric/Quilt/Forge/
// NeoForge) yet. See MainActivity.kt's installModrinthPack/
// ModpackInstallService.kt for exactly why and what that means: the pack's
// overrides + mod files land in a real new Java instance
// (JavaInstances.create), but a loader profile still needs adding manually
// (Pojav's own version-install UI already supports that) before mods that
// need Fabric/Forge will actually load.
const ModpackInstall = {
    // CurseForge packs aren't supported yet — see ModpackInstallService.kt's
    // doc comment for why (project/file ID resolution needs the API-keyed
    // CurseForge client, not a plain download URL like Modrinth).
    async installModrinth(projectId, packName) {
        if (!Bridge.installModrinthPack) {
            return { success: false, message: "Modpack install isn't available in this preview (no native bridge)." };
        }
        const file = await Modrinth.getLatestFile(projectId);
        if (!file) return { success: false, message: "No downloadable .mrpack version found." };
        try {
            return JSON.parse(Bridge.installModrinthPack(file.url, packName));
        } catch (e) {
            return { success: false, message: "Modpack install failed." };
        }
    },
};
