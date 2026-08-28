// Panel markup below is copied from the real Pages/Home.razor blocks (minus
// Razor's @if/@foreach directives, replaced with plain JS string building),
// keeping the same CSS classes so app.css styles it identically to desktop.

const PANEL_TABS = [
    { id: "settings", label: "Settings", icon: "fa-solid fa-gear" },
    { id: "clients", label: "Clients", icon: "fa-solid fa-puzzle-piece" },
    { id: "addons", label: "Addons", icon: "fa-solid fa-cube" },
    { id: "servers", label: "Servers", icon: "fa-solid fa-server" },
    { id: "mcversions", label: "MC Versions", icon: "fa-solid fa-box-archive" },
    { id: "bedrockworlds", label: "Worlds", icon: "fa-solid fa-globe" },
    { id: "bedrockpacks", label: "Packs", icon: "fa-solid fa-boxes-stacked" },
    { id: "bedrockbackups", label: "Backups", icon: "fa-solid fa-clock-rotate-left" },
    { id: "bedrockinstances", label: "Instances", icon: "fa-solid fa-layer-group" },
    { id: "bedrockscreenshots", label: "Photos", icon: "fa-solid fa-images" },
    { id: "credits", label: "Credits", icon: "fa-solid fa-heart" },
];

// Mirrors JavaTabs() in Home.BigFeatures.cs — the Java-edition equivalent of
// the Bedrock panel-tabs bar above (fewer, edition-specific destinations).
const JAVA_PANEL_TABS = [
    { id: "settings", label: "Settings", icon: "fa-solid fa-gear" },
    { id: "javaclients", label: "Launchers", icon: "fa-solid fa-rocket" },
    { id: "addons", label: "Mods", icon: "fa-solid fa-puzzle-piece" },
    { id: "javaversions", label: "Versions", icon: "fa-solid fa-box-archive" },
    { id: "javaprofile", label: "Profile", icon: "fa-solid fa-user" },
    { id: "javascreenshots", label: "Photos", icon: "fa-solid fa-images" },
    { id: "credits", label: "Credits", icon: "fa-solid fa-heart" },
];

function renderPanelTabs(activeId) {
    const tabs = App.state.edition === "java" ? JAVA_PANEL_TABS : PANEL_TABS;
    return `<div class="panel-tabs">${tabs.map(t => `
        <button class="panel-tab ${t.id === activeId ? "active" : ""}" data-open-panel="${t.id}">
            <i class="${t.icon}"></i>${t.label}
        </button>`).join("")}</div>`;
}

function panelShell({ id, title, headerActions = "", body, activeTabId }) {
    return `
    <div class="panel-overlay" id="panel-${id}">
        <div class="panel-handle"></div>
        <div class="panel-header">
            <div class="panel-title-wrap">
                <span class="panel-title">${title}</span>
                <div class="panel-title-underline"></div>
            </div>
            <div class="panel-header-actions">
                ${headerActions}
                <button class="panel-back-btn" data-close-panel data-tooltip="Back">
                    <i class="fa-solid fa-chevron-down"></i>
                </button>
            </div>
        </div>
        <div class="panel-body">${body}</div>
        ${renderPanelTabs(activeTabId)}
    </div>`;
}

function emptyState(title, caption, icon = "fa-solid fa-circle-info") {
    return `<div class="empty-state" style="padding:20px 20px 8px;">
        <i class="${icon}"></i>
        <span>${title}</span>
        <small>${caption}</small>
    </div>`;
}

// ── Clients ──────────────────────────────────────────────────────────────
// Same six cards, same order, same copy as Pages/Home.razor's "clients" panel.
function clientCardHtml({ id, name, iconHtml, statusHtml, desc, actionsHtml, error = "" }) {
    const active = App.state.settings.selectedClient === name ? "client-active" : "";
    return `
    <div class="client-card ${active}" data-client-id="${id}">
        <div class="client-card-header">
            <div class="client-card-icon client-card-icon-img">${iconHtml}</div>
            <div class="client-card-meta">
                <span class="client-card-name">${name}</span>
                <span class="client-card-sub">${statusHtml}</span>
            </div>
            <div class="client-card-actions">${actionsHtml}</div>
        </div>
        ${error ? `<span class="error-text">${error}</span>` : ""}
        <p class="client-card-desc">${desc}</p>
    </div>`;
}

function selectBtn(clientName) {
    return `<button class="icon-btn" data-tooltip="Select" data-select-client="${clientName}"><i class="fa-solid fa-check"></i></button>`;
}

