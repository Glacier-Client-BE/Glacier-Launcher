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
        mcVersions: [], // no real data source yet — see android/README.md
        mcVersionsChannel: "all",
        mcVersionsFilter: "",
        javaModsTab: "loaders",
        mrCategory: null,
        mrResults: [],
        mrTotalCount: 0,
        glacier: { loading: false, latest: null, error: null },
        javaVersions: { list: [], loading: false, error: null, filter: "", showSnapshots: false, showHistorical: false },
        downloads: [], // session-scoped, see downloadRowHtml()/downloadsPanelHtml() in panels.js
        levMods: { query: "", results: [], loading: false, error: null, hasSearched: false },
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
                this.state.mrCategory = Modrinth.javaCategories[0].facet;
                html = panelShell({ id, title: this.state.edition === "java" ? "Mods & Addons" : "Addons", body: addonsPanelBody(), activeTabId: id });
                break;
            }
            case "mcversions": html = mcVersionsPanelHtml(this.state.mcVersionsChannel, this.state.mcVersionsFilter, this.state.mcVersions); break;
            case "bedrockworlds": html = panelShell({ id, title: "Worlds", body: emptyState("No worlds found", "Needs Storage Access Framework wiring to the Java Edition companion app's shared storage."), activeTabId: id }); break;
            case "bedrockpacks": html = panelShell({ id, title: "Packs", body: emptyState("No packs installed", "Behavior and resource packs will list here once wired to shared storage."), activeTabId: id }); break;
            case "bedrockbackups": html = panelShell({ id, title: "Backups", body: emptyState("No backups yet", "World backups will list here once wired to shared storage."), activeTabId: id }); break;
            case "bedrockinstances": html = panelShell({ id, title: "Instances", body: emptyState("No instances yet", "Isolated Bedrock instances will list here once wired to shared storage."), activeTabId: id }); break;
            case "bedrockscreenshots": html = panelShell({ id, title: "Photos", body: emptyState("No screenshots yet", "Reads from the Java Edition companion app's shared storage once wired."), activeTabId: id }); break;
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
            case "javaprofile": html = panelShell({ id, title: "Profile", body: javaProfilePanelBody(), activeTabId: id }); break;
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
            case "stats": html = bareOverlayHtml("stats", "Statistics", "", statsPanelBody(this.state.settings.totalPlaytimeSeconds || 0)); break;
            case "logs": html = bareOverlayHtml("logs", "Logs & Crashes", `<button class="btn-sm" data-tooltip="Refresh"><i class="fa-solid fa-rotate"></i> Refresh</button>`, logsPanelBody()); break;
            case "levimods": {
                html = levModsPanelHtml(this.state.levMods);
                if (!this.state.levMods.hasSearched && !this.state.levMods.loading) this.loadLevMods();
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

    async runMrSearch(reset) {
        const resultsEl = document.getElementById("mr-results");
        if (!resultsEl) return;
        resultsEl.innerHTML = `<div style="padding:24px;text-align:center;"><span class="spinner"></span></div>`;
        const query = document.getElementById("mr-search-input")?.value || "";
        try {
            const res = await Modrinth.search(this.state.mrCategory, query, reset ? 0 : this.state.mrResults.length);
            this.state.mrResults = reset ? res.hits : this.state.mrResults.concat(res.hits);
            this.state.mrTotalCount = res.total_hits ?? this.state.mrResults.length;
        } catch (e) {
            resultsEl.innerHTML = `<div style="padding:20px;text-align:center;"><span class="error-text">Failed to reach Modrinth: ${e.message}</span></div>`;
            return;
        }
        this.renderMrResults();
    },

    renderMrResults() {
        const resultsEl = document.getElementById("mr-results");
        if (!resultsEl) return;
        if (this.state.mrResults.length === 0) {
            resultsEl.innerHTML = emptyState("No results", "Try a different search term or category.", "fa-solid fa-magnifying-glass");
            return;
        }
        resultsEl.innerHTML = this.state.mrResults.map(project => `
            <div class="server-row">
                <div class="server-meta" style="flex:1;">
                    <span class="server-name">${project.title}</span>
                    <span class="server-sub">${(project.description || "").slice(0, 90)}</span>
                </div>
                <button class="icon-btn" data-tooltip="Download & Install"><i class="fa-solid fa-download"></i></button>
            </div>`).join("") +
            (this.state.mrResults.length < this.state.mrTotalCount
                ? `<button class="btn-sm" id="mr-load-more" style="width:100%;margin-top:8px;"><i class="fa-solid fa-angles-down"></i> Load more (${this.state.mrResults.length} / ${this.state.mrTotalCount})</button>`
                : "");
        const loadMore = document.getElementById("mr-load-more");
        if (loadMore) loadMore.addEventListener("click", () => this.runMrSearch(false));
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

    async loadLevMods() {
        this.state.levMods.loading = true;
        this.state.levMods.error = null;
        if (this.state.openPanel === "levimods") this.openPanel("levimods");
        try {
            this.state.levMods.results = await LeviLaminaMods.search(this.state.levMods.query);
        } catch (e) {
            this.state.levMods.error = `Failed to load registry: ${e.message}`;
        }
        this.state.levMods.loading = false;
        this.state.levMods.hasSearched = true;
        if (this.state.openPanel === "levimods") this.openPanel("levimods");
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
                const downloadId = `client-${key}-${Date.now()}`;
                const clientLabels = { flarial: "Flarial Client", oderso: "OderSo Client", levilamina: "LeviLamina Client" };
                this.state.downloads = [{ id: downloadId, label: clientLabels[key] || key, status: "downloading", progress: 0 }, ...this.state.downloads];
                this.openPanel("clients");
                setTimeout(() => {
                    this.state.clients[key].downloading = false;
                    this.state.clients[key].downloaded = true;
                    this.state.clients[key].upToDate = true;
                    this.state.downloads = this.state.downloads.map(d => d.id === downloadId ? { ...d, status: "complete", progress: 1 } : d);
                    if (this.state.openPanel === "clients") this.openPanel("clients");
                    if (this.state.openPanel === "downloads") this.openPanel("downloads");
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
            if (e.target.closest("[data-glacier-launch]") || e.target.closest("[data-open-java-edition]")) { Bridge.launchJavaEdition(); return; }
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

            if (e.target.closest("[data-levmods-refresh]") || e.target.closest("[data-levmods-retry]")) {
                this.state.levMods.results = [];
                LeviLaminaMods._cache = null;
                this.loadLevMods();
                return;
            }
        });

        let cfDebounce;
        let mrDebounce;
        let mcvFilterDebounce;
        let javaVersionFilterDebounce;
        let levModsFilterDebounce;
        document.body.addEventListener("input", (e) => {
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
            if (e.target.id === "levmods-search-input") {
                clearTimeout(levModsFilterDebounce);
                const value = e.target.value;
                levModsFilterDebounce = setTimeout(() => {
                    this.state.levMods.query = value;
                    this.loadLevMods();
                    document.getElementById("levmods-search-input")?.focus();
                }, 300);
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
        });
    },
};

document.addEventListener("DOMContentLoaded", () => App.init());
