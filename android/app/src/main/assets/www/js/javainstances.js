// Bridges to JavaInstanceService.kt, which manages instances through the
// vendored Pojav library's own launcher_profiles.json multi-profile system
// rather than a parallel one — see that file for why this is a real,
// working instance switch and not just bookkeeping. Mirrors
// Services/JavaInstanceService.cs's public surface (Instances/Create/
// Rename/Delete/SetActive) at the JS boundary.
const JavaInstances = {
    list() {
        if (!Bridge.listJavaInstances) return [];
        try { return JSON.parse(Bridge.listJavaInstances() || "[]"); } catch (e) { return []; }
    },

    create(name, versionId) {
        if (!Bridge.createJavaInstance) return null;
        try { return JSON.parse(Bridge.createJavaInstance(name, versionId || "")); } catch (e) { return null; }
    },

    rename(id, newName) {
        return !!(Bridge.renameJavaInstance && Bridge.renameJavaInstance(id, newName));
    },

    delete(id) {
        return !!(Bridge.deleteJavaInstance && Bridge.deleteJavaInstance(id));
    },

    setActive(id) {
        return !!(Bridge.setActiveJavaInstance && Bridge.setActiveJavaInstance(id));
    },
};