function clientsPanelBody() {
    const s = App.state.settings;
    const sel = (name) => s.selectedClient === name;

    const flarial = App.state.clients.flarial;
    const oderso = App.state.clients.oderso;
    const levilamina = App.state.clients.levilamina;

    const dlActions = (key, c, name) => {
        if (c.downloading) return `<div class="dl-progress-row">
            <div class="progress-bar-wrap" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${Math.round(c.progress * 100)}">
                <div class="progress-bar-fill" style="width:${Math.round(c.progress * 100)}%"></div>
            </div>
            <span class="dl-pct">${Math.round(c.progress * 100)}%</span>
        </div>`;
        let html = "";
        if (!sel(name) && c.downloaded) html += selectBtn(name);
        if (!c.downloaded || !c.upToDate) {
            html += `<button class="icon-btn" data-tooltip="${c.downloaded ? "Update" : "Download"}" data-download-client="${key}"><i class="fa-solid ${c.downloaded ? "fa-arrow-up" : "fa-download"}"></i></button>`;
        }
        if (c.downloaded) html += `<button class="icon-btn icon-btn-ghost" data-tooltip="Delete" data-delete-client="${key}"><i class="fa-solid fa-trash"></i></button>`;
        return html;
    };

    return `
    ${clientCardHtml({
        id: "flarial", name: "Flarial Client",
        iconHtml: `<img src="images/clients/flarial.svg" alt="Flarial Client" />`,
        statusHtml: flarial.downloaded
            ? (flarial.upToDate ? `<span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Up to date</span>` : `<span class="tag-outdated"><i class="fa-solid fa-triangle-exclamation"></i> Update available</span>`)
            : `<span class="client-card-note">Not downloaded</span>`,
        desc: "Feature-rich Bedrock client with modules, HUD customization, and active development.",
        actionsHtml: dlActions("flarial", flarial, "Flarial Client"),
        error: flarial.error || "",
    })}
    ${clientCardHtml({
        id: "latite", name: "Latite Client",
        iconHtml: `<img src="images/clients/latite.png" alt="Latite Client" />`,
        statusHtml: `<span class="client-card-note">Versioned GitHub releases</span>`,
        desc: "Classic Minecraft Bedrock client. Choose a specific release from the Versions panel.",
        actionsHtml: `${!sel("Latite Client") ? selectBtn("Latite Client") : ""}
            <button class="icon-btn icon-btn-ghost" style="background:var(--bg-item);" data-tooltip="View Versions" data-open-panel="mcversions"><i class="fa-solid fa-clock-rotate-left"></i></button>`,
    })}
    ${clientCardHtml({
        id: "oderso", name: "OderSo Client",
        iconHtml: `<img src="images/clients/oderso.png" alt="OderSo Client" />`,
        statusHtml: oderso.downloaded
            ? (oderso.upToDate ? `<span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Up to date</span>` : `<span class="tag-outdated"><i class="fa-solid fa-triangle-exclamation"></i> Update available</span>`)
            : `<span class="client-card-note">Not downloaded</span>`,
        desc: "OderSo Client — curated Minecraft Bedrock experience by MasonOderSo.",
        actionsHtml: dlActions("oderso", oderso, "OderSo Client"),
        error: oderso.error || "",
    })}
    ${clientCardHtml({
        id: "levilamina", name: "LeviLamina Client",
        iconHtml: `<i class="fa-solid fa-layer-group"></i>`,
        statusHtml: levilamina.downloaded
            ? (levilamina.upToDate ? `<span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Up to date</span>` : `<span class="tag-outdated"><i class="fa-solid fa-triangle-exclamation"></i> Update available</span>`)
            : `<span class="client-card-note">Not downloaded</span>`,
        desc: "LeviLamina — open-source native Bedrock mod loader by LiteLDev, injected the same way as the other clients here.",
        actionsHtml: `${levilamina.downloaded ? `<button class="icon-btn" data-tooltip="Browse LeviLamina mods" data-open-panel="levimods"><i class="fa-solid fa-puzzle-piece"></i></button>` : ""}${dlActions("levilamina", levilamina, "LeviLamina Client")}`,
        error: levilamina.error || "",
    })}
    ${clientCardHtml({
        id: "vanilla", name: "Vanilla",
        iconHtml: `<i class="fa-solid fa-cube" style="color:var(--green);"></i>`,
        statusHtml: `<span class="client-card-note">Launches Minecraft with no DLL injection</span>`,
        desc: "Pure stock Minecraft Bedrock — useful for diagnostics, multiplayer realms, or just playing un-modified.",
        actionsHtml: !sel("Vanilla") ? selectBtn("Vanilla") : "",
    })}
    <div class="drop-hint-row">
        <i class="fa-solid fa-file-arrow-down"></i>
        <span>Client injection needs root on Android — see Settings</span>
        <button class="btn-sm" id="browse-mod-file" style="margin-left:auto;"><i class="fa-solid fa-folder-open"></i> Browse...</button>
    </div>`;
}

// ── Servers ──────────────────────────────────────────────────────────────
const POPULAR_SERVERS = [
    { name: "Hive", address: "geo.hivebedrock.network", port: 19132, icon: "fa-solid fa-hexagon-nodes", color: "#f5a623" },
    { name: "CubeCraft", address: "play.cubecraft.net", port: 19132, icon: "fa-solid fa-cube", color: "#4a90e2" },
    { name: "Mineplex", address: "mco.mineplex.com", port: 19132, icon: "fa-solid fa-tower-cell", color: "#e94e4e" },
    { name: "Lifeboat", address: "play.lbsg.net", port: 19132, icon: "fa-solid fa-life-ring", color: "#00bcd4" },
    { name: "Galaxite", address: "play.galaxite.net", port: 19132, icon: "fa-solid fa-star", color: "#9b51e0" },
];

function serverRowHtml(s, saved) {
    return `
    <div class="server-row">
        <div class="server-icon" style="background:${s.color || "var(--accent)"};">
            <i class="${s.icon || "fa-solid fa-server"}"></i>
        </div>
        <div class="server-meta">
            <span class="server-name">${s.name}</span>
            <span class="server-sub">${s.address}:${s.port}</span>
        </div>
        <div class="version-actions">
            ${saved ? `<button class="icon-btn icon-btn-ghost" data-tooltip="Edit"><i class="fa-solid fa-pen"></i></button>
                <button class="icon-btn icon-btn-ghost" data-tooltip="Delete" data-delete-server="${s.address}"><i class="fa-solid fa-trash"></i></button>`
                : `<button class="icon-btn" data-tooltip="Save" data-save-server="${s.address}"><i class="fa-solid fa-bookmark"></i></button>`}
            <button class="copy-btn" data-tooltip="Copy address" data-copy-address="${s.address}:${s.port}"><i class="fa-solid fa-copy"></i></button>
            <button class="icon-btn" data-tooltip="Launch & connect"><i class="fa-solid fa-play"></i></button>
        </div>
    </div>`;
}

function serversPanelBody() {
    const saved = App.state.settings.savedServers || [];
    const suggestions = POPULAR_SERVERS.filter(p => !saved.some(s => s.address === p.address));
    return `
    ${saved.length === 0
        ? emptyState("No saved servers yet", "Add a Minecraft Bedrock server to quick-launch into it. The current client will be injected before connecting.", "fa-solid fa-server")
        : saved.map(s => serverRowHtml(s, true)).join("")}
    ${suggestions.length > 0 ? `<span class="panel-section-label">Popular</span>${suggestions.map(s => serverRowHtml(s, false)).join("")}` : ""}`;
}

// ── Credits ──────────────────────────────────────────────────────────────
function creditCardHtml(name, sub, iconHtml, links) {
    return `
    <div class="credits-card">
        <div class="credits-card-icon" style="padding:6px;">${iconHtml}</div>
        <div class="credits-card-meta">
            <span class="credits-card-name">${name}</span>
            <span class="credits-card-sub">${sub}</span>
        </div>
        <div class="credits-card-actions">
            ${links.map(l => `<button class="credits-link-btn" data-tooltip="${l.label}" data-open-url="${l.url}"><i class="${l.icon}"></i></button>`).join("")}
        </div>
    </div>`;
}

