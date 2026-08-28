// App controller: state, quick-actions/home/footer rendering, panel routing,
// and event delegation. Structure mirrors Pages/Home.razor.cs + Home.Panels.cs
// (currentView string, OpenXxx() methods) translated to plain JS.

const DEFAULT_SETTINGS = {
    selectedClient: "Latite Client",
    discordRichPresence: true,
    username: "",
    autoInject: false,
    injectionDelayMs: 2000,
    closeAfterLaunch: false,
    accentColor: "#7289da",
    themePreset: "dark",
    compactMode: false,
    animationsEnabled: true,
    showRecentlyLaunched: true,
    recentlyLaunched: [],
    checkUpdatesOnStartup: true,
    xboxGamertag: "",
    profileDisplayMode: "auto",
    savedServers: [],
    curseForgeApiKeyOverride: "",
};

const App = {
    state: {
        edition: "bedrock", // "bedrock" | "java" — real toggle from Home.razor's .edition-switcher
        settings: { ...DEFAULT_SETTINGS },
        clients: {
            flarial: { downloaded: false, downloading: false, upToDate: true, progress: 0, error: "" },
            oderso: { downloaded: false, downloading: false, upToDate: true, progress: 0, error: "" },
            levilamina: { downloaded: false, downloading: false, upToDate: true, progress: 0, error: "" },
        },
        openPanel: null,
        javaInstalled: false,
        cfCategory: null,
        cfResults: [],
        cfTotalCount: 0,
    },

    init() {
        try { this.state.settings = { ...DEFAULT_SETTINGS, ...JSON.parse(Bridge.getSettingsJson() || "{}") }; } catch (e) {}
        this.state.javaInstalled = Bridge.isJavaEditionInstalled();
        document.getElementById("version-pill").textContent = `v${Bridge.appVersionName()}`;
        this.renderTopBar();
        this.renderQuickActions();
        this.renderHome();
        this.renderFooter();
        this.renderNews();
        this.bindGlobalEvents();
    },

    saveSettings() {
        Bridge.saveSettingsJson(JSON.stringify(this.state.settings));
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

    // ── Footer (real markup: .footer / .footer-profile-area / .footer-right) ──
    renderFooter() {
        const s = this.state.settings;
        document.getElementById("footer-username").textContent = s.xboxGamertag || s.username || "Not signed in";
        document.getElementById("footer-handle").textContent = s.xboxGamertag ? "Xbox Live" : "Local profile";
        document.getElementById("rpc-toggle").classList.toggle("rpc-active", !!s.discordRichPresence);

        const xboxBtn = document.getElementById("xbox-connect-btn");
        if (s.xboxGamertag) {
            xboxBtn.outerHTML = `<div class="xbox-connected-pill" id="xbox-connect-btn"><i class="fa-solid fa-gamepad"></i><span>${s.xboxGamertag}</span></div>`;
        }
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
                html = panelShell({ id, title: this.state.edition === "java" ? "Mods & Addons" : "Addons", body: addonsPanelBody(), activeTabId: id });
                break;
            }
            case "mcversions": html = panelShell({ id, title: "MC Versions", body: emptyState("No versions cached yet", "Bedrock version management is queued — see android/README.md."), activeTabId: id }); break;
            case "bedrockworlds": html = panelShell({ id, title: "Worlds", body: emptyState("No worlds found", "Needs Storage Access Framework wiring to the Java Edition companion app's shared storage."), activeTabId: id }); break;
            case "bedrockpacks": html = panelShell({ id, title: "Packs", body: emptyState("No packs installed", "Behavior and resource packs will list here once wired to shared storage."), activeTabId: id }); break;
            case "bedrockbackups": html = panelShell({ id, title: "Backups", body: emptyState("No backups yet", "World backups will list here once wired to shared storage."), activeTabId: id }); break;
            case "bedrockinstances": html = panelShell({ id, title: "Instances", body: emptyState("No instances yet", "Isolated Bedrock instances will list here once wired to shared storage."), activeTabId: id }); break;
            case "bedrockscreenshots": html = panelShell({ id, title: "Photos", body: emptyState("No screenshots yet", "Reads from the Java Edition companion app's shared storage once wired."), activeTabId: id }); break;
            default: html = panelShell({ id, title: id, body: emptyState("Coming soon", "This panel is queued — see android/README.md's status list."), activeTabId: id });
        }
        root.innerHTML = html;

        if (id === "addons") this.runCfSearch(true);
        if (id === "settings") this.bindSettingsEvents();
    },

    settingsPanelHtml(category) {
        return panelShell({
            id: "settings", title: "Settings",
            body: `<div class="versions-client-switcher" id="settings-categories">${SETTINGS_CATEGORIES(this.state.edition).map(c =>
                `<button class="vcs-btn ${c.id === category ? "active" : ""}" data-settings-category="${c.id}"><i class="${c.icon}"></i> ${c.label}</button>`).join("")}
            </div><div id="settings-body">${settingsPanelBody(category)}</div>`,
            activeTabId: "settings",
        });
    },

    bindSettingsEvents() {
        const body = document.getElementById("settings-body");
        if (!body) return;
        const s = this.state.settings;

        body.querySelectorAll("[data-toggle-setting]").forEach(el => el.addEventListener("click", () => {
            const key = el.dataset.toggleSetting;
            s[key] = !s[key];
            this.saveSettings();
            this.openPanel("settings"); // re-render with current category preserved via active tab class already set server-side isn't tracked; acceptable re-open to "all"
        }));
        body.querySelectorAll("[data-set-accent]").forEach(el => el.addEventListener("click", () => {
            s.accentColor = el.dataset.setAccent;
            document.documentElement.style.setProperty("--accent", s.accentColor);
            this.saveSettings();
            this.openPanel("settings");
        }));
        const activeClientSel = document.getElementById("setting-active-client");
        if (activeClientSel) activeClientSel.addEventListener("change", (e) => { s.selectedClient = e.target.value; this.saveSettings(); this.renderTopBar(); });
        const usernameInput = document.getElementById("setting-username");
        if (usernameInput) usernameInput.addEventListener("change", (e) => { s.username = e.target.value; this.saveSettings(); this.renderFooter(); });
        const cfKeyInput = document.getElementById("setting-cf-key");
        if (cfKeyInput) cfKeyInput.addEventListener("change", (e) => { s.curseForgeApiKeyOverride = e.target.value; this.saveSettings(); });
        const openJava = document.getElementById("open-java-edition");
        if (openJava) openJava.addEventListener("click", () => Bridge.launchJavaEdition());
        const resetBtn = document.getElementById("reset-settings");
        if (resetBtn) resetBtn.addEventListener("click", () => { this.state.settings = { ...DEFAULT_SETTINGS }; this.saveSettings(); this.openPanel("settings"); this.renderFooter(); this.renderHome(); });
        const clearHistory = document.getElementById("clear-recent-history");
        if (clearHistory) clearHistory.addEventListener("click", () => { s.recentlyLaunched = []; this.saveSettings(); this.renderHome(); });

        document.querySelectorAll("[data-settings-category]").forEach(el => el.addEventListener("click", () => {
            document.getElementById("panel-settings").querySelector(".panel-body").innerHTML =
                `<div class="versions-client-switcher" id="settings-categories">${SETTINGS_CATEGORIES(this.state.edition).map(c =>
                    `<button class="vcs-btn ${c.id === el.dataset.settingsCategory ? "active" : ""}" data-settings-category="${c.id}"><i class="${c.icon}"></i> ${c.label}</button>`).join("")}
                </div><div id="settings-body">${settingsPanelBody(el.dataset.settingsCategory)}</div>`;
            this.bindSettingsEvents();
        }));
    },

    async runCfSearch(reset) {
        const resultsEl = document.getElementById("cf-results");
        if (!resultsEl) return;
        resultsEl.innerHTML = `<div style="padding:24px;text-align:center;"><span class="spinner"></span></div>`;
        const query = document.getElementById("cf-search-input")?.value || "";
        try {
            const gameId = this.state.edition === "java" ? CurseForge.GAME_ID_JAVA : CurseForge.GAME_ID_BEDROCK;
            const res = await CurseForge.search(gameId, this.state.cfCategory, query, reset ? 0 : this.state.cfResults.length);
            this.state.cfResults = reset ? res.data : this.state.cfResults.concat(res.data);
            this.state.cfTotalCount = res.pagination ? res.pagination.totalCount : this.state.cfResults.length;
        } catch (e) {
            resultsEl.innerHTML = `<div style="padding:20px;text-align:center;"><span class="error-text">Failed to reach CurseForge: ${e.message}</span></div>`;
            return;
        }
        this.renderCfResults();
    },

    renderCfResults() {
        const resultsEl = document.getElementById("cf-results");
        if (!resultsEl) return;
        if (this.state.cfResults.length === 0) {
            resultsEl.innerHTML = emptyState("No results", "Try a different search term or category.", "fa-solid fa-magnifying-glass");
            return;
        }
        resultsEl.innerHTML = this.state.cfResults.map(mod => `
            <div class="server-row">
                <div class="server-meta" style="flex:1;">
                    <span class="server-name">${mod.name}</span>
                    <span class="server-sub">${(mod.summary || "").slice(0, 90)}</span>
                </div>
                <button class="icon-btn" data-tooltip="Download & Install"><i class="fa-solid fa-download"></i></button>
            </div>`).join("") +
            (this.state.cfResults.length < this.state.cfTotalCount
                ? `<button class="btn-sm" id="cf-load-more" style="width:100%;margin-top:8px;"><i class="fa-solid fa-angles-down"></i> Load more (${this.state.cfResults.length} / ${this.state.cfTotalCount})</button>`
                : "");
        const loadMore = document.getElementById("cf-load-more");
        if (loadMore) loadMore.addEventListener("click", () => this.runCfSearch(false));
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
            this.saveSettings();
            this.renderFooter();
        });

        document.body.addEventListener("click", (e) => {
            const openBtn = e.target.closest("[data-open-panel]");
            if (openBtn) { this.openPanel(openBtn.dataset.openPanel); return; }

            const closeBtn = e.target.closest("[data-close-panel]");
            if (closeBtn) { this.closePanel(); return; }

            const launchBtn = e.target.closest("#launch-btn");
            if (launchBtn) {
                if (this.state.edition === "bedrock") Bridge.launchBedrock(); else Bridge.launchJavaEdition();
                return;
            }

            const selectClient = e.target.closest("[data-select-client]");
            if (selectClient) {
                this.state.settings.selectedClient = selectClient.dataset.selectClient;
                this.saveSettings();
                this.renderTopBar();
                this.openPanel("clients");
                return;
            }

            const dlBtn = e.target.closest("[data-download-client]");
            if (dlBtn) {
                const key = dlBtn.dataset.downloadClient;
                this.state.clients[key].downloading = true;
                this.state.clients[key].progress = 0;
                this.openPanel("clients");
                setTimeout(() => {
                    this.state.clients[key].downloading = false;
                    this.state.clients[key].downloaded = true;
                    this.state.clients[key].upToDate = true;
                    if (this.state.openPanel === "clients") this.openPanel("clients");
                }, 800);
                return;
            }

            const delBtn = e.target.closest("[data-delete-client]");
            if (delBtn) { this.state.clients[delBtn.dataset.deleteClient].downloaded = false; this.openPanel("clients"); return; }

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
        });

        const cfSearchInput = document.body;
        let cfDebounce;
        document.body.addEventListener("input", (e) => {
            if (e.target.id === "cf-search-input") {
                clearTimeout(cfDebounce);
                cfDebounce = setTimeout(() => this.runCfSearch(true), 350);
            }
        });
    },
};

document.addEventListener("DOMContentLoaded", () => App.init());
