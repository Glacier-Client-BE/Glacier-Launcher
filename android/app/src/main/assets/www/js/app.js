// App controller: state, quick-actions/home/footer rendering, panel routing,
// and event delegation. Structure mirrors Pages/Home.razor.cs + Home.Panels.cs
// (currentView string, OpenXxx() methods) translated to plain JS.

const DEFAULT_SETTINGS = {
    selectedClient: "Vanilla",
    // Off by default to match DiscordRpcService.kt: presence needs a user
    // account token the user must supply deliberately (ToS/ban risk), so a
    // default-on toggle would claim to be doing something it isn't.
    discordRichPresence: false,
    username: "",
    closeAfterLaunch: false,
    accentColor: "#7289da",
    themePreset: "dark",
    compactMode: false,
    animationsEnabled: true,
    showRecentlyLaunched: true,
    recentlyLaunched: [],
    checkUpdatesOnStartup: true,
    skippedLauncherVersion: "",
    lastDismissedAnnouncementId: "",
    playSessions: [],
    onboardingCompleted: false,
    userHandle: "",
    xboxGamertag: "",
    xboxXuid: "",
    xboxGamerPictureUrl: "",
    xboxGamerscore: "",
    xboxAccountTier: "",
    javaUsername: "",
    javaUuid: "",
    javaAccessToken: "",
    javaSkinUrl: "",
    discordLoggedIn: false,
    discordUsername: "",
    discordAvatar: "",
    discordUserId: "",
    discordAccessToken: "",
    profileDisplayMode: "auto",
    savedServers: [],
    curseForgeApiKeyOverride: "",
    customDllPath: "",
    activeThemeId: "",
};

// Wraps a settings object in a Proxy that debounce-persists to the native
// SharedPreferences-backed config file (see AndroidBridge.saveSettingsJson,
// "glacier_settings" in MainActivity.kt) on every property write, so the
// config file always reflects current state without every call site having
// to remember an explicit save — the desktop app gets the same guarantee
// for free from SettingsService.Save() being a single json-file writer
// (Models/LauncherSettings.cs) called from one place. Debounced (150ms)
// so a rapid burst of writes (e.g. dragging the theme color picker) only
// hits disk once client code settles.
function autoSavingSettings(target, persist) {
    let timer = null;
    return new Proxy(target, {
        set(obj, prop, value) {
            obj[prop] = value;
            clearTimeout(timer);
            timer = setTimeout(persist, 150);
            return true;
        },
        deleteProperty(obj, prop) {
            delete obj[prop];
            clearTimeout(timer);
            timer = setTimeout(persist, 150);
            return true;
        },
    });
}