function creditsPanelBody() {
    return `
    <span class="panel-section-label">Launcher</span>
    <div class="credits-card credits-card-glacier">
        <div class="credits-card-icon" style="padding:6px;"><img src="images/icon.png" style="width:100%;height:100%;object-fit:contain;"/></div>
        <div class="credits-card-meta">
            <span class="credits-card-name">Glacier Launcher</span>
            <span class="credits-card-sub">Built by Pepe · Glacier Productions</span>
        </div>
        <div class="credits-card-actions">
            <button class="credits-link-btn" data-tooltip="GitHub" data-open-url="https://github.com/Glacier-Client-BE"><i class="fa-brands fa-github"></i></button>
            <button class="credits-link-btn" data-tooltip="Website" data-open-url="https://glacierclient.xyz"><i class="fa-solid fa-globe"></i></button>
            <button class="credits-link-btn" data-tooltip="Discord" data-open-url="https://discord.glacierclient.xyz"><i class="fa-brands fa-discord"></i></button>
        </div>
    </div>
    <span class="panel-section-label">Clients</span>
    ${creditCardHtml("Latite Client", "by Imrglop & contributors", `<img src="images/clients/latite.png" style="width:100%;height:100%;object-fit:contain;"/>`, [
        { label: "GitHub Releases", icon: "fa-brands fa-github", url: "https://github.com/Imrglop/Latite-Releases" },
        { label: "Discord", icon: "fa-brands fa-discord", url: "https://discord.gg/latite" },
    ])}
    ${creditCardHtml("Flarial Client", "by the Flarial team", `<img src="images/clients/flarial.svg" style="width:100%;height:100%;object-fit:contain;"/>`, [
        { label: "Website", icon: "fa-solid fa-globe", url: "https://flarial.xyz" },
        { label: "Discord", icon: "fa-brands fa-discord", url: "https://discord.gg/flarial" },
    ])}
    ${creditCardHtml("OderSo Client", "by MasonOderSo", `<img src="images/clients/oderso.png" style="width:100%;height:100%;object-fit:contain;"/>`, [
        { label: "GitHub", icon: "fa-brands fa-github", url: "https://github.com/MasonOderSo/oderso-data" },
    ])}
    <span class="panel-section-label">Open Source</span>
    <div class="credits-oss-row">
        <i class="fa-solid fa-code-branch" style="color:var(--accent);"></i>
        <span>Glacier Launcher is open source. Contributions and forks are welcome.</span>
        <button class="btn-sm" data-open-url="https://github.com/Glacier-Client-BE/Glacier-Launcher"><i class="fa-brands fa-github"></i> View</button>
    </div>`;
}

// ── Addons ─────────────────────────────────────────────────────────────
// Bedrock: straight to CurseForge search, same as desktop. Java: the same
// javaModsTab sub-tab bar (Loaders/Mods/Assets/Datapacks/Tools/CurseForge/
// Modrinth) from Pages/Home.razor's "addons" panel Java branch.
function curseForgeSearchBody() {
    if (!CurseForge.isAvailable()) {
        return emptyState("CurseForge API key required", "Get a free key from the CurseForge developer console, then paste it in Settings.", "fa-solid fa-key");
    }
    const cats = App.state.edition === "java" ? CurseForge.javaCategories : CurseForge.bedrockCategories;
    return `
    <div class="versions-client-switcher" id="cf-categories">
        ${cats.map((c, i) => `<button class="vcs-btn ${i === 0 ? "active" : ""}" data-cf-category="${c.classId}"><i class="${c.icon}"></i> ${c.label}</button>`).join("")}
    </div>
    <div class="panel-search-wrap">
        <i class="fa-solid fa-magnifying-glass"></i>
        <input class="panel-search-input" id="cf-search-input" placeholder="Search CurseForge ${App.state.edition === "java" ? "for Java mods, modpacks, shaders..." : "addons..."}" />
    </div>
    <div id="cf-results"></div>`;
}

function modrinthSearchBody() {
    return `
    <div class="versions-client-switcher" id="mr-categories">
        ${Modrinth.javaCategories.map((c, i) => `<button class="vcs-btn ${i === 0 ? "active" : ""}" data-mr-category="${c.facet}"><i class="${c.icon}"></i> ${c.label}</button>`).join("")}
    </div>
    <div class="panel-search-wrap">
        <i class="fa-solid fa-magnifying-glass"></i>
        <input class="panel-search-input" id="mr-search-input" placeholder="Search Modrinth..." />
    </div>
    <div id="mr-results"></div>`;
}

// Loaders: real cards from Pages/Home.razor (Fabric/Quilt/Forge/NeoForge),
// gated the same way desktop gates them — behind an active Java version,
// which this app doesn't track yet, so the honest "no version selected"
// branch is also the true current state, not a shortcut around it.
function javaLoadersBody() {
    return emptyState("No version selected", "Pick a Minecraft version from the Versions panel first, then come back to install a mod loader.", "fa-solid fa-triangle-exclamation");
}

function javaModsBody() {
    return emptyState("No local mods", "Drop jars into the active instance mods folder, or browse CurseForge/Modrinth.", "fa-solid fa-puzzle-piece");
}

function javaAssetsBody() {
    const rows = [
        { name: "Resource Packs", icon: "fa-solid fa-image", color: "rgba(67,181,129,0.15)", fg: "var(--green)" },
        { name: "Shader Packs", icon: "fa-solid fa-wand-magic-sparkles", color: "rgba(255,200,50,0.15)", fg: "#ffc832" },
        { name: "Saves / Worlds", icon: "fa-solid fa-map", color: "rgba(114,137,218,0.15)", fg: "var(--accent)" },
        { name: "Screenshots", icon: "fa-solid fa-camera", color: "rgba(240,71,71,0.15)", fg: "var(--red)" },
        { name: "Schematics", icon: "fa-solid fa-drafting-compass", color: "rgba(250,166,26,0.15)", fg: "var(--orange)" },
    ];
    return rows.map(r => `
    <div class="client-card">
        <div class="client-card-header">
            <div class="client-card-icon" style="background:${r.color}; color:${r.fg}; padding:8px;"><i class="${r.icon}"></i></div>
            <div class="client-card-meta">
                <span class="client-card-name">${r.name}</span>
                <span class="client-card-sub" style="opacity:0.5;">Needs the Java Edition companion app installed</span>
            </div>
            <div class="client-card-actions">
                <button class="icon-btn" data-tooltip="Open folder" disabled><i class="fa-solid fa-folder-open"></i></button>
            </div>
        </div>
    </div>`).join("");
}

function javaToolsBody() {
    const rows = [
        { name: "Backup Saves", sub: "Zip your worlds before a risky update", icon: "fa-solid fa-box-archive", color: "rgba(114,137,218,0.15)", fg: "var(--accent)", action: "fa-solid fa-download" },
        { name: "Export Modpack", sub: "Bundle mods + config into a shareable zip", icon: "fa-solid fa-file-zipper", color: "rgba(67,181,129,0.15)", fg: "var(--green)", action: "fa-solid fa-file-export" },
        { name: "Duplicate Instance", sub: "Clone your active instance with all mods & config", icon: "fa-solid fa-clone", color: "rgba(250,166,26,0.15)", fg: "var(--orange)", action: "fa-solid fa-copy" },
    ];
    return rows.map(r => `
    <div class="client-card">
        <div class="client-card-header">
            <div class="client-card-icon" style="background:${r.color}; color:${r.fg}; padding:8px;"><i class="${r.icon}"></i></div>
            <div class="client-card-meta">
                <span class="client-card-name">${r.name}</span>
                <span class="client-card-sub" style="opacity:0.5;">${r.sub}</span>
            </div>
            <div class="client-card-actions">
                <button class="icon-btn" disabled><i class="${r.action}"></i></button>
            </div>
        </div>
    </div>`).join("");
}

const JAVA_MODS_TABS = [
    { id: "loaders", label: "Loaders", icon: "fa-solid fa-screwdriver-wrench" },
    { id: "mods", label: "Mods", icon: "fa-solid fa-puzzle-piece" },
    { id: "assets", label: "Assets", icon: "fa-solid fa-image" },
    { id: "datapacks", label: "Datapacks", icon: "fa-solid fa-cubes-stacked" },
    { id: "tools", label: "Tools", icon: "fa-solid fa-toolbox" },
    { id: "curseforge", label: "CurseForge", icon: "fa-solid fa-fire" },
    { id: "modrinth", label: "Modrinth", icon: "fa-solid fa-leaf" },
];

function javaAddonsBody(tab) {
    const switcher = `<div class="versions-client-switcher">${JAVA_MODS_TABS.map(t =>
        `<button class="vcs-btn ${t.id === tab ? "active" : ""}" data-java-mods-tab="${t.id}"><i class="${t.icon}"></i> ${t.label}</button>`).join("")}</div>`;

    let body;
    switch (tab) {
        case "loaders": body = javaLoadersBody(); break;
        case "assets": body = javaAssetsBody(); break;
        case "tools": body = javaToolsBody(); break;
        case "datapacks": body = emptyState("Datapacks", "Needs a world picker wired to the Java Edition companion app's shared storage — queued.", "fa-solid fa-cubes-stacked"); break;
        case "curseforge": return `${switcher}${curseForgeSearchBody()}`;
        case "modrinth": return `${switcher}${modrinthSearchBody()}`;
        default: body = javaModsBody();
    }
    return `${switcher}${body}`;
}

function addonsPanelBody() {
    return App.state.edition === "java" ? javaAddonsBody(App.state.javaModsTab) : curseForgeSearchBody();
}

// ── Settings ───────────────────────────────────────────────────────────
const SETTINGS_CATEGORIES = (edition) => ([
    { id: "all", label: "All", icon: "fa-solid fa-layer-group" },
    edition === "bedrock"
        ? { id: "injection", label: "Inject", icon: "fa-solid fa-syringe" }
        : { id: "java", label: "Java", icon: "fa-brands fa-java" },
    { id: "appearance", label: "Looks", icon: "fa-solid fa-palette" },
    { id: "account", label: "Account", icon: "fa-solid fa-user" },
    { id: "system", label: "System", icon: "fa-solid fa-sliders" },
]);

function settingRow(label, hint, controlHtml) {
    return `<div class="setting-row">
        <div class="setting-meta"><span class="setting-label">${label}</span>${hint ? `<span class="setting-hint">${hint}</span>` : ""}</div>
        ${controlHtml}
    </div>`;
}

function toggleHtml(key, on) {
    return `<div class="toggle ${on ? "on" : ""}" data-toggle-setting="${key}"></div>`;
}