const App = {
    state: {
        edition: "bedrock", // "bedrock" | "java" — real toggle from Home.razor's .edition-switcher
        settings: { ...DEFAULT_SETTINGS },
        openPanel: null,
        cfCategory: null,
        cfResults: [],
        cfTotalCount: 0,
        mcVersions: [],
        mcVersionsLoading: false,
        mcVersionsLoaded: false,
        mcVersionsChannel: "all",
        mcVersionsFilter: "",
        javaModsTab: "loaders",
        mrCategory: null,
        mrResults: [],
        mrTotalCount: 0,
        glacier: { loading: false, latest: null, error: null },
        javaVersions: { list: [], loading: false, error: null, filter: "", showSnapshots: false, showHistorical: false },
        downloads: [], // session-scoped, see downloadRowHtml()/downloadsPanelHtml() in panels.js
        modpacks: { source: "mr", query: "", results: [], searching: false, searched: false, error: null, installingId: null },
        themes: [], // loaded from localStorage on init — see loadThemes()/saveThemes()
        selectedThemeId: null,
        // No filesystem skin library on Android, so resolved Mojang texture
        // URLs (stable, signed by Mojang's CDN) are kept in localStorage
        // instead of downloaded PNG bytes. See loadSkins()/saveSkins().
        skinLibrary: { skins: [], username: "", busy: false, error: null },
        msAuth: { loading: false, error: null },
        discordAuth: { loading: false, error: null },
        update: { checking: false, available: false, modalOpen: false, installing: false, progress: 0, info: null },
        // `editing` is the world id whose level.dat editor is expanded (null
        // when none), `levelDat` its loaded summary — see LevelDatService.kt.
        bedrockWorlds: { hasAccess: false, loading: false, loaded: false, worlds: [], editing: null, levelDat: null },
        bedrockPacks: { hasAccess: false, loading: false, loaded: false, kind: "resource", packs: [] },
        bedrockBackups: { hasAccess: false, loading: false, loaded: false, creating: false, confirmDeleteName: null, backups: [] },
        bedrockScreenshots: { hasAccess: false, loading: false, loaded: false, screenshots: [] },
        announcement: null,
        onboarding: { open: false, step: 0, edition: "bedrock", username: "" },
        pendingSessionStart: null,
        javaInstances: { instances: [], renamingId: null, renameValue: "", confirmDeleteId: null },
        news: {
            loading: false, posts: [], releases: [],
            fallbackItems: [
                { icon: "fa-solid fa-snowflake", title: "Glacier Launcher", subtitle: "now on Android", url: "https://glacierclient.xyz" },
                { icon: "fa-brands fa-github", title: "Open source", subtitle: "contributions welcome", url: "https://github.com/Glacier-Client-BE/Glacier-Launcher" },
            ],
        },
    },

    init() {
        try { this.state.settings = { ...DEFAULT_SETTINGS, ...JSON.parse(Bridge.getSettingsJson() || "{}") }; } catch (e) {}
        this.state.settings = autoSavingSettings(this.state.settings, () => this.saveSettings());
        this.loadThemes();
        document.getElementById("version-pill").textContent = `v${Bridge.appVersionName()}`;
        this.renderTopBar();
        this.renderQuickActions();
        this.renderHome();
        this.renderFooter();
        this.renderNews();
        this.updateNotifBadge();
        this.bindGlobalEvents();

        const active = this.state.themes.find(t => t.id === this.state.settings.activeThemeId);
        if (active) ThemeEngine.apply(active);
        // Re-applies a previously chosen wallpaper (MainActivity's
        // customBackgroundUrl) over index.html's bundled bg.jpg default.
        ThemeEngine.restoreWallpaper();

        if (this.state.settings.checkUpdatesOnStartup) this.checkForUpdate(false);
        this.loadAnnouncement();

        this.state.onboarding.open = !this.state.settings.onboardingCompleted;
        this.state.onboarding.edition = this.state.edition;
        this.state.onboarding.username = this.state.settings.username;
        this.renderOnboarding();

        const hasBedrockAccess = BedrockStorage.hasAccess();
        this.state.bedrockWorlds.hasAccess = hasBedrockAccess;
        this.state.bedrockPacks.hasAccess = hasBedrockAccess;
        this.state.bedrockBackups.hasAccess = hasBedrockAccess;
        this.state.bedrockScreenshots.hasAccess = hasBedrockAccess;
    },

    // Worlds, Packs, Backups, and Screenshots share the same one-time SAF
    // grant (all read through the same com.mojang root) — a grant from any
    // one panel unlocks the others too.
    async requestBedrockStorageAccess() {
        const granted = await BedrockStorage.requestAccess();
        this.state.bedrockWorlds.hasAccess = granted;
        this.state.bedrockPacks.hasAccess = granted;
        this.state.bedrockBackups.hasAccess = granted;
        this.state.bedrockScreenshots.hasAccess = granted;
        if (granted) {
            if (this.state.openPanel === "bedrockworlds") await this.loadBedrockWorlds();
            if (this.state.openPanel === "bedrockpacks") await this.loadBedrockPacks();
            if (this.state.openPanel === "bedrockbackups") await this.loadBedrockBackups();
            if (this.state.openPanel === "bedrockscreenshots") await this.loadBedrockScreenshots();
        }
        if (this.state.openPanel === "bedrockworlds") this.openPanel("bedrockworlds");
        if (this.state.openPanel === "bedrockpacks") this.openPanel("bedrockpacks");
        if (this.state.openPanel === "bedrockbackups") this.openPanel("bedrockbackups");
        if (this.state.openPanel === "bedrockscreenshots") this.openPanel("bedrockscreenshots");
    },

    async loadBedrockScreenshots() {
        const bs = this.state.bedrockScreenshots;
        bs.loading = true;
        if (this.state.openPanel === "bedrockscreenshots") this.openPanel("bedrockscreenshots");
        bs.screenshots = BedrockStorage.listScreenshots().sort((a, b) => b.modifiedAt - a.modifiedAt);
        bs.loading = false;
        bs.loaded = true;
        if (this.state.openPanel === "bedrockscreenshots") this.openPanel("bedrockscreenshots");
    },

    async loadBedrockBackups() {
        const bb = this.state.bedrockBackups;
        bb.loading = true;
        if (this.state.openPanel === "bedrockbackups") this.openPanel("bedrockbackups");
        bb.backups = BedrockStorage.listBackups();
        bb.loading = false;
        bb.loaded = true;
        if (this.state.openPanel === "bedrockbackups") this.openPanel("bedrockbackups");
    },

    async createBedrockBackup() {
        const bb = this.state.bedrockBackups;
        bb.creating = true;
        if (this.state.openPanel === "bedrockbackups") this.openPanel("bedrockbackups");
        const result = BedrockStorage.createBackup();
        bb.creating = false;
        if (result.success) {
            bb.backups = BedrockStorage.listBackups();
        } else {
            alert(result.message);
        }
        if (this.state.openPanel === "bedrockbackups") this.openPanel("bedrockbackups");
    },

    confirmDeleteBedrockBackup(fileName) {
        this.state.bedrockBackups.confirmDeleteName = fileName;
        if (this.state.openPanel === "bedrockbackups") this.openPanel("bedrockbackups");
    },

    cancelDeleteBedrockBackup() {
        this.state.bedrockBackups.confirmDeleteName = null;
        if (this.state.openPanel === "bedrockbackups") this.openPanel("bedrockbackups");
    },

    deleteBedrockBackup(fileName) {
        BedrockStorage.deleteBackup(fileName);
        const bb = this.state.bedrockBackups;
        bb.confirmDeleteName = null;
        bb.backups = BedrockStorage.listBackups();
        if (this.state.openPanel === "bedrockbackups") this.openPanel("bedrockbackups");
    },

    async loadBedrockPacks() {
        const bp = this.state.bedrockPacks;
        bp.loading = true;
        if (this.state.openPanel === "bedrockpacks") this.openPanel("bedrockpacks");
        bp.packs = BedrockStorage.listPacks(bp.kind);
        bp.loading = false;
        bp.loaded = true;
        if (this.state.openPanel === "bedrockpacks") this.openPanel("bedrockpacks");
    },

    switchBedrockPackKind(kind) {
        const bp = this.state.bedrockPacks;
        if (bp.kind === kind) return;
        bp.kind = kind;
        bp.loaded = false;
        if (this.state.openPanel === "bedrockpacks") this.openPanel("bedrockpacks");
    },

    async pickCustomDll() {
        try {
            const path = await CustomDllPicker.pick();
            if (path) {
                this.state.settings.customDllPath = path;
                this.saveSettings();
            }
        } catch (e) { /* cancelled or no native bridge */ }
        if (this.state.openPanel === "clients") this.openPanel("clients");
    },

    clearCustomDll() {
        this.state.settings.customDllPath = "";
        this.saveSettings();
        if (this.state.openPanel === "clients") this.openPanel("clients");
    },

    stageCustomDllInjection() {
        const path = this.state.settings.customDllPath;
        if (!path || !Bridge.attemptInject) return;
        const message = Bridge.attemptInject(path);
        this.state.settings.selectedClient = "Custom DLL";
        this.saveSettings();
        alert(message);
        if (this.state.openPanel === "clients") this.openPanel("clients");
    },

    async loadMcVersions() {
        this.state.mcVersionsLoading = true;
        if (this.state.openPanel === "mcversions") this.openPanel("mcversions");
        try {
            this.state.mcVersions = await BedrockVersions.fetch();
        } catch (e) {
            this.state.mcVersions = [];
        }
        this.state.mcVersionsLoading = false;
        this.state.mcVersionsLoaded = true;
        if (this.state.openPanel === "mcversions") this.openPanel("mcversions");
    },

    newJavaInstance() {
        const created = JavaInstances.create("New Instance", "");
        this.state.javaInstances.instances = JavaInstances.list();
        if (this.state.openPanel === "javaprofile") this.openPanel("javaprofile");
        return created;
    },

    switchJavaInstance(id) {
        if (!JavaInstances.setActive(id)) return;
        this.state.javaInstances.instances = JavaInstances.list();
        if (this.state.openPanel === "javaprofile") this.openPanel("javaprofile");
    },

    beginRenameInstance(id, name) {
        this.state.javaInstances.renamingId = id;
        this.state.javaInstances.renameValue = name;
        if (this.state.openPanel === "javaprofile") this.openPanel("javaprofile");
    },

    commitRenameInstance(id) {
        const input = document.querySelector("[data-rename-instance-input]");
        const newName = input ? input.value : this.state.javaInstances.renameValue;
        JavaInstances.rename(id, newName);
        this.state.javaInstances.renamingId = null;
        this.state.javaInstances.instances = JavaInstances.list();
        if (this.state.openPanel === "javaprofile") this.openPanel("javaprofile");
    },

    confirmDeleteInstance(id) {
        this.state.javaInstances.confirmDeleteId = id;
        if (this.state.openPanel === "javaprofile") this.openPanel("javaprofile");
    },

    cancelDeleteInstance() {
        this.state.javaInstances.confirmDeleteId = null;
        if (this.state.openPanel === "javaprofile") this.openPanel("javaprofile");
    },

    deleteJavaInstance(id) {
        JavaInstances.delete(id);
        this.state.javaInstances.confirmDeleteId = null;
        this.state.javaInstances.instances = JavaInstances.list();
        if (this.state.openPanel === "javaprofile") this.openPanel("javaprofile");
    },

    async loadBedrockWorlds() {
        this.state.bedrockWorlds.loading = true;
        if (this.state.openPanel === "bedrockworlds") this.openPanel("bedrockworlds");
        this.state.bedrockWorlds.worlds = BedrockStorage.listWorlds();
        this.state.bedrockWorlds.loading = false;
        this.state.bedrockWorlds.loaded = true;
        if (this.state.openPanel === "bedrockworlds") this.openPanel("bedrockworlds");
    },

    saveSettings() {
        Bridge.saveSettingsJson(JSON.stringify(this.state.settings));
    },

    // Theme Studio's list lives in localStorage (per-device, like the desktop
    // app's themes.json file) rather than the SharedPreferences settings blob,
    // since it's a list of documents rather than a single settings object.
    loadThemes() {
        try { this.state.themes = JSON.parse(localStorage.getItem("glacier_themes") || "[]"); } catch (e) { this.state.themes = []; }
    },

    loadSkins() {
        try { this.state.skinLibrary.skins = JSON.parse(localStorage.getItem("glacier_skins") || "[]"); } catch (e) { this.state.skinLibrary.skins = []; }
    },

    saveSkins() {
        localStorage.setItem("glacier_skins", JSON.stringify(this.state.skinLibrary.skins));
    },

    renderSkinLibraryBody() {
        const body = document.getElementById("skinlib-body");
        if (body) body.innerHTML = skinLibraryPanelBody(this.state.skinLibrary);
    },

    // ── level.dat editor (desktop's LevelDatEditorService.cs) ────────────

    openLevelDatEditor(worldId) {
        const bw = this.state.bedrockWorlds;
        // Clicking the open world's own button closes it again.
        if (bw.editing === worldId) { this.closeLevelDatEditor(); return; }
        bw.editing = worldId;
        bw.levelDat = { loading: true };
        if (this.state.openPanel === "bedrockworlds") this.openPanel("bedrockworlds");

        const result = JSON.parse(Bridge.levelDatSummary(worldId));
        bw.levelDat = result;
        if (this.state.openPanel === "bedrockworlds") this.openPanel("bedrockworlds");
    },

    closeLevelDatEditor() {
        this.state.bedrockWorlds.editing = null;
        this.state.bedrockWorlds.levelDat = null;
        if (this.state.openPanel === "bedrockworlds") this.openPanel("bedrockworlds");
    },

    // Edits are held in the loaded summary and only written on Save, so a
    // mis-tap never touches level.dat — matching desktop, where the editor
    // is a form with its own Save button rather than live-applying.
    setLevelDatField(key, value) {
        const ld = this.state.bedrockWorlds.levelDat;
        if (!ld || !ld.ok) return;
        ld[key] = value;
        ld.status = "";
        if (this.state.openPanel === "bedrockworlds") this.openPanel("bedrockworlds");
    },

    toggleLevelDatExperiment(name) {
        const ld = this.state.bedrockWorlds.levelDat;
        if (!ld || !ld.ok || !ld.experiments) return;
        ld.experiments[name] = !ld.experiments[name];
        ld.status = "";
        if (this.state.openPanel === "bedrockworlds") this.openPanel("bedrockworlds");
    },

    saveLevelDat() {
        const bw = this.state.bedrockWorlds;
        const ld = bw.levelDat;
        if (!ld || !ld.ok || !bw.editing) return;

        // Only the editable fields go over; the seed is read-only and the
        // native side ignores anything it isn't asked to change.
        const patch = {
            gameType: ld.gameType,
            difficulty: ld.difficulty,
            generator: ld.generator,
            cheats: !!ld.cheats,
            experiments: ld.experiments || {},
        };
        const result = JSON.parse(Bridge.saveLevelDat(bw.editing, JSON.stringify(patch)));
        ld.status = result.ok ? "Saved." : (result.error || "Couldn't save.");
        if (this.state.openPanel === "bedrockworlds") this.openPanel("bedrockworlds");
        if (result.ok) this.loadBedrockWorlds();
    },

    // Reports the real outcome in the card's own subtitle rather than a
    // generic success: each native call returns the produced path, or "" when
    // there was nothing to do (e.g. no worlds to back up), and desktop
    // distinguishes those cases too.
    runJavaTool(id) {
        const status = document.querySelector(`[data-java-tool-status="${id}"]`);
        const say = (msg) => { if (status) status.textContent = msg; };
        say("Working…");

        let result = "";
        try {
            if (id === "backup-saves") result = Bridge.backupJavaSaves();
            else if (id === "export-modpack") result = Bridge.exportJavaModpack();
            else if (id === "duplicate-instance") {
                const active = JSON.parse(Bridge.listJavaInstances() || "[]").find(i => i.isActive);
                result = active ? Bridge.duplicateJavaInstance(active.id) : "";
            }
        } catch (e) {
            say("Failed: " + e.message);
            return;
        }

        if (!result) {
            say(id === "backup-saves" ? "Nothing to back up yet." : "Couldn't complete — no active instance.");
            return;
        }
        if (id === "duplicate-instance") {
            const copy = JSON.parse(result);
            say(`Duplicated as "${copy.name}".`);
            return;
        }
        say("Saved to " + result.split("/").pop());
    },

    async addSkinFromUsername() {
        const sl = this.state.skinLibrary;
        if (!sl.username.trim() || sl.busy) return;
        sl.busy = true; sl.error = null;
        this.renderSkinLibraryBody();
        try {
            const skin = await SkinLibrary.addFromUsername(sl.username);
            sl.skins.unshift(skin);
            sl.username = "";
            this.saveSkins();
        } catch (e) {
            sl.error = e.message;
        }
        sl.busy = false;
        this.renderSkinLibraryBody();
    },

    async applySkinFromLibrary(skin, slim) {
        const sl = this.state.skinLibrary;
        if (!this.state.settings.javaAccessToken) {
            sl.error = "Sign in with Microsoft (Java) first to apply skins.";
            this.renderSkinLibraryBody();
            return;
        }
        sl.busy = true; sl.error = null;
        this.renderSkinLibraryBody();
        const error = await SkinLibrary.applySkin(this.state.settings.javaAccessToken, skin.url, slim);
        sl.busy = false;
        if (error) {
            sl.error = error;
        } else {
            this.state.settings.javaSkinUrl = skin.url;
            this.saveSettings();
            this.renderFooter();
        }
        this.renderSkinLibraryBody();
    },

    saveThemes() {
        localStorage.setItem("glacier_themes", JSON.stringify(this.state.themes));
    },

    setEdition(edition) {
        this.state.edition = edition;
        document.getElementById("edition-bedrock").classList.toggle("active", edition === "bedrock");
        document.getElementById("edition-java").classList.toggle("active", edition === "java");
        this.renderQuickActions();
        this.closePanel();
    },

    // ── Top bar ────────────────────────────────────────────────────────
    renderTopBar() {
        const chip = document.getElementById("client-chip");
        const label = document.getElementById("client-chip-label");
        if (this.state.edition === "bedrock") {
            chip.style.display = "";
            label.textContent = this.state.settings.selectedClient;
        } else {
            chip.style.display = "none";
        }
    },

    // ── Quick actions dock (real markup: .main-content > .btn-row) ──────
    renderQuickActions() {
        const bedrock = `
            <button class="action-btn hover-grow launch-btn btn-sq" id="launch-btn" data-tooltip="Launch (Ctrl+L)">
                <i class="fa-solid fa-play"></i><span class="btn-label">Launch</span>
            </button>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="settings" data-tooltip="Settings">
                <i class="fa-solid fa-gear"></i><span class="btn-label">Settings</span></button></div>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="clients" data-tooltip="Clients (Ctrl+1)">
                <i class="fa-solid fa-puzzle-piece"></i><span class="btn-label">Clients</span></button></div>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="addons" data-tooltip="Addons (Ctrl+2)">
                <i class="fa-solid fa-cube"></i><span class="btn-label">Addons</span></button></div>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="servers" data-tooltip="Servers (Ctrl+3)">
                <i class="fa-solid fa-server"></i><span class="btn-label">Servers</span></button></div>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="mcversions" data-tooltip="MC Versions (Ctrl+4)">
                <i class="fa-solid fa-box-archive"></i><span class="btn-label">MC Versions</span></button></div>`;
        const java = `
            <button class="action-btn hover-grow launch-btn btn-sq" id="launch-btn" data-tooltip="Launch (Ctrl+L)">
                <i class="fa-solid fa-play"></i><span class="btn-label">Launch</span>
            </button>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="settings" data-tooltip="Settings">
                <i class="fa-solid fa-gear"></i><span class="btn-label">Settings</span></button></div>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="javaclients" data-tooltip="Launchers (Ctrl+1)">
                <i class="fa-solid fa-rocket"></i><span class="btn-label">Launchers</span></button></div>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="addons" data-tooltip="Mods (Ctrl+2)">
                <i class="fa-solid fa-puzzle-piece"></i><span class="btn-label">Mods</span></button></div>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="javaversions" data-tooltip="Versions (Ctrl+3)">
                <i class="fa-solid fa-box-archive"></i><span class="btn-label">Versions</span></button></div>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="javaprofile" data-tooltip="Profile">
                <i class="fa-solid fa-user"></i><span class="btn-label">Profile</span></button></div>
            <div class="btn-badge-wrap"><button class="action-btn btn-sq hover-grow" data-open-panel="javascreenshots" data-tooltip="Screenshots">
                <i class="fa-solid fa-images"></i><span class="btn-label">Screenshots</span></button></div>`;
        document.getElementById("quick-actions").innerHTML = this.state.edition === "bedrock" ? bedrock : java;
    },

    // ── Home view ────────────────────────────────────────────────────────
    renderHome() {
        const s = this.state.settings;
        const statusEl = document.getElementById("status-msg");
        statusEl.textContent = "";
        statusEl.classList.remove("visible", "error");

        const wrap = document.getElementById("recently-launched");
        const items = document.getElementById("recently-launched-items");
        if (s.showRecentlyLaunched && (s.recentlyLaunched || []).length > 0) {
            wrap.style.display = "";
            items.innerHTML = s.recentlyLaunched.slice(0, 3).map(name =>
                `<button class="recent-item"><i class="fa-solid fa-clock-rotate-left"></i>${name}</button>`).join("");
        } else {
            wrap.style.display = "none";
        }
    },

    // Mirrors Pages/Home.razor.cs's EffectiveProfile(): "auto" prefers Xbox,
    // falling back to Discord, falling back to no signed-in profile.
    effectiveProfile() {
        const s = this.state.settings;
        switch (s.profileDisplayMode) {
            case "xbox":    return s.xboxGamertag ? "xbox" : "none";
            case "discord": return s.discordLoggedIn ? "discord" : "none";
            default:
                if (s.xboxGamertag) return "xbox";
                if (s.discordLoggedIn) return "discord";
                return "none";
        }
    },

    // ── Footer (real markup: .footer / .footer-profile-area / .footer-right) ──
    renderFooter() {
        const s = this.state.settings;
        const which = this.effectiveProfile();
        const avatar = document.querySelector("#footer-profile .avatar");
        if (avatar) avatar.classList.remove("xbox-connected", "discord-connected");
        if (which === "xbox") {
            document.getElementById("footer-username").textContent = s.xboxGamertag;
            document.getElementById("footer-handle").textContent = "Xbox Live";
            if (avatar) { avatar.src = s.xboxGamerPictureUrl || "images/icon.png"; avatar.classList.add("xbox-connected"); }
        } else if (which === "discord") {
            document.getElementById("footer-username").textContent = s.discordUsername;
            document.getElementById("footer-handle").textContent = "Discord";
            if (avatar) { avatar.src = s.discordAvatar || "images/icon.png"; avatar.classList.add("discord-connected"); }
        } else {
            document.getElementById("footer-username").textContent = s.username || "Not signed in";
            document.getElementById("footer-handle").textContent = "Local profile";
            if (avatar) avatar.src = "images/icon.png";
        }
        document.getElementById("rpc-toggle").classList.toggle("rpc-active", !!s.discordRichPresence);

        const xboxBtn = document.getElementById("xbox-connect-btn");
        if (s.xboxGamertag && xboxBtn) {
            xboxBtn.outerHTML = `<div class="xbox-connected-pill" id="xbox-connect-btn"><i class="fa-solid fa-gamepad"></i><span>${s.xboxGamertag}</span></div>`;
        }
        const discordBtn = document.getElementById("discord-connect-btn");
        if (s.discordLoggedIn && discordBtn) {
            discordBtn.outerHTML = `<div class="discord-connected-pill" id="discord-connect-btn"><i class="fa-brands fa-discord"></i><span>${s.discordUsername}</span></div>`;
        }
    },

    async signInWithMicrosoft() {
        if (this.state.msAuth.loading) return;
        this.state.msAuth.loading = true;
        this.state.msAuth.error = null;
        this.renderFooter();
        if (this.state.openPanel === "javaprofile") this.openPanel("javaprofile");
        try {
            const { profile, mcProfile, mcAccessToken } = await MicrosoftAuth.begin();
            const s = this.state.settings;
            s.xboxGamertag = profile.gamertag;
            s.xboxXuid = profile.xuid;
            s.xboxGamerPictureUrl = profile.gamerPictureUrl;
            s.xboxGamerscore = profile.gamerscore;
            s.xboxAccountTier = profile.accountTier;
            if (mcProfile) {
                s.javaUsername = mcProfile.name;
                s.javaUuid = mcProfile.uuid;
                s.javaAccessToken = mcAccessToken || "";
                s.javaSkinUrl = mcProfile.skinUrl || "";
            }
            this.saveSettings();
        } catch (e) {
            this.state.msAuth.error = e.message;
        }
        this.state.msAuth.loading = false;
        this.renderFooter();
        if (this.state.openPanel === "javaprofile") this.openPanel("javaprofile");
    },

    signOutMicrosoft() {
        const s = this.state.settings;
        s.xboxGamertag = ""; s.xboxXuid = ""; s.xboxGamerPictureUrl = ""; s.xboxGamerscore = ""; s.xboxAccountTier = "";
        s.javaUsername = ""; s.javaUuid = ""; s.javaAccessToken = ""; s.javaSkinUrl = "";
        this.saveSettings();
        this.renderFooter();
        if (this.state.openPanel === "javaprofile") this.openPanel("javaprofile");
    },

    async signInWithDiscord() {
        if (this.state.discordAuth.loading) return;
        this.state.discordAuth.loading = true;
        this.state.discordAuth.error = null;
        this.renderFooter();
        try {
            const { profile, accessToken } = await DiscordAuth.begin();
            const s = this.state.settings;
            s.discordLoggedIn = true;
            s.discordUsername = profile.username;
            s.discordAvatar = profile.avatarUrl;
            s.discordUserId = profile.userId;
            s.discordAccessToken = accessToken;
            this.saveSettings();
        } catch (e) {
            this.state.discordAuth.error = e.message;
        }
        this.state.discordAuth.loading = false;
        this.renderFooter();
    },

    signOutDiscord() {
        const s = this.state.settings;
        s.discordLoggedIn = false; s.discordUsername = ""; s.discordAvatar = ""; s.discordUserId = ""; s.discordAccessToken = "";
        this.saveSettings();
        this.renderFooter();
    },

    // ── Lightweight session-timer (mirrors StatsPanel's session tracking,
    // approximated per MainActivity.kt's onResume() doc comment) ──────────
    launchJava() {
        this.recordLaunchStart();
        // No version id: launchJavaEdition() lets Pojav resolve the profile's
        // own last-used version, so presence reports the menu state. The
        // versioned launch path below sets the real version instead.
        Bridge.discordRpcSetJava("", "");
        Bridge.launchJavaEdition();
    },

    launchBedrockGame() {
        this.recordLaunchStart();
        // selectedClient holds exactly the names DiscordRpcService.cs's
        // SetInGamePresence() switches on. Bedrock is launched as an
        // installed app here, so its version tag isn't ours to know.
        Bridge.discordRpcSetBedrock("", this.state.settings.selectedClient);
        Bridge.launchBedrock();
    },

    // Pushes the current enabled-state to the native service. Called on the
    // toggle and once at startup so the two stay in step.
    applyDiscordRpcSetting() {
        Bridge.discordRpcConfigure(!!this.state.settings.discordRichPresence, "");
    },

    recordLaunchStart() {
        this.state.pendingSessionStart = Date.now();
    },

    // Called from native (MainActivity.kt's onResume) once this Activity
    // regains focus — the closest signal Android gives for "the launched
    // game Activity closed." Sessions under 10s are dropped as probably a
    // launch that immediately failed/bounced rather than real play.
    onResumeFromGame() {
        const start = this.state.pendingSessionStart;
        if (!start) return;
        this.state.pendingSessionStart = null;
        // Back to the launcher, so back to the idle presence — same
        // transition the desktop app makes when the game exits.
        Bridge.discordRpcSetIdle();
        const durationSeconds = Math.round((Date.now() - start) / 1000);
        if (durationSeconds < 10) return;

        const sessions = this.state.settings.playSessions || [];
        sessions.push({ start, durationSeconds });
        if (sessions.length > 200) sessions.splice(0, sessions.length - 200);
        this.state.settings.playSessions = sessions;
        this.saveSettings();
        if (this.state.openPanel === "stats") this.openPanel("stats");
    },

    // ── Onboarding wizard (mirrors Home.Onboarding.cs) ─────────────────────
    renderOnboarding() {
        document.getElementById("onboarding-root").innerHTML = onboardingModalHtml(this.state.onboarding);
    },

    onboardingPickEdition(edition) {
        this.state.onboarding.edition = edition;
        this.renderOnboarding();
    },

    onboardingNext() {
        const input = document.getElementById("onboarding-username-input");
        if (input) this.state.onboarding.username = input.value;
        this.setEdition(this.state.onboarding.edition);
        this.state.onboarding.step = 1;
        this.renderOnboarding();
    },

    onboardingBack() {
        this.state.onboarding.step = 0;
        this.renderOnboarding();
    },

    finishOnboarding() {
        const input = document.getElementById("onboarding-username-input");
        const username = (input ? input.value : this.state.onboarding.username).trim();
        if (username) {
            this.state.settings.username = username;
            this.state.settings.userHandle = "@" + username.toLowerCase().replace(/\s+/g, "");
        }
        this.state.settings.onboardingCompleted = true;
        this.saveSettings();
        this.state.onboarding.open = false;
        this.renderOnboarding();
        this.renderFooter();
    },

    skipOnboarding() {
        this.state.settings.onboardingCompleted = true;
        this.saveSettings();
        this.state.onboarding.open = false;
        this.renderOnboarding();
    },

    // ── Announcement / maintenance banner (mirrors Home.Announcement.cs) ──
    async loadAnnouncement() {
        this.state.announcement = await AnnouncementFeed.fetch();
        this.renderAnnouncement();
    },

    renderAnnouncement() {
        document.getElementById("announcement-root").innerHTML =
            announcementBannerHtml(this.state.announcement, this.state.settings.lastDismissedAnnouncementId);
    },

    dismissAnnouncement(id) {
        this.state.settings.lastDismissedAnnouncementId = id;
        this.saveSettings();
        this.renderAnnouncement();
    },

    // ── Launcher self-update (mirrors AutoUpdateService.cs / Home.Panels.cs's
    // RunStartupChecksAsync/ManualUpdateCheck) ──────────────────────────────
    renderUpdatePill() {
        const pill = document.getElementById("update-pill");
        if (!pill) return;
        if (this.state.update.available) {
            pill.innerHTML = `<i class="fa-solid fa-arrow-up"></i><span>Update</span>`;
        } else {
            pill.innerHTML = `<span id="version-pill">v${Bridge.appVersionName ? Bridge.appVersionName() : "0.0.0"}</span>`;
        }
    },

    renderUpdateModal() {
        document.getElementById("update-root").innerHTML = updateModalHtml(this.state.update, Bridge.appVersionName ? Bridge.appVersionName() : "0.0.0");
    },

    async checkForUpdate(manual) {
        if (this.state.update.checking) return;
        this.state.update.checking = true;
        try {
            const info = await LauncherUpdate.check();
            if (info && info.tag !== this.state.settings.skippedLauncherVersion) {
                this.state.update.available = true;
                this.state.update.info = info;
                if (manual) this.openUpdateModal();
            } else if (manual) {
                this.state.update.available = false;
            }
        } catch (e) {
            // Offline / rate-limited — same silent-fallback behavior as
            // AutoUpdateService's startup check catch block.
        }
        this.state.update.checking = false;
        this.renderUpdatePill();
    },

    openUpdateModal() { this.state.update.modalOpen = true; this.renderUpdateModal(); },
    closeUpdateModal() { this.state.update.modalOpen = false; this.renderUpdateModal(); },

    skipUpdate() {
        if (!this.state.update.info) return;
        this.state.settings.skippedLauncherVersion = this.state.update.info.tag;
        this.saveSettings();
        this.state.update.available = false;
        this.state.update.modalOpen = false;
        this.renderUpdatePill();
        this.renderUpdateModal();
    },

    async installUpdate() {
        if (!this.state.update.info) return;
        this.state.update.installing = true;
        this.state.update.progress = 0;
        this.renderUpdateModal();
        try {
            await LauncherUpdate.install(this.state.update.info, (pct) => {
                this.state.update.progress = pct;
                this.renderUpdateModal();
            });
            // The system install dialog is now on top of the WebView — close
            // our own modal underneath it rather than leaving a stale 100%
            // progress bar around for whenever the user returns.
            this.state.update.installing = false;
            this.state.update.modalOpen = false;
            this.renderUpdateModal();
        } catch (e) {
            this.state.update.installing = false;
            this.renderUpdateModal();
        }
    },

    // ── Global search (real markup: .search-overlay/.search-modal) ──────
    openSearch() {
        this.state.search = { query: "", selIdx: 0, edition: this.state.edition };
        this._searchResultsCache = searchQuickActions(this.state.edition);
        document.getElementById("search-root").innerHTML = searchOverlayHtml(this.state.search);
        setTimeout(() => document.getElementById("search-modal-input")?.focus(), 0);
    },

    closeSearch() {
        this.state.search = null;
        document.getElementById("search-root").innerHTML = "";
    },

    renderSearch() {
        if (!this.state.search) return;
        document.getElementById("search-root").innerHTML = searchOverlayHtml(this.state.search);
        document.getElementById("search-modal-input")?.focus();
    },

    _filteredSearchResults() {
        const q = (this.state.search?.query || "").trim().toLowerCase();
        const all = searchQuickActions(this.state.search?.edition || this.state.edition);
        return q ? all.filter(r => r.label.toLowerCase().includes(q) || (r.sub || "").toLowerCase().includes(q)) : all;
    },

    activateSearchResult(idx) {
        const r = this._filteredSearchResults()[idx];
        if (!r) return;
        this.closeSearch();
        if (r.panel) this.openPanel(r.panel);
        else if (r.action === "launch") { if (this.state.edition === "bedrock") this.launchBedrockGame(); else this.launchJava(); }
        else if (r.action === "xbox") { if (this.state.settings.xboxGamertag) this.signOutMicrosoft(); else this.signInWithMicrosoft(); }
        else if (r.action === "discord") { if (this.state.settings.discordLoggedIn) this.signOutDiscord(); else this.signInWithDiscord(); }
    },

    // ── Notifications bell (real markup: .notif-bell-wrap/.notif-panel) ──
    toggleNotifPanel() {
        const wrap = document.getElementById("notif-bell-wrap");
        const existing = document.getElementById("notif-panel");
        if (existing) { existing.remove(); return; }
        wrap.insertAdjacentHTML("beforeend", notifPanelHtml(this.state.downloads));
    },

    closeNotifPanel() {
        document.getElementById("notif-panel")?.remove();
    },

    updateNotifBadge() {
        const badge = document.getElementById("notif-bell-badge");
        const count = this.state.downloads.filter(d => d.status === "downloading").length;
        badge.style.display = count > 0 ? "" : "none";
        badge.textContent = count > 9 ? "9+" : String(count);
    },

    renderNews() {
        const items = [
            { icon: "fa-solid fa-snowflake", title: "Glacier Launcher", subtitle: "now on Android", url: "https://glacierclient.xyz" },
            { icon: "fa-brands fa-github", title: "Open source", subtitle: "contributions welcome", url: "https://github.com/Glacier-Client-BE/Glacier-Launcher" },
        ];
        const html = items.concat(items).map(i =>
            `<span class="news-item" data-open-url="${i.url}"><i class="${i.icon}"></i><span class="news-title">${i.title}</span><span>${i.subtitle}</span></span>`).join("");
        document.getElementById("news-track").innerHTML = html;
    },

    // ── Panel routing ────────────────────────────────────────────────────
    openPanel(id) {
        this.state.openPanel = id;
        document.getElementById("main-content").classList.add("panel-open");
        const root = document.getElementById("panel-root");

        let html;
        switch (id) {
            case "clients": html = panelShell({ id, title: "Clients", headerActions: `<button class="panel-icon-btn" data-open-panel="mcversions"><i class="fa-solid fa-clock-rotate-left"></i><span>Versions</span></button>`, body: clientsPanelBody(), activeTabId: id }); break;
            case "settings": html = this.settingsPanelHtml("all"); break;
            case "servers": html = panelShell({ id, title: "Servers", headerActions: `<button class="panel-icon-btn"><i class="fa-solid fa-plus"></i><span>Add</span></button>`, body: serversPanelBody(), activeTabId: id }); break;
            case "credits": html = panelShell({ id, title: "Credits", body: creditsPanelBody(), activeTabId: id }); break;
            case "addons": {
                this.state.cfCategory = (this.state.edition === "java" ? CurseForge.javaCategories : CurseForge.bedrockCategories)[0].classId;
                this.state.mrCategory = Modrinth.javaCategories[0].facet;
                const isJava = this.state.edition === "java";
                html = panelShell({
                    id,
                    title: isJava ? "Mods & Addons" : "Addons",
                    headerActions: isJava ? `<button class="btn-sm" data-open-panel="modpacks"><i class="fa-solid fa-cubes"></i> Modpacks</button>` : "",
                    body: addonsPanelBody(),
                    activeTabId: id,
                });
                break;
            }
            case "mcversions": {
                html = mcVersionsPanelHtml(this.state.mcVersionsChannel, this.state.mcVersionsFilter, this.state.mcVersions, this.state.mcVersionsLoading);
                if (!this.state.mcVersionsLoaded && !this.state.mcVersionsLoading) this.loadMcVersions();
                break;
            }
            case "bedrockworlds": {
                const bw = this.state.bedrockWorlds;
                html = panelShell({
                    id,
                    title: "Worlds",
                    headerActions: bw.hasAccess
                        ? `<button class="panel-icon-btn" data-open-bedrock-folder="minecraftWorlds" data-tooltip="Open folder"><i class="fa-solid fa-folder-open"></i></button>
                           <button class="panel-icon-btn ${bw.loading ? "spinning" : ""}" ${bw.loading ? "disabled" : ""} data-refresh-bedrock-worlds data-tooltip="Refresh"><i class="fa-solid fa-rotate"></i></button>`
                        : "",
                    body: bedrockWorldsPanelBody(bw),
                    activeTabId: id,
                });
                if (bw.hasAccess && !bw.loaded && !bw.loading) this.loadBedrockWorlds();
                break;
            }
            case "bedrockpacks": {
                const bp = this.state.bedrockPacks;
                html = panelShell({
                    id,
                    title: "Packs",
                    headerActions: bp.hasAccess
                        ? `<button class="panel-icon-btn" data-open-bedrock-folder="${BEDROCK_PACK_DIR_NAMES[bp.kind]}" data-tooltip="Open folder"><i class="fa-solid fa-folder-open"></i></button>
                           <button class="panel-icon-btn ${bp.loading ? "spinning" : ""}" ${bp.loading ? "disabled" : ""} data-refresh-bedrock-packs data-tooltip="Refresh"><i class="fa-solid fa-rotate"></i></button>`
                        : "",
                    body: bedrockPacksPanelBody(bp),
                    activeTabId: id,
                });
                if (bp.hasAccess && !bp.loaded && !bp.loading) this.loadBedrockPacks();
                break;
            }
            case "bedrockbackups": {
                const bb = this.state.bedrockBackups;
                html = panelShell({
                    id,
                    title: "Backups",
                    headerActions: bb.hasAccess
                        ? `<button class="panel-icon-btn ${bb.loading ? "spinning" : ""}" ${bb.loading ? "disabled" : ""} data-refresh-bedrock-backups data-tooltip="Refresh"><i class="fa-solid fa-rotate"></i></button>`
                        : "",
                    body: bedrockBackupsPanelBody(bb),
                    activeTabId: id,
                });
                if (bb.hasAccess && !bb.loaded && !bb.loading) this.loadBedrockBackups();
                break;
            }
            case "bedrockinstances": html = panelShell({ id, title: "Instances", body: emptyState("No instances yet", "Isolated Bedrock instances will list here once wired to shared storage."), activeTabId: id }); break;
            case "bedrockscreenshots": {
                const bs = this.state.bedrockScreenshots;
                html = panelShell({
                    id,
                    title: "Photos",
                    headerActions: bs.hasAccess
                        ? `<button class="panel-icon-btn" data-open-bedrock-folder="Screenshots" data-tooltip="Open folder"><i class="fa-solid fa-folder-open"></i></button>
                           <button class="panel-icon-btn ${bs.loading ? "spinning" : ""}" ${bs.loading ? "disabled" : ""} data-refresh-bedrock-screenshots data-tooltip="Refresh"><i class="fa-solid fa-rotate"></i></button>`
                        : "",
                    body: bedrockScreenshotsPanelBody(bs),
                    activeTabId: id,
                });
                if (bs.hasAccess && !bs.loaded && !bs.loading) this.loadBedrockScreenshots();
                break;
            }
            case "javaclients": {
                html = `<div class="panel-overlay" id="panel-javaclients">
                    <div class="panel-handle"></div>
                    <div class="panel-header">
                        <div class="panel-title-wrap"><span class="panel-title">Launchers</span><div class="panel-title-underline"></div></div>
                        <div class="panel-header-actions"><button class="panel-back-btn" data-close-panel data-tooltip="Back"><i class="fa-solid fa-chevron-down"></i></button></div>
                    </div>
                    ${javaClientsPanelBody(this.state.glacier)}
                    ${renderPanelTabs("javaclients")}
                </div>`;
                if (!this.state.glacier.latest && !this.state.glacier.loading) this.loadGlacierManifest();
                break;
            }
            case "javaversions": {
                const jv = this.state.javaVersions;
                html = javaVersionsPanelHtml(jv.filter, jv.showSnapshots, jv.showHistorical, jv.list, jv.loading, jv.error);
                if (jv.list.length === 0 && !jv.loading) this.loadJavaVersions();
                break;
            }
            case "javaprofile": {
                this.state.javaInstances.instances = JavaInstances.list();
                html = panelShell({ id, title: "Profile", body: javaProfilePanelBody(), activeTabId: id });
                break;
            }
            case "javascreenshots": html = panelShell({ id, title: "Screenshots", headerActions: `<button class="panel-icon-btn" data-tooltip="Refresh"><i class="fa-solid fa-rotate"></i></button>`, body: javaScreenshotsPanelBody(), activeTabId: id }); break;
            case "news": {
                html = newsPanelHtml(this.state.news);
                if (this.state.news.posts.length === 0 && this.state.news.releases.length === 0 && !this.state.news.loading) this.loadNews();
                break;
            }
            case "downloads": html = downloadsPanelHtml(this.state.downloads); break;
            // Stats/Logs are standalone overlay components on desktop (Components/
            // StatsPanel.razor, LogsPanel.razor) with no .panel-tabs footer at all —
            // not routed through panelShell(), which always appends one.
            case "stats": html = bareOverlayHtml("stats", "Statistics", "", statsPanelBody(this.state.settings.playSessions || [])); break;
            case "logs": html = bareOverlayHtml("logs", "Logs & Crashes", `<button class="btn-sm" data-tooltip="Refresh"><i class="fa-solid fa-rotate"></i> Refresh</button>`, logsPanelBody()); break;
            case "skinlibrary": {
                if (this.state.skinLibrary.skins.length === 0) this.loadSkins();
                html = skinLibraryPanelHtml(this.state.skinLibrary);
                break;
            }
            case "modpacks": {
                // No .panel-tabs footer on desktop (Components/ModpacksPanel.razor) either.
                html = bareOverlayHtml("modpacks", "Modpacks", "", modpacksPanelBody(this.state.modpacks));
                if (!this.state.modpacks.searched && !this.state.modpacks.searching) this.searchModpacks();
                break;
            }
            case "themestudio": {
                if (this.state.selectedThemeId === null && this.state.themes.length > 0) {
                    this.state.selectedThemeId = this.state.themes.find(t => t.id === this.state.settings.activeThemeId)?.id || this.state.themes[0].id;
                }
                const sel = this.state.themes.find(t => t.id === this.state.selectedThemeId) || null;
                const headerActions = `<button class="btn-sm" data-theme-new-header><i class="fa-solid fa-plus"></i> New</button>`;
                // No .panel-tabs footer on desktop (Components/ThemeStudioPanel.razor) either.
                html = bareOverlayHtml("themestudio", "Theme Studio", headerActions, themeStudioPanelBody(this.state.themes, sel, this.state.settings.activeThemeId));
                break;
            }
            default: html = panelShell({ id, title: id, body: emptyState("Coming soon", "This panel is queued — see android/README.md's status list."), activeTabId: id });
        }
        root.innerHTML = html;

        if (id === "addons") {
            const activeTab = this.state.edition === "java" ? this.state.javaModsTab : "curseforge";
            if (activeTab === "curseforge") this.runCfSearch(true);
            if (activeTab === "modrinth") this.runMrSearch(true);
        }
        if (id === "settings") this.bindSettingsEvents();
    },

    // Not routed through panelShell(): desktop's Settings panel puts the
    // category switcher as a sibling of .panel-body (not nested inside it,
    // per Pages/Home.razor), and the body itself carries a real
    // ".settings-body" class alongside ".panel-body" — app.css's
    // ".panel-body.settings-body { gap: 12px; }" compound-class rule (and
    // its padding) never applied when this was previously built as one
    // panelShell() body with only an id="settings-body" (ids don't match
    // class selectors), which is what left the panel looking unpadded.
    settingsPanelHtml(category) {
        return `
        <div class="panel-overlay" id="panel-settings">
            <div class="panel-handle"></div>
            <div class="panel-header">
                <div class="panel-title-wrap"><span class="panel-title">Settings</span><div class="panel-title-underline"></div></div>
                <div class="panel-header-actions"><button class="panel-back-btn" data-close-panel data-tooltip="Back"><i class="fa-solid fa-chevron-down"></i></button></div>
            </div>
            <div class="versions-client-switcher" id="settings-categories">${SETTINGS_CATEGORIES(this.state.edition).map(c =>
                `<button class="vcs-btn ${c.id === category ? "active" : ""}" data-settings-category="${c.id}"><i class="${c.icon}"></i> ${c.label}</button>`).join("")}
            </div>
            <div class="panel-body settings-body" id="settings-body">${settingsPanelBody(category)}</div>
            ${renderPanelTabs("settings")}
        </div>`;
    },

    bindSettingsEvents() {
        const body = document.getElementById("settings-body");
        if (!body) return;
        const s = this.state.settings;

        body.querySelectorAll("[data-toggle-setting]").forEach(el => el.addEventListener("click", () => {
            const key = el.dataset.toggleSetting;
            s[key] = !s[key];
            // Rich Presence lives natively (DiscordRpcService.kt), so the
            // flag alone does nothing — the native side has to connect or
            // disconnect. Passing "" keeps the stored token.
            if (key === "discordRichPresence") this.applyDiscordRpcSetting();
            this.saveSettings();
            this.openPanel("settings"); // re-render with current category preserved via active tab class already set server-side isn't tracked; acceptable re-open to "all"
        }));
        body.querySelectorAll("[data-set-accent]").forEach(el => el.addEventListener("click", () => {
            s.accentColor = el.dataset.setAccent;
            document.documentElement.style.setProperty("--accent", s.accentColor);
            this.saveSettings();
            this.openPanel("settings");
        }));
        const usernameInput = document.getElementById("setting-username");
        if (usernameInput) usernameInput.addEventListener("change", (e) => { s.username = e.target.value; this.saveSettings(); this.renderFooter(); });
        const discordToken = document.getElementById("setting-discord-token");
        if (discordToken) discordToken.addEventListener("change", (e) => {
            const token = e.target.value.trim();
            if (!token) return;                 // empty submit keeps the stored token
            e.target.value = "";                // never leave credentials sitting in the DOM
            Bridge.discordRpcConfigure(!!s.discordRichPresence, token);
            e.target.placeholder = "Token saved — type to replace";
        });
        const cfKeyInput = document.getElementById("setting-cf-key");
        if (cfKeyInput) cfKeyInput.addEventListener("change", (e) => { s.curseForgeApiKeyOverride = e.target.value; this.saveSettings(); });
        const openJava = document.getElementById("open-java-edition");
        if (openJava) openJava.addEventListener("click", () => this.launchJava());
        const resetBtn = document.getElementById("reset-settings");
        if (resetBtn) resetBtn.addEventListener("click", () => { this.state.settings = autoSavingSettings({ ...DEFAULT_SETTINGS }, () => this.saveSettings()); this.saveSettings(); this.openPanel("settings"); this.renderFooter(); this.renderHome(); });
        const clearHistory = document.getElementById("clear-recent-history");
        if (clearHistory) clearHistory.addEventListener("click", () => { s.recentlyLaunched = []; this.saveSettings(); this.renderHome(); });

        document.querySelectorAll("[data-settings-category]").forEach(el => el.addEventListener("click", () => {
            const category = el.dataset.settingsCategory;
            document.getElementById("settings-categories").innerHTML = SETTINGS_CATEGORIES(this.state.edition).map(c =>
                `<button class="vcs-btn ${c.id === category ? "active" : ""}" data-settings-category="${c.id}"><i class="${c.icon}"></i> ${c.label}</button>`).join("");
            document.getElementById("settings-body").innerHTML = settingsPanelBody(category);
            this.bindSettingsEvents();
        }));
    },

    // ── Generic paginated content search ─────────────────────────────────
    // CurseForge and Modrinth search were two near-identical copies of the
    // same fetch/render/load-more/error-handling logic, differing only in
    // which API to call and how to read a result item's title/summary. One
    // reusable engine, driven by a small per-source config object, replaces
    // both — search sources beyond these two only need a config, not a
    // fresh copy of this logic.
    async runPagedSearch(cfg) {
        const resultsEl = document.getElementById(cfg.resultsId);
        if (!resultsEl) return;
        resultsEl.innerHTML = `<div style="padding:24px;text-align:center;"><span class="spinner"></span></div>`;
        const query = document.getElementById(cfg.inputId)?.value || "";
        const offset = cfg.reset ? 0 : this.state[cfg.resultsKey].length;
        try {
            const page = await cfg.fetchPage(query, offset);
            this.state[cfg.resultsKey] = cfg.reset ? page.items : this.state[cfg.resultsKey].concat(page.items);
            this.state[cfg.totalKey] = page.total ?? this.state[cfg.resultsKey].length;
        } catch (e) {
            resultsEl.innerHTML = `<div style="padding:20px;text-align:center;"><span class="error-text">Failed to reach ${cfg.sourceName}: ${e.message}</span></div>`;
            return;
        }
        this.renderPagedResults(cfg);
    },

    renderPagedResults(cfg) {
        const resultsEl = document.getElementById(cfg.resultsId);
        if (!resultsEl) return;
        const results = this.state[cfg.resultsKey];
        const total = this.state[cfg.totalKey];
        if (results.length === 0) {
            resultsEl.innerHTML = emptyState("No results", "Try a different search term or category.", "fa-solid fa-magnifying-glass");
            return;
        }
        const loadMoreId = `${cfg.resultsId}-load-more`;
        resultsEl.innerHTML = results.map(item => contentResultRowHtml(cfg.mapResult(item))).join("") +
            (results.length < total
                ? `<button class="btn-sm" id="${loadMoreId}" style="width:100%;margin-top:8px;"><i class="fa-solid fa-angles-down"></i> Load more (${results.length} / ${total})</button>`
                : "");
        document.getElementById(loadMoreId)?.addEventListener("click", () => this.runPagedSearch({ ...cfg, reset: false }));
    },

    runCfSearch(reset) {
        const gameId = this.state.edition === "java" ? CurseForge.GAME_ID_JAVA : CurseForge.GAME_ID_BEDROCK;
        this.runPagedSearch({
            reset, resultsId: "cf-results", inputId: "cf-search-input",
            resultsKey: "cfResults", totalKey: "cfTotalCount", sourceName: "CurseForge",
            fetchPage: async (query, offset) => {
                const res = await CurseForge.search(gameId, this.state.cfCategory, query, offset);
                return { items: res.data, total: res.pagination ? res.pagination.totalCount : res.data.length };
            },
            mapResult: mod => ({ name: mod.name, summary: mod.summary }),
        });
    },

    runMrSearch(reset) {
        this.runPagedSearch({
            reset, resultsId: "mr-results", inputId: "mr-search-input",
            resultsKey: "mrResults", totalKey: "mrTotalCount", sourceName: "Modrinth",
            fetchPage: async (query, offset) => {
                const res = await Modrinth.search(this.state.mrCategory, query, offset);
                return { items: res.hits, total: res.total_hits };
            },
            mapResult: project => ({ name: project.title, summary: project.description }),
        });
    },

    async loadGlacierManifest() {
        this.state.glacier = { loading: true, latest: null, error: null };
        if (this.state.openPanel === "javaclients") this.openPanel("javaclients");
        try {
            const manifest = await GlacierClient.fetchManifest();
            const latestId = manifest.latestRelease;
            const version = (manifest.versions || []).find(v => v.id === latestId) || (manifest.versions || [])[0];
            this.state.glacier = {
                loading: false,
                error: null,
                latest: version ? {
                    name: version.name || version.id,
                    loader: version.fabric ? "Fabric" : version.forge ? "Forge" : "Unknown",
                    installed: false, // no local jar-tracking on Android yet — see JavaEditionBridge
                } : null,
            };
        } catch (e) {
            this.state.glacier = { loading: false, latest: null, error: `Failed to fetch manifest: ${e.message}` };
        }
        if (this.state.openPanel === "javaclients") this.openPanel("javaclients");
    },

    async loadJavaVersions() {
        this.state.javaVersions.loading = true;
        this.state.javaVersions.error = null;
        if (this.state.openPanel === "javaversions") this.openPanel("javaversions");
        try {
            const manifest = await MojangVersions.fetchManifest();
            // No installed/active version tracking on Android yet (see
            // JavaEditionBridge) — the desktop app marks the user's actually
            // active version here; we don't have one to mark truthfully.
            this.state.javaVersions.list = manifest.versions.map(v => ({
                id: v.id,
                type: v.type,
                typeLabel: MojangVersions.typeLabel(v.type),
                active: false,
            }));
        } catch (e) {
            this.state.javaVersions.error = `Failed to fetch Mojang manifest: ${e.message}`;
        }
        this.state.javaVersions.loading = false;
        if (this.state.openPanel === "javaversions") this.openPanel("javaversions");
    },

    async loadNews() {
        this.state.news.loading = true;
        if (this.state.openPanel === "news") this.openPanel("news");
        const [postsResult, releasesResult] = await Promise.allSettled([NewsFeed.fetchPosts(), NewsFeed.fetchReleases()]);
        this.state.news.posts = postsResult.status === "fulfilled" ? postsResult.value : [];
        this.state.news.releases = releasesResult.status === "fulfilled" ? releasesResult.value : [];
        this.state.news.loading = false;
        if (this.state.openPanel === "news") this.openPanel("news");
    },

    async searchModpacks() {
        const m = this.state.modpacks;
        m.searching = true;
        m.searched = true;
        m.error = null;
        if (this.state.openPanel === "modpacks") this.openPanel("modpacks");
        try {
            if (m.source === "cf") {
                if (!CurseForge.isAvailable()) {
                    m.results = [];
                } else {
                    const res = await CurseForge.search(CurseForge.GAME_ID_JAVA, CurseForge.JAVA_CLASS_MODPACKS, m.query);
                    m.results = res.data.map(a => ({ id: a.id, source: "cf", title: a.name, author: "", summary: a.summary, icon: a.logo?.thumbnailUrl || "", downloads: a.downloadCount }));
                }
            } else {
                const res = await Modrinth.search("modpack", m.query);
                m.results = res.hits.map(p => ({ id: p.project_id, source: "mr", title: p.title, author: p.author || "", summary: p.description, icon: p.icon_url || "", downloads: p.downloads }));
            }
        } catch (e) {
            m.error = e.message;
        }
        m.searching = false;
        if (this.state.openPanel === "modpacks") this.openPanel("modpacks");
    },

    async installModpack(projectId, packName) {
        const m = this.state.modpacks;
        if (m.installingId) return;
        m.installingId = projectId;
        if (this.state.openPanel === "modpacks") this.openPanel("modpacks");

        const result = await ModpackInstall.installModrinth(projectId, packName);
        m.installingId = null;
        if (this.state.openPanel === "modpacks") this.openPanel("modpacks");

        if (!result.success) {
            alert(result.message);
            return;
        }
        this.state.javaInstances.instances = JavaInstances.list();
        const loaderNote = result.loader
            ? `\n\nThis pack uses ${result.loader} — add that loader profile from the MC Versions panel before launching, mods won't load on vanilla.`
            : "";
        const failedNote = result.failedFiles > 0 ? `\n\n${result.failedFiles} file(s) failed to download — check Profile > Instances.` : "";
        alert(`Installed "${result.instanceName}" (${result.downloadedFiles} files).${loaderNote}${failedNote}`);
    },

    closePanel() {
        this.state.openPanel = null;
        document.getElementById("main-content").classList.remove("panel-open");
        document.getElementById("panel-root").innerHTML = "";
    },

    // ── Global click delegation ──────────────────────────────────────────
    bindGlobalEvents() {
        document.getElementById("edition-bedrock").addEventListener("click", () => this.setEdition("bedrock"));
        document.getElementById("edition-java").addEventListener("click", () => this.setEdition("java"));
        document.getElementById("rpc-toggle").addEventListener("click", () => {
            this.state.settings.discordRichPresence = !this.state.settings.discordRichPresence;
            // Same as the Settings toggle: the flag is only the UI half,
            // the native service has to connect/disconnect (DiscordRpcService.kt).
            this.applyDiscordRpcSetting();
            this.saveSettings();
            this.renderFooter();
        });

        document.getElementById("update-pill").addEventListener("click", () => {
            if (this.state.update.available) this.openUpdateModal();
            else this.checkForUpdate(true);
        });

        document.getElementById("search-trigger-btn").addEventListener("click", () => this.openSearch());
        document.getElementById("notif-bell-btn").addEventListener("click", () => this.toggleNotifPanel());
        document.getElementById("app-close-btn").addEventListener("click", () => Bridge.closeApp());

        document.body.addEventListener("click", (e) => {
            const openBtn = e.target.closest("[data-open-panel]");
            if (openBtn) { this.openPanel(openBtn.dataset.openPanel); return; }

            if (e.target.closest("#profile-signin-btn")) { this.signInWithMicrosoft(); return; }
            if (e.target.closest("#profile-signout-btn")) { this.signOutMicrosoft(); return; }
            if (e.target.closest("#xbox-connect-btn")) {
                if (this.state.settings.xboxGamertag) this.signOutMicrosoft();
                else this.signInWithMicrosoft();
                return;
            }
            if (e.target.closest("#discord-connect-btn")) {
                if (this.state.settings.discordLoggedIn) this.signOutDiscord();
                else this.signInWithDiscord();
                return;
            }
            const newInstanceBtn = e.target.closest("[data-new-instance]");
            if (newInstanceBtn) { this.newJavaInstance(); return; }
            const switchInstanceBtn = e.target.closest("[data-switch-instance]");
            if (switchInstanceBtn) { this.switchJavaInstance(switchInstanceBtn.dataset.switchInstance); return; }
            const renameInstanceBtn = e.target.closest("[data-rename-instance]");
            if (renameInstanceBtn) { this.beginRenameInstance(renameInstanceBtn.dataset.renameInstance, renameInstanceBtn.dataset.renameInstanceName); return; }
            const commitRenameBtn = e.target.closest("[data-commit-rename-instance]");
            if (commitRenameBtn) { this.commitRenameInstance(commitRenameBtn.dataset.commitRenameInstance); return; }
            const confirmDeleteInstanceBtn = e.target.closest("[data-confirm-delete-instance]");
            if (confirmDeleteInstanceBtn) { this.confirmDeleteInstance(confirmDeleteInstanceBtn.dataset.confirmDeleteInstance); return; }
            if (e.target.closest("[data-cancel-delete-instance]")) { this.cancelDeleteInstance(); return; }
            const deleteInstanceBtn = e.target.closest("[data-delete-instance]");
            if (deleteInstanceBtn) { this.deleteJavaInstance(deleteInstanceBtn.dataset.deleteInstance); return; }

            const installModpackBtn = e.target.closest("[data-install-modpack]");
            if (installModpackBtn) { this.installModpack(installModpackBtn.dataset.installModpack, installModpackBtn.dataset.installModpackName); return; }
            if (e.target.closest("[data-pick-custom-dll]")) { this.pickCustomDll(); return; }
            if (e.target.closest("[data-clear-custom-dll]")) { this.clearCustomDll(); return; }
            if (e.target.closest("[data-stage-custom-dll]")) { this.stageCustomDllInjection(); return; }
            if (e.target.closest("[data-grant-bedrock-storage]")) { this.requestBedrockStorageAccess(); return; }
            const openFolderBtn = e.target.closest("[data-open-bedrock-folder]");
            if (openFolderBtn) { BedrockStorage.openFolder(openFolderBtn.dataset.openBedrockFolder); return; }
            if (e.target.closest("[data-refresh-bedrock-worlds]")) { this.loadBedrockWorlds(); return; }
            if (e.target.closest("[data-refresh-bedrock-packs]")) { this.loadBedrockPacks(); return; }
            const packKindBtn = e.target.closest("[data-bedrock-pack-kind]");
            if (packKindBtn) { this.switchBedrockPackKind(packKindBtn.dataset.bedrockPackKind); return; }
            if (e.target.closest("[data-refresh-bedrock-backups]")) { this.loadBedrockBackups(); return; }
            if (e.target.closest("[data-refresh-bedrock-screenshots]")) { this.loadBedrockScreenshots(); return; }
            if (e.target.closest("[data-create-bedrock-backup]")) { this.createBedrockBackup(); return; }
            const confirmDeleteBackupBtn = e.target.closest("[data-confirm-delete-bedrock-backup]");
            if (confirmDeleteBackupBtn) { this.confirmDeleteBedrockBackup(confirmDeleteBackupBtn.dataset.confirmDeleteBedrockBackup); return; }
            if (e.target.closest("[data-cancel-delete-bedrock-backup]")) { this.cancelDeleteBedrockBackup(); return; }
            const deleteBackupBtn = e.target.closest("[data-delete-bedrock-backup]");
            if (deleteBackupBtn) { this.deleteBedrockBackup(deleteBackupBtn.dataset.deleteBedrockBackup); return; }
            const onboardingEditionBtn = e.target.closest("[data-onboarding-pick-edition]");
            if (onboardingEditionBtn) { this.onboardingPickEdition(onboardingEditionBtn.dataset.onboardingPickEdition); return; }
            if (e.target.closest("[data-onboarding-next]")) { this.onboardingNext(); return; }
            if (e.target.closest("[data-onboarding-back]")) { this.onboardingBack(); return; }
            if (e.target.closest("[data-onboarding-finish]")) { this.finishOnboarding(); return; }
            if (e.target.closest("[data-onboarding-skip]")) { this.skipOnboarding(); return; }
            const dismissAnnouncementBtn = e.target.closest("[data-dismiss-announcement]");
            if (dismissAnnouncementBtn) { this.dismissAnnouncement(dismissAnnouncementBtn.dataset.dismissAnnouncement); return; }
            if (e.target.closest("[data-skip-update]")) { this.skipUpdate(); return; }
            if (e.target.closest("[data-install-update]")) { this.installUpdate(); return; }
            if (e.target.closest("[data-close-update]")) { this.closeUpdateModal(); return; }
            if (e.target.matches("[data-close-update-backdrop]")) { this.closeUpdateModal(); return; }

            if (e.target.closest("#footer-profile") && !e.target.closest("#rpc-toggle")) {
                const which = this.effectiveProfile();
                if (which === "discord") { if (this.state.settings.discordLoggedIn) this.signOutDiscord(); }
                else if (which === "xbox") { if (this.state.settings.xboxGamertag) this.signOutMicrosoft(); }
                return;
            }

            if (e.target.closest("#search-overlay-backdrop") && !e.target.closest(".search-modal")) { this.closeSearch(); return; }
            const searchResult = e.target.closest("[data-search-idx]");
            if (searchResult) { this.activateSearchResult(Number(searchResult.dataset.searchIdx)); return; }

            if (!e.target.closest("#notif-bell-wrap")) this.closeNotifPanel();

            const closeBtn = e.target.closest("[data-close-panel]");
            if (closeBtn) { this.closePanel(); return; }

            const launchBtn = e.target.closest("#launch-btn");
            if (launchBtn) {
                if (this.state.edition === "bedrock") this.launchBedrockGame(); else this.launchJava();
                return;
            }

            const urlBtn = e.target.closest("[data-open-url]");
            if (urlBtn) { Bridge.openUrl(urlBtn.dataset.openUrl); return; }

            const cfCat = e.target.closest("[data-cf-category]");
            if (cfCat) {
                document.querySelectorAll("#cf-categories .vcs-btn").forEach(b => b.classList.remove("active"));
                cfCat.classList.add("active");
                this.state.cfCategory = Number(cfCat.dataset.cfCategory);
                this.runCfSearch(true);
                return;
            }

            const saveServer = e.target.closest("[data-save-server]");
            if (saveServer) {
                const server = POPULAR_SERVERS.find(s => s.address === saveServer.dataset.saveServer);
                if (server) {
                    this.state.settings.savedServers = (this.state.settings.savedServers || []).concat([{ name: server.name, address: server.address, port: server.port }]);
                    this.saveSettings();
                    this.openPanel("servers");
                }
                return;
            }
            const deleteServer = e.target.closest("[data-delete-server]");
            if (deleteServer) {
                this.state.settings.savedServers = (this.state.settings.savedServers || []).filter(s => s.address !== deleteServer.dataset.deleteServer);
                this.saveSettings();
                this.openPanel("servers");
                return;
            }

            const mcvChannel = e.target.closest("[data-mcv-channel]");
            if (mcvChannel) {
                this.state.mcVersionsChannel = mcvChannel.dataset.mcvChannel;
                this.openPanel("mcversions");
                return;
            }

            const javaModsTab = e.target.closest("[data-java-mods-tab]");
            if (javaModsTab) {
                this.state.javaModsTab = javaModsTab.dataset.javaModsTab;
                this.openPanel("addons");
                return;
            }

            const mrCat = e.target.closest("[data-mr-category]");
            if (mrCat) {
                document.querySelectorAll("#mr-categories .vcs-btn").forEach(b => b.classList.remove("active"));
                mrCat.classList.add("active");
                this.state.mrCategory = mrCat.dataset.mrCategory;
                this.runMrSearch(true);
                return;
            }

            if (e.target.closest("[data-glacier-retry]") || e.target.closest("[data-glacier-install]")) { this.loadGlacierManifest(); return; }
            if (e.target.closest("[data-glacier-launch]") || e.target.closest("[data-open-java-edition]")) { this.launchJava(); return; }
            const launchJavaVer = e.target.closest("[data-launch-java-version]");
            if (e.target.closest("#theme-pick-wallpaper")) { ThemeEngine.pickWallpaper(); return; }
            if (e.target.closest("#theme-reset-wallpaper")) { ThemeEngine.clearWallpaper(); return; }

            if (launchJavaVer) {
                const versionId = launchJavaVer.dataset.launchJavaVersion;
                this.recordLaunchStart();
                Bridge.discordRpcSetJava(versionId, "");
                Bridge.launchJavaEditionVersion(versionId);
                return;
            }
            if (e.target.closest("[data-glacier-uninstall]")) { this.state.glacier.latest.installed = false; this.openPanel("javaclients"); return; }

            if (e.target.closest("[data-toggle-java-snapshots]")) {
                this.state.javaVersions.showSnapshots = !this.state.javaVersions.showSnapshots;
                this.openPanel("javaversions");
                return;
            }
            if (e.target.closest("[data-toggle-java-historical]")) {
                this.state.javaVersions.showHistorical = !this.state.javaVersions.showHistorical;
                this.openPanel("javaversions");
                return;
            }
            if (e.target.closest("[data-refresh-java-versions]")) { this.state.javaVersions.list = []; this.loadJavaVersions(); return; }
            if (e.target.closest("[data-refresh-news]")) { this.loadNews(); return; }

            const editLevelDat = e.target.closest("[data-edit-leveldat]");
            if (editLevelDat) { this.openLevelDatEditor(editLevelDat.dataset.editLeveldat); return; }
            if (e.target.closest("[data-leveldat-close]")) { this.closeLevelDatEditor(); return; }
            if (e.target.closest("[data-leveldat-save]")) { this.saveLevelDat(); return; }
            if (e.target.closest("[data-leveldat-cheats]")) {
                // Optional-chained: the argument is evaluated before
                // setLevelDatField's own null guard can run, so a click
                // landing on a stale node would throw here otherwise.
                this.setLevelDatField("cheats", !this.state.bedrockWorlds.levelDat?.cheats); return;
            }
            const ldGameType = e.target.closest("[data-leveldat-gametype]");
            if (ldGameType) { this.setLevelDatField("gameType", Number(ldGameType.dataset.leveldatGametype)); return; }
            const ldDifficulty = e.target.closest("[data-leveldat-difficulty]");
            if (ldDifficulty) { this.setLevelDatField("difficulty", Number(ldDifficulty.dataset.leveldatDifficulty)); return; }
            const ldExperiment = e.target.closest("[data-leveldat-experiment]");
            if (ldExperiment) { this.toggleLevelDatExperiment(ldExperiment.dataset.leveldatExperiment); return; }

            const javaTool = e.target.closest("[data-java-tool]");
            if (javaTool) { this.runJavaTool(javaTool.dataset.javaTool); return; }

            if (e.target.closest("#skinlib-add-btn")) { this.addSkinFromUsername(); return; }
            if (e.target.closest("#skinlib-add-png")) { SkinLibrary.addFromPng(); return; }
            if (e.target.closest("#skinlib-save-current")) {
                if (this.state.settings.javaUsername) {
                    this.state.skinLibrary.username = this.state.settings.javaUsername;
                    this.addSkinFromUsername();
                }
                return;
            }
            const slApply = e.target.closest("[data-skinlib-apply]");
            const slApplyAlt = e.target.closest("[data-skinlib-apply-alt]");
            if (slApply || slApplyAlt) {
                const id = (slApply || slApplyAlt).dataset.skinlibApply || (slApply || slApplyAlt).dataset.skinlibApplyAlt;
                const skin = this.state.skinLibrary.skins.find(s => s.id === id);
                if (skin) this.applySkinFromLibrary(skin, slApplyAlt ? !skin.slim : skin.slim);
                return;
            }
            const slDelete = e.target.closest("[data-skinlib-delete]");
            if (slDelete) {
                const id = slDelete.dataset.skinlibDelete;
                this.state.skinLibrary.skins = this.state.skinLibrary.skins.filter(s => s.id !== id);
                this.saveSkins();
                this.renderSkinLibraryBody();
                return;
            }

            const removeDl = e.target.closest("[data-remove-download]");
            if (removeDl) {
                this.state.downloads = this.state.downloads.filter(d => d.id !== removeDl.dataset.removeDownload);
                this.openPanel("downloads");
                return;
            }
            if (e.target.closest("[data-clear-finished-downloads]")) {
                this.state.downloads = this.state.downloads.filter(d => d.status === "downloading");
                this.openPanel("downloads");
                return;
            }

            const modpackSource = e.target.closest("[data-modpack-source]");
            if (modpackSource) {
                const source = modpackSource.dataset.modpackSource;
                if (this.state.modpacks.source !== source) {
                    this.state.modpacks.source = source;
                    this.searchModpacks();
                }
                return;
            }
            if (e.target.closest("[data-modpack-search]")) {
                this.state.modpacks.query = document.getElementById("modpack-search-input")?.value || "";
                this.searchModpacks();
                return;
            }

            // ── Theme Studio ──────────────────────────────────────────
            if (e.target.closest("[data-theme-new]") || e.target.closest("[data-theme-new-header]")) {
                const t = newThemeFrom(this.state.settings.themePreset, this.state.settings.accentColor);
                this.state.themes = [...this.state.themes, t];
                this.state.selectedThemeId = t.id;
                this.saveThemes();
                ThemeEngine.apply(t);
                this.openPanel("themestudio");
                return;
            }
            const themeSelect = e.target.closest("[data-theme-select]");
            if (themeSelect) {
                this.state.selectedThemeId = themeSelect.dataset.themeSelect;
                const t = this.state.themes.find(x => x.id === this.state.selectedThemeId);
                if (t) ThemeEngine.apply(t);
                this.openPanel("themestudio");
                return;
            }
            if (e.target.closest("[data-theme-activate]")) {
                this.state.settings.activeThemeId = this.state.selectedThemeId;
                this.saveSettings();
                this.openPanel("themestudio");
                return;
            }
            if (e.target.closest("[data-theme-deactivate]")) {
                this.state.settings.activeThemeId = "";
                this.saveSettings();
                this.openPanel("themestudio");
                return;
            }
            if (e.target.closest("[data-theme-duplicate]")) {
                const src = this.state.themes.find(t => t.id === this.state.selectedThemeId);
                if (src) {
                    const copy = { ...src, id: `theme-${Date.now()}`, name: `${src.name} Copy` };
                    this.state.themes = [...this.state.themes, copy];
                    this.state.selectedThemeId = copy.id;
                    this.saveThemes();
                    this.openPanel("themestudio");
                }
                return;
            }
            if (e.target.closest("[data-theme-delete]")) {
                const wasActive = this.state.selectedThemeId === this.state.settings.activeThemeId;
                this.state.themes = this.state.themes.filter(t => t.id !== this.state.selectedThemeId);
                this.state.selectedThemeId = this.state.themes[0]?.id || null;
                this.saveThemes();
                if (wasActive) { this.state.settings.activeThemeId = ""; this.saveSettings(); ThemeEngine.clear(); }
                this.openPanel("themestudio");
                return;
            }
            if (e.target.closest("[data-theme-reseed]")) {
                const t = this.state.themes.find(x => x.id === this.state.selectedThemeId);
                if (t) {
                    const seed = PRESET_SEEDS.find(p => p.preset === t.basePreset) || PRESET_SEEDS[0];
                    Object.assign(t, { bg: seed.bg, bgPanel: seed.bgPanel, text: seed.text, textDim: seed.textDim });
                    this.saveThemes();
                    ThemeEngine.apply(t);
                    this.openPanel("themestudio");
                }
                return;
            }
            const colorCell = e.target.closest("[data-theme-color-key]");
            if (colorCell && !e.target.closest("[data-theme-color-input]")) {
                colorCell.querySelector("[data-theme-color-input]")?.click();
                return;
            }
            if (e.target.closest("[data-theme-apply-css]")) {
                const t = this.state.themes.find(x => x.id === this.state.selectedThemeId);
                if (t) {
                    t.customCss = document.getElementById("theme-custom-css")?.value || "";
                    this.saveThemes();
                    ThemeEngine.setCustomCss(t.customCss);
                }
                return;
            }
            if (e.target.closest("[data-theme-reset-css]")) {
                const t = this.state.themes.find(x => x.id === this.state.selectedThemeId);
                if (t) {
                    t.customCss = "";
                    this.saveThemes();
                    ThemeEngine.setCustomCss("");
                    this.openPanel("themestudio");
                }
                return;
            }
        });

        let cfDebounce;
        let mrDebounce;
        let mcvFilterDebounce;
        let javaVersionFilterDebounce;
        let themeSaveDebounce;
        document.body.addEventListener("keydown", (e) => {
            if (e.target.id === "modpack-search-input" && e.key === "Enter") {
                this.state.modpacks.query = e.target.value;
                this.searchModpacks();
            }
            if (e.target.id === "skinlib-username-input" && e.key === "Enter") {
                this.state.skinLibrary.username = e.target.value;
                this.addSkinFromUsername();
            }
            if (this.state.search) {
                if (e.key === "Escape") { this.closeSearch(); return; }
                const results = this._filteredSearchResults();
                if (e.key === "ArrowDown") {
                    e.preventDefault();
                    this.state.search.selIdx = Math.min(results.length - 1, this.state.search.selIdx + 1);
                    this.renderSearch();
                } else if (e.key === "ArrowUp") {
                    e.preventDefault();
                    this.state.search.selIdx = Math.max(0, this.state.search.selIdx - 1);
                    this.renderSearch();
                } else if (e.key === "Enter" && e.target.id === "search-modal-input") {
                    this.activateSearchResult(this.state.search.selIdx);
                }
            }
        });

        document.body.addEventListener("input", (e) => {
            if (e.target.id === "skinlib-username-input") {
                this.state.skinLibrary.username = e.target.value;
            }
            if (e.target.id === "search-modal-input") {
                this.state.search.query = e.target.value;
                this.state.search.selIdx = 0;
                this.renderSearch();
            }
            if (e.target.id === "cf-search-input") {
                clearTimeout(cfDebounce);
                cfDebounce = setTimeout(() => this.runCfSearch(true), 350);
            }
            if (e.target.id === "mr-search-input") {
                clearTimeout(mrDebounce);
                mrDebounce = setTimeout(() => this.runMrSearch(true), 350);
            }
            if (e.target.id === "java-version-filter-input") {
                clearTimeout(javaVersionFilterDebounce);
                const value = e.target.value;
                javaVersionFilterDebounce = setTimeout(() => {
                    this.state.javaVersions.filter = value;
                    this.openPanel("javaversions");
                    document.getElementById("java-version-filter-input")?.focus();
                }, 250);
            }
            if (e.target.id === "mcv-filter-input") {
                clearTimeout(mcvFilterDebounce);
                const value = e.target.value;
                mcvFilterDebounce = setTimeout(() => {
                    this.state.mcVersionsFilter = value;
                    this.openPanel("mcversions");
                    document.getElementById("mcv-filter-input")?.focus();
                }, 250);
            }

            // ── Theme Studio: live-apply sliders/fields without a full re-render
            // (dragging a slider is 60fps; re-rendering the whole panel isn't) ──
            const t = this.state.themes.find(x => x.id === this.state.selectedThemeId);
            if (!t) return;
            const applyAndSave = () => { ThemeEngine.apply(t); clearTimeout(themeSaveDebounce); themeSaveDebounce = setTimeout(() => this.saveThemes(), 500); };

            if (e.target.id === "theme-name-input") { t.name = e.target.value; clearTimeout(themeSaveDebounce); themeSaveDebounce = setTimeout(() => this.saveThemes(), 500); }
            if (e.target.id === "theme-font-input") { t.fontFamily = e.target.value; applyAndSave(); }
            if (e.target.id === "theme-custom-css") { /* applied via the explicit Apply button, not live */ }
            if (e.target.id === "theme-radius-sm") { t.radiusSm = Number(e.target.value); e.target.nextElementSibling.textContent = `${t.radiusSm}px`; applyAndSave(); }
            if (e.target.id === "theme-radius-md") { t.radiusMd = Number(e.target.value); e.target.nextElementSibling.textContent = `${t.radiusMd}px`; applyAndSave(); }
            if (e.target.id === "theme-blur") { t.blur = Number(e.target.value); e.target.nextElementSibling.textContent = `${t.blur}px`; applyAndSave(); }
            if (e.target.id === "theme-overlay") { t.overlayStrength = Number(e.target.value) / 100; e.target.nextElementSibling.textContent = `${Number(e.target.value)}%`; applyAndSave(); }
            if (e.target.id === "theme-anim-speed") { t.animationSpeed = Number(e.target.value); e.target.nextElementSibling.textContent = `${t.animationSpeed}x`; applyAndSave(); }
        });

        document.body.addEventListener("change", (e) => {
            const colorInput = e.target.closest("[data-theme-color-input]");
            if (colorInput) {
                const t = this.state.themes.find(x => x.id === this.state.selectedThemeId);
                if (t) {
                    t[colorInput.dataset.themeColorInput] = colorInput.value;
                    ThemeEngine.apply(t);
                    this.saveThemes();
                    this.openPanel("themestudio");
                }
                return;
            }
            if (e.target.id === "theme-base-preset-select") {
                const t = this.state.themes.find(x => x.id === this.state.selectedThemeId);
                if (t) {
                    t.basePreset = e.target.value;
                    ThemeEngine.apply(t);
                    this.saveThemes();
                }
                return;
            }
        });
    },
};

// See js/xboxauth.js's window.MicrosoftAuth comment — a top-level `const`
// doesn't attach to `window`, but MainActivity.kt's onResume() calls back
// via window.App.onResumeFromGame() for the Stats session-timer, which
// needs an explicit assignment.
window.App = App;

document.addEventListener("DOMContentLoaded", () => App.init());