function settingsPanelBody(category) {
    const s = App.state.settings;
    const cat = (id) => category === "all" || category === id;
    let html = "";

    if (cat("injection")) {
        html += `<div class="settings-section"><span class="panel-section-label">Injection</span>
        ${settingRow("Active client", "", `<select class="setting-select" id="setting-active-client">
            ${["Latite Client", "Flarial Client", "OderSo Client", "LeviLamina Client", "Vanilla"].map(c => `<option value="${c}" ${s.selectedClient === c ? "selected" : ""}>${c}</option>`).join("")}
        </select>`)}
        ${settingRow("Injection delay", "How long to wait after game launches before injecting",
            `<input type="range" min="500" max="15000" step="500" id="setting-injection-delay" value="${s.injectionDelayMs}" /><span style="font-size:11px;color:var(--text-dim);">${(s.injectionDelayMs / 1000).toFixed(1)}s</span>`)}
        ${settingRow("Auto-inject", "Inject automatically once the game process is detected", toggleHtml("autoInject", s.autoInject))}
        ${settingRow("Close after launch", "Minimise the launcher once injection succeeds", toggleHtml("closeAfterLaunch", s.closeAfterLaunch))}
        </div>`;
    }

    if (cat("java")) {
        html += `<div class="settings-section"><span class="panel-section-label">Java Edition</span>
        <div style="padding:8px 0;">RAM, JVM args, resolution, offline mode, and version filters are configured in the Java Edition companion app itself.</div>
        ${settingRow("Java Edition companion app", App.state.javaInstalled ? "Installed" : "Not installed",
            `<button class="btn-sm" id="open-java-edition" ${App.state.javaInstalled ? "" : "disabled"}>Open</button>`)}
        </div>`;
    }

    if (cat("appearance")) {
        html += `<div class="settings-section"><span class="panel-section-label">Appearance</span>
        ${settingRow("Accent colour", "Used for buttons, highlights and glows",
            `<div class="color-swatches">${["#7289da", "#43b581", "#f04747", "#faa61a", "#9b59b6", "#00bcd4", "#e91e63", "#ffffff"].map(c =>
                `<div class="color-swatch ${s.accentColor === c ? "active" : ""}" style="background:${c};" data-set-accent="${c}"></div>`).join("")}</div>`)}
        ${settingRow("Theme preset", "Background tone — affects panels and overlays",
            `<select class="setting-select" id="setting-theme-preset">${["dark", "darker", "midnight", "slate", "ocean", "forest", "sunset", "light"].map(t =>
                `<option value="${t}" ${s.themePreset === t ? "selected" : ""}>${t[0].toUpperCase() + t.slice(1)}</option>`).join("")}</select>`)}
        ${settingRow("Compact mode", "Tighter spacing throughout", toggleHtml("compactMode", s.compactMode))}
        ${settingRow("Animations", "Disable for low-end devices", toggleHtml("animationsEnabled", s.animationsEnabled))}
        </div>`;
    }

    if (cat("account")) {
        html += `<div class="settings-section"><span class="panel-section-label">Account</span>
        ${settingRow("Display name", "", `<input class="setting-input" id="setting-username" type="text" value="${s.username || ""}" />`)}
        ${settingRow("Profile display", "Which account to show in the footer",
            `<select class="setting-select" id="setting-profile-display">${["auto", "xbox", "discord"].map(m =>
                `<option value="${m}" ${s.profileDisplayMode === m ? "selected" : ""}>${m}</option>`).join("")}</select>`)}
        </div>
        <div class="settings-section"><span class="panel-section-label">Social</span>
        ${settingRow("Discord Rich Presence", "Posts a “now playing” webhook message (no native IPC on Android)", toggleHtml("discordRichPresence", s.discordRichPresence))}
        ${settingRow("Xbox profile", s.xboxGamertag || "Not signed in", `<button class="btn-sm" id="xbox-sign-in-settings">Sign in</button>`)}
        </div>`;
    }

    if (cat("system")) {
        html += `<div class="settings-section"><span class="panel-section-label">Quality of Life</span>
        ${settingRow("Show recently launched", "", toggleHtml("showRecentlyLaunched", s.showRecentlyLaunched))}
        ${settingRow("Clear recent history", "", `<button class="btn-sm" id="clear-recent-history">Clear</button>`)}
        </div>
        <div class="settings-section"><span class="panel-section-label">Updates</span>
        ${settingRow("Check for updates on startup", "", toggleHtml("checkUpdatesOnStartup", s.checkUpdatesOnStartup))}
        </div>
        <div class="settings-section"><span class="panel-section-label">CurseForge</span>
        ${settingRow("CurseForge API key", "", `<input class="setting-input" id="setting-cf-key" type="text" value="${s.curseForgeApiKeyOverride || ""}" />`)}
        </div>
        <div class="settings-section"><span class="panel-section-label">Backup</span>
        ${settingRow("Reset to defaults", "", `<button class="btn-sm" style="color:var(--red);" id="reset-settings">Reset</button>`)}
        </div>
        <div class="settings-section"><span class="panel-section-label">About</span>
        <div style="padding:8px 0; font-size:12px; color:var(--text-dim);">Glacier Launcher for Android</div>
        </div>`;
    }

    return html;
}

// ── MC Versions ────────────────────────────────────────────────────────
// Mirrors Pages/Home.razor's "mcversions" panel structure (channel tabs,
// filter, version rows with download/switch/delete). The desktop panel's
// "Install from Microsoft Store" row (VanillaVersionService/
// StoreInstallService's BedrockUpdater sideload) is Windows-only — Android
// Bedrock is a single always-current Play Store app with no side-loadable
// version history, so that row is replaced with an honest note instead of
// a non-functional button, same treatment as ClientInjectionService.
function versionRowHtml(v) {
    const actionsHtml = v.downloaded
        ? `<div class="version-actions">
            <button class="icon-btn icon-btn-ghost" title="Delete" data-delete-mcversion="${v.id}"><i class="fa-solid fa-trash"></i></button>
            ${!v.active ? `<button class="icon-btn mcv-switch-btn" title="Switch to this version" data-switch-mcversion="${v.id}"><i class="fa-solid fa-right-left"></i></button>` : ""}
        </div>`
        : `<div class="version-actions"><button class="icon-btn" title="Download" data-download-mcversion="${v.id}"><i class="fa-solid fa-download"></i></button></div>`;
    return `
    <div class="version-row ${v.active ? "mcv-active-row" : ""}">
        <div class="version-meta">
            <div style="display:flex; align-items:center; gap:6px;">
                <span class="version-name">Minecraft ${v.id}</span>
                ${v.active ? `<span class="tag-active">Active</span>` : ""}
            </div>
            ${v.downloaded ? `<span class="version-sub">Ready · ${v.size}</span>` : `<span class="version-sub">${v.size}</span>`}
        </div>
        ${actionsHtml}
    </div>`;
}

// Full panel-overlay markup, not routed through panelShell(): the desktop
// panel puts the info bar / search / channel tabs BETWEEN .panel-header and
// .panel-body (siblings, not nested inside it), unlike the other panels.
function mcVersionsPanelHtml(channel, filter, versions) {
    const filtered = versions.filter(v =>
        (channel === "all" || v.channel === channel) &&
        (!filter || v.id.toLowerCase().includes(filter.toLowerCase())));
    const active = filtered.filter(v => v.active);
    const rest = filtered.filter(v => !v.active);

    const listHtml = filtered.length === 0
        ? `<div class="versions-loading"><span style="opacity:0.5;">${
            versions.length === 0 ? "No versions found." : `No results for "${filter}".`
        }</span></div>`
        : `${active.length > 0 ? `<span class="panel-section-label">Active</span>${active.map(versionRowHtml).join("")}<span class="panel-section-label">All Versions</span>` : ""}
           ${rest.map(versionRowHtml).join("")}`;

    return `
    <div class="panel-overlay" id="panel-mcversions">
        <div class="panel-handle"></div>
        <div class="panel-header">
            <div class="panel-title-wrap">
                <span class="panel-title">MC Versions</span>
                <div class="panel-title-underline"></div>
            </div>
            <div class="panel-header-actions">
                <button class="panel-back-btn" data-close-panel data-tooltip="Back"><i class="fa-solid fa-chevron-down"></i></button>
            </div>
        </div>
        <div class="mcv-info-bar">
            <i class="fa-solid fa-circle-info"></i>
            <span>Android Bedrock is a single always-current Play Store app — there's no side-loadable
            version history the way Windows' Developer Mode sideload or Microsoft Store rollback allow.
            This list is illustrative of the desktop panel's layout; switching versions isn't possible here.</span>
        </div>
        <div class="panel-search-wrap">
            <i class="fa-solid fa-magnifying-glass"></i>
            <input class="panel-search-input" id="mcv-filter-input" placeholder="Filter Minecraft versions..." value="${filter}" />
        </div>
        <div class="mcv-channel-tabs">
            ${["all", "release", "preview"].map(c =>
                `<button class="mcv-channel-tab ${channel === c ? "active" : ""}" data-mcv-channel="${c}">${c === "all" ? "All" : c === "release" ? "Releases" : "Previews"}</button>`).join("")}
        </div>
        <div class="panel-body">${listHtml}</div>
        ${renderPanelTabs("mcversions")}
    </div>`;
}

// ── Java Launchers ("javaclients") ───────────────────────────────────────
// Mirrors Pages/Home.razor's "javaclients" panel: Vanilla (built-in, links
// to Versions), Glacier Client (real manifest fetch — same CDN as
// GlacierClientService.cs), Lunar Client/Badlion. The desktop panel detects
// locally-installed Lunar/Badlion .exe and can direct-launch them; neither
// client ships an Android build at all, so that row is an honest "not
// available on Android" rather than fake detection.
function javaClientsPanelBody(glacierState) {
    const glacierActions = (() => {
        if (glacierState.loading) return `<span style="opacity:0.6;">Checking…</span>`;
        if (glacierState.error) return `<button class="vcs-btn vcs-btn-ghost" data-glacier-retry><i class="fa-solid fa-rotate"></i> Retry</button>`;
        if (!glacierState.latest) return "";
        return glacierState.latest.installed
            ? `<button class="vcs-btn" data-glacier-launch><i class="fa-solid fa-play"></i> Launch</button>
               <button class="vcs-btn vcs-btn-ghost vcs-btn-icon" data-tooltip="Uninstall" data-glacier-uninstall><i class="fa-solid fa-trash"></i></button>`
            : `<button class="vcs-btn" data-glacier-install><i class="fa-solid fa-download"></i> Install</button>`;
    })();
    const glacierSub = glacierState.loading
        ? "Checking for releases…"
        : glacierState.error
            ? `<span class="error-text">${glacierState.error}</span>`
            : glacierState.latest
                ? `${glacierState.latest.name} · ${glacierState.latest.loader}${glacierState.latest.installed ? ` · <span style="color:var(--green);"><i class="fa-solid fa-circle-check"></i> Installed</span>` : ` · <span class="client-card-note">Not installed</span>`}`
                : "No releases available right now.";

    return `
    <div class="mcv-info-bar">
        <i class="fa-solid fa-circle-info"></i>
        <span>Vanilla and Glacier Client launch through the Java Edition companion app. Lunar Client and Badlion don't ship an Android build — there's no client to detect or launch here.</span>
    </div>
    <div class="panel-body">
        <span class="panel-section-label">Built-in</span>
        <div class="credits-card credits-card-glacier">
            <div class="credits-card-icon" style="background:var(--accent-bg); color:var(--accent);"><i class="fa-brands fa-java"></i></div>
            <div class="credits-card-meta">
                <span class="credits-card-name">Vanilla (Glacier built-in)</span>
                <span class="credits-card-sub">Launches Mojang's Java client directly. Pick a version in the Versions panel.</span>
            </div>
            <div class="credits-card-actions">
                <button class="vcs-btn" data-open-panel="javaversions"><i class="fa-solid fa-box-archive"></i> Versions</button>
            </div>
        </div>
        <div class="credits-card">
            <div class="credits-card-icon" style="background:var(--accent-bg); color:var(--accent);"><i class="fa-solid fa-snowflake"></i></div>
            <div class="credits-card-meta">
                <span class="credits-card-name">Glacier Client</span>
                <span class="credits-card-sub">${glacierSub}</span>
            </div>
            <div class="credits-card-actions">${glacierActions}</div>
        </div>
        <span class="panel-section-label">Third-party</span>
        <div class="credits-card">
            <div class="credits-card-icon" style="background:#06b6d422; color:#22d3ee;"><i class="fa-solid fa-moon"></i></div>
            <div class="credits-card-meta">
                <span class="credits-card-name">Lunar Client <span style="opacity:0.55; font-weight:500;">incl. Badlion</span></span>
                <span class="credits-card-sub">Not available on Android — Lunar/Badlion have no Android client to detect or launch.</span>
            </div>
            <div class="credits-card-actions">
                <button class="vcs-btn vcs-btn-ghost" data-open-url="https://www.lunarclient.com"><i class="fa-solid fa-arrow-up-right-from-square"></i> Website</button>
            </div>
        </div>
    </div>`;
}

// ── Java Versions ─────────────────────────────────────────────────────────
// Real Mojang version_manifest_v2.json, same as JavaVersionService.cs.
// Install/Launch hand off to the Java Edition companion app (Pojav owns
// actual per-version install + launch), rather than duplicating that here.
function javaVersionRowHtml(v) {
    return `
    <div class="version-row ${v.active ? "mcv-active-row" : ""}">
        <div class="version-meta">
            <div style="display:flex; align-items:center; gap:6px;">
                <span class="version-name">Minecraft ${v.id}</span>
                ${v.active ? `<span class="tag-active">Active</span>` : ""}
                <span class="version-sub" style="opacity:0.6;">${v.typeLabel}</span>
            </div>
        </div>
        <button class="vcs-btn" data-open-java-edition><i class="fa-solid fa-external-link-alt"></i>&nbsp;Open in Java Edition</button>
    </div>`;
}

function javaVersionsPanelHtml(filter, showSnapshots, showHistorical, versions, loading, error) {
    const filtered = versions.filter(v => {
        if (v.type === "release") return true;
        if (v.type === "snapshot") return showSnapshots;
        if (v.type === "old_beta" || v.type === "old_alpha") return showHistorical;
        return false;
    }).filter(v => !filter || v.id.toLowerCase().includes(filter.toLowerCase()));

    let listHtml;
    if (loading) {
        listHtml = `<div class="versions-loading"><span class="spinner"></span><span>Fetching Java versions...</span></div>`;
    } else if (error && versions.length === 0) {
        listHtml = `<div class="versions-loading versions-error"><i class="fa-solid fa-triangle-exclamation"></i><span>${error}</span>
            <button class="vcs-btn" style="margin-top:8px;" data-refresh-java-versions><i class="fa-solid fa-rotate"></i> Retry</button></div>`;
    } else if (filtered.length === 0) {
        listHtml = `<div class="versions-loading"><span style="opacity:0.5;">No results${filter ? ` for "${filter}"` : ""}.</span></div>`;
    } else {
        listHtml = (error ? `<div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>${error}</span></div>` : "") +
            `<span class="panel-section-label">Available (${filtered.length})</span>` +
            filtered.slice(0, 200).map(javaVersionRowHtml).join("");
    }

    return `
    <div class="panel-overlay" id="panel-javaversions">
        <div class="panel-handle"></div>
        <div class="panel-header">
            <div class="panel-title-wrap"><span class="panel-title">Versions</span><div class="panel-title-underline"></div></div>
            <div class="panel-header-actions">
                <button class="panel-icon-btn ${showSnapshots ? "active" : ""}" data-toggle-java-snapshots data-tooltip="${showSnapshots ? "Hide snapshots" : "Show snapshots"}"><i class="fa-solid fa-flask"></i></button>
                <button class="panel-icon-btn ${showHistorical ? "active" : ""}" data-toggle-java-historical data-tooltip="${showHistorical ? "Hide beta/alpha" : "Show beta/alpha"}"><i class="fa-solid fa-clock-rotate-left"></i></button>
                <button class="panel-icon-btn" data-refresh-java-versions data-tooltip="Refresh"><i class="fa-solid fa-rotate"></i></button>
                <button class="panel-back-btn" data-close-panel data-tooltip="Back"><i class="fa-solid fa-chevron-down"></i></button>
            </div>
        </div>
        <div class="versions-client-switcher"><button class="vcs-btn active"><i class="fa-brands fa-java"></i> Vanilla</button></div>
        <div class="panel-search-wrap">
            <i class="fa-solid fa-magnifying-glass"></i>
            <input class="panel-search-input" id="java-version-filter-input" placeholder="Filter Java versions (e.g. 1.21, 1.8.9)..." value="${filter}" />
        </div>
        <div class="panel-body">${listHtml}</div>
        ${renderPanelTabs("javaversions")}
    </div>`;
}

// ── Java Profile ───────────────────────────────────────────────────────
// Mirrors Pages/Home.razor's "javaprofile" panel. The full signed-in view
// (SkinViewer 3D preview, cape wardrobe, playtime stats) needs a real
// Microsoft/Xbox sign-in this app doesn't have wired yet, so the honest
// "not signed in" branch — the desktop panel's own gate when JavaUuid is
// empty — is also this app's true current state, not a shortcut.
function javaProfilePanelBody() {
    return `<div class="skin-empty">
        <i class="fa-solid fa-user-astronaut"></i>
        <span>Sign in with your Microsoft account to view your Minecraft profile.</span>
        <button class="vcs-btn" disabled data-tooltip="Microsoft sign-in is queued — see android/README.md">
            <i class="fa-brands fa-xbox"></i>&nbsp;Sign in
        </button>
    </div>`;
}

// ── Java Screenshots ──────────────────────────────────────────────────────
function javaScreenshotsPanelBody() {
    return emptyState("No screenshots yet", "Press F2 in-game to capture one — they'll show up here once wired to shared storage.", "fa-solid fa-image");
}

// ── News & Updates ─────────────────────────────────────────────────────
// Real data: GitHub's public releases API for this repo (same as
// AutoUpdateService.cs) and the Glacier news feed (same as
// NewsService.cs) — both plain public HTTP, no auth needed.
const NEWS_PANEL_TABS = [
    { id: "settings", label: "Settings", icon: "fa-solid fa-gear" },
    { id: "home", label: "Home", icon: "fa-solid fa-house" },
    { id: "news", label: "News", icon: "fa-solid fa-newspaper" },
    { id: "credits", label: "Credits", icon: "fa-solid fa-heart" },
];

function escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text ?? "";
    // innerHTML only escapes &, <, > (safe for text nodes) — also escape quotes
    // since callers interpolate this into HTML attributes too.
    return div.innerHTML.replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}

function newsRowHtml(item) {
    return `
    <div class="news-row" data-open-url="${escapeHtml(item.url || "")}">
        <div class="news-icon"><i class="${item.icon || "fa-solid fa-newspaper"}"></i></div>
        <div class="news-meta">
            <span class="news-title">${escapeHtml(item.title)}</span>
            <span class="news-sub">${escapeHtml(item.subtitle || "")}</span>
        </div>
        ${item.url ? `<i class="fa-solid fa-arrow-up-right-from-square news-ext"></i>` : ""}
    </div>`;
}

function newsPanelHtml(state) {
    let body;
    if (state.loading) {
        body = `<div class="versions-loading"><span class="spinner"></span><span>Loading news…</span></div>`;
    } else {
        const posts = state.posts.length > 0 ? state.posts : state.fallbackItems;
        body = `<span class="panel-section-label">Latest</span>${posts.map(newsRowHtml).join("")}`;
        body += state.releases.length > 0
            ? `<span class="panel-section-label">Changelog</span>${state.releases.map(r => `
                <div class="changelog-entry">
                    <div class="changelog-head">
                        <span class="changelog-tag">${r.tag}</span>
                        ${r.publishedAt ? `<span class="changelog-date">${new Date(r.publishedAt).toLocaleDateString()}</span>` : ""}
                    </div>
                    <div class="changelog-body">${escapeHtml(r.body).replace(/\n/g, "<br>")}</div>
                </div>`).join("")}`
            : `<div class="cache-notice"><i class="fa-solid fa-circle-info"></i><span>No release notes available (offline or rate-limited).</span></div>`;
    }

    return `
    <div class="panel-overlay" id="panel-news">
        <div class="panel-handle"></div>
        <div class="panel-header">
            <div class="panel-title-wrap"><span class="panel-title">News & Updates</span><div class="panel-title-underline"></div></div>
            <div class="panel-header-actions">
                <button class="panel-icon-btn ${state.loading ? "spinning" : ""}" ${state.loading ? "disabled" : ""} data-refresh-news data-tooltip="Refresh"><i class="fa-solid fa-rotate"></i></button>
                <button class="panel-back-btn" data-close-panel data-tooltip="Back"><i class="fa-solid fa-chevron-down"></i></button>
            </div>
        </div>
        <div class="panel-body">${body}</div>
        <div class="panel-tabs">${NEWS_PANEL_TABS.map(t => `
            <button class="panel-tab ${t.id === "news" ? "active" : ""}" ${t.id === "home" ? "data-close-panel" : `data-open-panel="${t.id}"`}>
                <i class="${t.icon}"></i>${t.label}
            </button>`).join("")}</div>
    </div>`;
}

// ── Downloads ──────────────────────────────────────────────────────────
// Mirrors Pages/Home.razor's "downloads" panel — a session-scoped list fed
// by DownloadService.cs on desktop; here it's App.state.downloads, pushed
// to whenever a client/mod download starts (see app.js).
const DOWNLOADS_PANEL_TABS = [
    { id: "settings", label: "Settings", icon: "fa-solid fa-gear" },
    { id: "home", label: "Home", icon: "fa-solid fa-house" },
    { id: "downloads", label: "Downloads", icon: "fa-solid fa-download" },
    { id: "credits", label: "Credits", icon: "fa-solid fa-heart" },
];

function downloadRowHtml(entry) {
    const statusIcon = entry.status === "downloading" ? "fa-solid fa-arrow-down" : entry.status === "failed" ? "fa-solid fa-triangle-exclamation" : "fa-solid fa-circle-check";
    const statusLabel = entry.status === "downloading" ? "Downloading" : entry.status === "failed" ? "Failed" : "Complete";
    const sub = entry.status === "downloading" ? `${statusLabel} · ${Math.round(entry.progress * 100)}%` : entry.status === "failed" && entry.error ? `${statusLabel} · ${escapeHtml(entry.error)}` : statusLabel;
    return `
    <div class="version-row">
        <div class="version-meta" style="flex:1; min-width:0;">
            <span class="version-name" style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">${escapeHtml(entry.label)}</span>
            <span class="version-sub"><i class="${statusIcon}"></i> ${sub}</span>
            ${entry.status === "downloading" ? `<div class="progress-bar-wrap" style="margin-top:4px;"><div class="progress-bar-fill" style="width:${Math.round(entry.progress * 100)}%"></div></div>` : ""}
        </div>
        <div class="version-actions">
            <button class="icon-btn icon-btn-ghost" data-tooltip="${entry.status === "downloading" ? "Cancel" : "Remove"}" data-remove-download="${entry.id}">
                <i class="fa-solid ${entry.status === "downloading" ? "fa-xmark" : "fa-trash"}"></i>
            </button>
        </div>
    </div>`;
}

function downloadsPanelHtml(downloads) {
    const hasFinished = downloads.some(d => d.status !== "downloading");
    const body = downloads.length === 0
        ? emptyState("No downloads yet", "Versions, mods, and packs you download this session show up here.", "fa-solid fa-download")
        : downloads.map(downloadRowHtml).join("");

    return `
    <div class="panel-overlay" id="panel-downloads">
        <div class="panel-handle"></div>
        <div class="panel-header">
            <div class="panel-title-wrap"><span class="panel-title">Downloads</span><div class="panel-title-underline"></div></div>
            <div class="panel-header-actions">
                ${hasFinished ? `<button class="panel-icon-btn" data-clear-finished-downloads data-tooltip="Clear finished"><i class="fa-solid fa-broom"></i></button>` : ""}
                <button class="panel-back-btn" data-close-panel data-tooltip="Back"><i class="fa-solid fa-chevron-down"></i></button>
            </div>
        </div>
        <div class="panel-body">${body}</div>
        <div class="panel-tabs">${DOWNLOADS_PANEL_TABS.map(t => `
            <button class="panel-tab ${t.id === "downloads" ? "active" : ""}" ${t.id === "home" ? "data-close-panel" : `data-open-panel="${t.id}"`}>
                <i class="${t.icon}"></i>${t.label}
            </button>`).join("")}</div>
    </div>`;
}

// ── Statistics ─────────────────────────────────────────────────────────
// Mirrors Components/StatsPanel.razor. This app doesn't track play
// sessions yet (no launch-time hooks — see JavaEditionBridge), so every
// figure is the honest zero/empty state rather than a fabricated number:
// same as the desktop panel shows on a completely fresh profile.
function statsPanelBody(totalPlaytimeSeconds) {
    const formatH = (seconds) => {
        if (seconds <= 0) return "0m";
        const h = Math.floor(seconds / 3600);
        const m = Math.floor((seconds % 3600) / 60);
        return h > 0 ? (m > 0 ? `${h}h ${m}m` : `${h}h`) : `${m}m`;
    };
    return `
    <div class="stats-cards">
        <div class="stats-card"><i class="fa-solid fa-stopwatch"></i><span class="stats-val">${formatH(totalPlaytimeSeconds)}</span><span class="stats-label">Total playtime</span></div>
        <div class="stats-card"><i class="fa-solid fa-gamepad"></i><span class="stats-val">0</span><span class="stats-label">Sessions</span></div>
        <div class="stats-card"><i class="fa-solid fa-trophy"></i><span class="stats-val">0m</span><span class="stats-label">Longest session</span></div>
        <div class="stats-card"><i class="fa-solid fa-calendar-day"></i><span class="stats-val">—</span><span class="stats-label">Last played</span></div>
    </div>
    <span class="panel-section-label">Last 14 days</span>
    <div class="stats-empty">No play sessions recorded yet — launch the game to start tracking.</div>`;
}

// ── Logs & Crashes ─────────────────────────────────────────────────────
// Mirrors Components/LogsPanel.razor. Listing real log/crash files needs
// Storage Access Framework wiring to the Java Edition companion app's
// shared storage (queued); mclo.gs sharing (a real public paste API) has
// nothing to share until then.
function logsPanelBody() {
    return `<div class="stats-empty">No logs or crash reports found for the active instance yet.</div>`;
}

// Overlay shell for panels that have no .panel-tabs footer on desktop
// (StatsPanel.razor, LogsPanel.razor) — panelShell() always appends one.
function bareOverlayHtml(id, title, headerActions, body) {
    return `
    <div class="panel-overlay" id="panel-${id}">
        <div class="panel-handle"></div>
        <div class="panel-header">
            <div class="panel-title-wrap">
                <span class="panel-title">${title}</span>
                <div class="panel-title-underline"></div>
            </div>
            <div class="panel-header-actions">
                ${headerActions}
                <button class="panel-back-btn" data-close-panel data-tooltip="Back">
                    <i class="fa-solid fa-chevron-down"></i>
                </button>
            </div>
        </div>
        <div class="panel-body">${body}</div>
    </div>`;
}
