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

function customDllCardHtml(s) {
    const hasFile = !!s.customDllPath;
    const fileName = hasFile ? s.customDllPath.split("/").pop() : "";
    const actionsHtml = hasFile
        ? `<button class="icon-btn icon-btn-ghost" data-tooltip="Clear" data-clear-custom-dll><i class="fa-solid fa-xmark"></i></button>`
        : `<button class="icon-btn icon-btn-ghost" data-tooltip="Pick a .so file" data-pick-custom-dll><i class="fa-solid fa-file-import"></i></button>`;
    return `
    <div class="client-card">
        <div class="client-card-header">
            <div class="client-card-icon client-card-icon-img"><i class="fa-solid fa-file-code" style="color:var(--accent);"></i></div>
            <div class="client-card-meta">
                <span class="client-card-name">Custom Client (.so)</span>
                <span class="client-card-sub">${hasFile ? escapeHtml(fileName) : "No file selected"}</span>
            </div>
            <div class="client-card-actions">${actionsHtml}</div>
        </div>
        <p class="client-card-desc">Same idea as desktop's custom-DLL slot, staged for
        ClientInjectionService's root-based fallback — pick any .so file, then use "Stage for injection"
        below. Requires root; the real non-root technique (package-context + dlopen) isn't built yet.</p>
        ${hasFile ? `<div class="modal-btn-row" style="margin-top:8px;">
            <button class="modal-btn modal-btn-confirm" data-stage-custom-dll><i class="fa-solid fa-upload"></i> Stage for injection</button>
        </div>` : ""}
    </div>`;
}

function clientsPanelBody() {
    // Flarial/Latite/OderSo/LeviLamina are Windows DLL-injected clients (or,
    // for LeviLamina, a native mod loader injected the same way). A real
    // non-root Android equivalent exists (package-context + dlopen, same as
    // other Android Bedrock launchers — see ClientInjectionService.kt's doc
    // comment and android/README.md's gap analysis) but hasn't been built
    // and verified on a device in this pass. The one real injection path
    // that does exist today (root-based file staging) had no way to pick a
    // file at all until customDllCardHtml — everything else stays
    // unwired since there's no client binary to stage for them yet.
    return `
    ${clientCardHtml({
        id: "vanilla", name: "Vanilla",
        iconHtml: `<i class="fa-solid fa-cube" style="color:var(--green);"></i>`,
        statusHtml: `<span class="tag-uptodate"><i class="fa-solid fa-circle-check"></i> Selected</span>`,
        desc: "Pure stock Minecraft Bedrock — the only client this app can launch on Android.",
        actionsHtml: "",
    })}
    ${customDllCardHtml(App.state.settings)}
    <div class="mcv-info-bar">
        <i class="fa-solid fa-circle-info"></i>
        <span>Flarial, Latite, OderSo and LeviLamina aren't wired up yet — not because Android can't do it without root, but because the real technique (re-hosting the installed Minecraft app's own code) is real engineering work this build hasn't shipped. See Settings for details.</span>
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
            ${saved ? `<button class="icon-btn icon-btn-ghost" data-tooltip="Edit" data-edit-server="${s.address}"><i class="fa-solid fa-pen"></i></button>
                <button class="icon-btn icon-btn-ghost" data-tooltip="Delete" data-delete-server="${s.address}"><i class="fa-solid fa-trash"></i></button>`
                : `<button class="icon-btn" data-tooltip="Save" data-save-server="${s.address}"><i class="fa-solid fa-bookmark"></i></button>`}
            <button class="copy-btn" data-tooltip="Copy address" data-copy-address="${s.address}:${s.port}"><i class="fa-solid fa-copy"></i></button>
            <button class="icon-btn" data-tooltip="Launch & connect"><i class="fa-solid fa-play"></i></button>
        </div>
    </div>`;
}

// Add/edit server modal — real markup from Pages/Home.razor's "ADD/EDIT
// SERVER MODAL" region, validated the same way Home.Panels.cs's
// SaveServerModal() does (address required, port 1-65535).
function serverModalHtml(state) {
    if (!state || !state.open) return "";
    const editing = !!state.editingAddress;
    return `
    <div class="modal-overlay" data-close-server-modal-backdrop>
        <div class="modal-box">
            <div class="modal-header">
                <span class="modal-title">
                    <i class="fa-solid fa-server" style="color:var(--accent);margin-right:8px;"></i>
                    ${editing ? "Edit server" : "Add server"}
                </span>
                <button class="modal-close-btn" data-close-server-modal><i class="fa-solid fa-xmark"></i></button>
            </div>
            <div class="modal-body">
                <label class="modal-input-label">Display name</label>
                <input class="modal-input" id="server-modal-name" placeholder="My Server" value="${escapeHtml(state.name)}" />

                <label class="modal-input-label" style="margin-top:8px;">Address (IP or hostname)</label>
                <input class="modal-input" id="server-modal-address" placeholder="play.example.com" value="${escapeHtml(state.address)}" />

                <label class="modal-input-label" style="margin-top:8px;">Port</label>
                <input class="modal-input" id="server-modal-port" type="number" placeholder="19132" value="${state.port}" />

                ${state.error ? `<span class="error-text" style="display:block;margin-top:6px;">${escapeHtml(state.error)}</span>` : ""}

                <div class="modal-btn-row" style="margin-top:14px;">
                    <button class="modal-btn modal-btn-cancel" data-close-server-modal>Cancel</button>
                    <button class="modal-btn modal-btn-confirm" data-save-server-modal><i class="fa-solid fa-floppy-disk"></i> Save</button>
                </div>
            </div>
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

// Shared row template for any paginated content search result (CurseForge,
// Modrinth, ...) — reused by App.renderPagedResults() in app.js so adding a
// new search source only means a config object, not a new markup copy.
function contentResultRowHtml({ name, summary }) {
    return `
    <div class="server-row">
        <div class="server-meta" style="flex:1;">
            <span class="server-name">${escapeHtml(name)}</span>
            <span class="server-sub">${escapeHtml((summary || "").slice(0, 90))}</span>
        </div>
        <button class="icon-btn" data-tooltip="Download & Install"><i class="fa-solid fa-download"></i></button>
    </div>`;
}

// Same >=1M/>=1K/plain-count formatting as Home.razor.cs's FormatDownloads().
function formatDownloads(count) {
    count = Number(count) || 0;
    if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M downloads`;
    if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K downloads`;
    return `${count} downloads`;
}

// Real CurseForge result card — mirrors Home.razor's cfResults @foreach
// (thumbnail, author, download count, category tag, clamped summary,
// download button) instead of the generic server-row list item other
// search sources use. Kept as a distinct template (not routed through
// contentResultRowHtml/App.renderPagedResults' generic mapResult) because
// desktop's CurseForge card carries fields — thumbnail, author, category —
// that plain title/summary can't express.
function curseForgeCardHtml(mod) {
    const thumb = mod.logo?.thumbnailUrl || mod.thumbnailUrl || "";
    const category = (mod.categories && mod.categories[0]?.name) || "";
    const icon = thumb
        ? `<img src="${escapeHtml(thumb)}" style="width:100%;height:100%;object-fit:cover;border-radius:4px;" alt="" />`
        : `<i class="fa-solid fa-cube" style="font-size:18px; color:var(--accent);"></i>`;
    return `
    <div class="client-card" style="margin-bottom:6px;">
        <div class="client-card-header">
            <div class="client-card-icon" style="padding:3px; border-radius:6px; overflow:hidden;">${icon}</div>
            <div class="client-card-meta" style="min-width:0;">
                <span class="client-card-name" style="white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">${escapeHtml(mod.name)}</span>
                <span class="client-card-sub" style="display:flex; gap:8px; align-items:center;">
                    <span style="opacity:0.6;">${escapeHtml(mod.authors?.[0]?.name || "")}</span>
                    <span style="opacity:0.4;">${formatDownloads(mod.downloadCount)}</span>
                    ${category ? `<span class="tag-uptodate" style="font-size:9px; padding:1px 5px;">${escapeHtml(category)}</span>` : ""}
                </span>
            </div>
            <div class="client-card-actions">
                <button class="icon-btn" data-tooltip="Download & Install" data-cf-download="${mod.id}"><i class="fa-solid fa-download"></i></button>
            </div>
        </div>
        <p class="client-card-desc" style="display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;">${escapeHtml(mod.summary || "")}</p>
    </div>`;
}

// Shared shell for a category-switcher + search-input + results-list panel
// body — CurseForge and Modrinth search are the same layout with different
// category lists/attributes/placeholders/result containers.
function categorySearchShell({ categories, categoryAttr, categoriesId, inputId, placeholder, resultsId }) {
    return `
    <div class="versions-client-switcher" id="${categoriesId}">
        ${categories.map((c, i) => `<button class="vcs-btn ${i === 0 ? "active" : ""}" data-${categoryAttr}="${c.value}"><i class="${c.icon}"></i> ${c.label}</button>`).join("")}
    </div>
    <div class="panel-search-wrap">
        <i class="fa-solid fa-magnifying-glass"></i>
        <input class="panel-search-input" id="${inputId}" placeholder="${placeholder}" />
    </div>
    <div id="${resultsId}"></div>`;
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
    return categorySearchShell({
        categories: cats.map(c => ({ value: c.classId, icon: c.icon, label: c.label })),
        categoryAttr: "cf-category", categoriesId: "cf-categories", inputId: "cf-search-input",
        placeholder: `Search CurseForge ${App.state.edition === "java" ? "for Java mods, modpacks, shaders..." : "addons..."}`,
        resultsId: "cf-results",
    });
}

function modrinthSearchBody() {
    return categorySearchShell({
        categories: Modrinth.javaCategories.map(c => ({ value: c.facet, icon: c.icon, label: c.label })),
        categoryAttr: "mr-category", categoriesId: "mr-categories", inputId: "mr-search-input",
        placeholder: "Search Modrinth...", resultsId: "mr-results",
    });
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

// Real counts and sizes from JavaInstanceService.listAssetFolders(). These
// were disabled cards captioned "Needs Storage Access Framework wiring to
// shared storage", which had the storage model wrong: Java Edition's data
// lives at a real path under Pojav's own directory on primary external
// storage, reachable with the MANAGE_EXTERNAL_STORAGE permission this app
// already declares. SAF is only needed for Bedrock, whose data sits inside
// another app's Android/data sandbox.
const JAVA_ASSET_STYLES = {
    "Resource Packs": { icon: "fa-solid fa-image", color: "rgba(67,181,129,0.15)", fg: "var(--green)" },
    "Shader Packs": { icon: "fa-solid fa-wand-magic-sparkles", color: "rgba(255,200,50,0.15)", fg: "#ffc832" },
    "Saves / Worlds": { icon: "fa-solid fa-map", color: "rgba(114,137,218,0.15)", fg: "var(--accent)" },
    "Screenshots": { icon: "fa-solid fa-camera", color: "rgba(240,71,71,0.15)", fg: "var(--red)" },
    "Schematics": { icon: "fa-solid fa-drafting-compass", color: "rgba(250,166,26,0.15)", fg: "var(--orange)" },
};

function javaAssetsBody() {
    let rows;
    try { rows = JSON.parse(Bridge.listJavaAssetFolders() || "[]"); } catch (e) { rows = []; }
    if (rows.length === 0) {
        return emptyState("No Java instance yet", "Create one from the Java Versions panel first.", "fa-solid fa-folder-open");
    }
    return rows.map(r => {
        const style = JAVA_ASSET_STYLES[r.name] || { icon: "fa-solid fa-folder", color: "rgba(114,137,218,0.15)", fg: "var(--accent)" };
        const sub = !r.exists
            ? "Not created yet — opening it will make the folder"
            : `${r.count} item${r.count === 1 ? "" : "s"} · ${formatBytes(r.sizeBytes)}`;
        return `
    <div class="client-card">
        <div class="client-card-header">
            <div class="client-card-icon" style="background:${style.color}; color:${style.fg}; padding:8px;"><i class="${style.icon}"></i></div>
            <div class="client-card-meta">
                <span class="client-card-name">${escapeHtml(r.name)}</span>
                <span class="client-card-sub" style="opacity:0.5;">${escapeHtml(sub)}</span>
            </div>
            <div class="client-card-actions">
                <button class="icon-btn" data-open-java-asset="${escapeHtml(r.folder)}" data-tooltip="Open folder"><i class="fa-solid fa-folder-open"></i></button>
            </div>
        </div>
    </div>`;
    }).join("");
}

// Backed by JavaInstanceService.kt's backupSaves/exportModpack/duplicate,
// which are ports of the desktop service's BackupSavesAsync/
// ExportModpackAsync/Duplicate. These were rendered as permanently disabled
// cards; they are plain file I/O over the instance directory, so nothing
// blocked them beyond not being written yet.
function javaToolsBody() {
    const rows = [
        { id: "backup-saves", name: "Backup Saves", sub: "Zip your worlds before a risky update", icon: "fa-solid fa-box-archive", color: "rgba(114,137,218,0.15)", fg: "var(--accent)", action: "fa-solid fa-download" },
        { id: "export-modpack", name: "Export Modpack", sub: "Bundle mods + config into a shareable zip", icon: "fa-solid fa-file-zipper", color: "rgba(67,181,129,0.15)", fg: "var(--green)", action: "fa-solid fa-file-export" },
        { id: "duplicate-instance", name: "Duplicate Instance", sub: "Clone your active instance with all mods & config", icon: "fa-solid fa-clone", color: "rgba(250,166,26,0.15)", fg: "var(--orange)", action: "fa-solid fa-copy" },
    ];
    return rows.map(r => `
    <div class="client-card">
        <div class="client-card-header">
            <div class="client-card-icon" style="background:${r.color}; color:${r.fg}; padding:8px;"><i class="${r.icon}"></i></div>
            <div class="client-card-meta">
                <span class="client-card-name">${r.name}</span>
                <span class="client-card-sub" style="opacity:0.5;" data-java-tool-status="${r.id}">${r.sub}</span>
            </div>
            <div class="client-card-actions">
                <button class="icon-btn" data-java-tool="${r.id}" data-tooltip="${r.name}"><i class="${r.action}"></i></button>
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
        case "datapacks": body = emptyState("Datapacks", "Needs a world picker wired to the built-in Java Edition runtime's shared storage — queued.", "fa-solid fa-cubes-stacked"); break;
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
        ? { id: "clients", label: "Clients", icon: "fa-solid fa-puzzle-piece" }
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

// Where Glacier keeps everything (GlacierStorage.kt). Pojav's own default
// was this app's private Android/data sandbox, which Android 11+ hides from
// file managers, so worlds and mods were effectively unreachable. They now
// live in games/Glacier — but writing outside the sandbox needs All Files
// Access on Android 11+, and without it the launcher stays on the old
// private folder rather than breaking Java Edition with an unwritable root.
// So the row shows the path actually in use and offers the toggle only when
// it would change something.
function glacierStorageRow() {
    const path = Bridge.glacierStoragePath();
    const shared = Bridge.glacierStorageIsShared();
    const control = shared
        ? `<span class="client-card-note">Shared folder</span>`
        : `<button class="btn-sm" id="grant-all-files">Grant access</button>`;
    const hint = shared
        ? `Worlds, mods, instances, exports and this launcher's config all live here.`
        : `Currently in this app's private folder, which file managers can't open.
           Granting all-files access moves everything to <code>games/Glacier</code> on the next launch.`;
    return settingRow("Storage folder", `${escapeHtml(path)}<br>${hint}`, control);
}

function settingsPanelBody(category) {
    const s = App.state.settings;
    const cat = (id) => category === "all" || category === id;
    let html = "";

    if (cat("clients")) {
        html += `<div class="settings-section"><span class="panel-section-label">Clients</span>
        ${settingRow("Active client", "Vanilla — the only client this app can launch", `<span class="client-card-note">Vanilla</span>`)}
        ${settingRow("Close after launch", "Minimise the launcher once Minecraft starts", toggleHtml("closeAfterLaunch", s.closeAfterLaunch))}
        </div>
        <div class="settings-section"><span class="panel-section-label">Why no Flarial / Latite / OderSo / LeviLamina?</span>
        <div style="padding:4px 0; font-size:12px; color:var(--text-dim); line-height:1.5;">
            All four work by loading a native DLL into Minecraft's own process on Windows
            (<code>CreateRemoteThread</code> + <code>LoadLibrary</code>). Android sandboxes every
            app by UID, so there's no cross-process equivalent — but that doesn't mean this needs
            root: real non-root Android Bedrock launchers re-host the installed, licensed
            Minecraft app's own code inside their own process (a package-context + <code>dlopen</code>
            technique) and hook it the same way a Windows DLL hooks Minecraft.Windows.exe. That's
            real engineering work this build hasn't shipped and verified on a device yet — see
            android/README.md's gap analysis — so there's nothing honest to wire a "select client"
            button to here yet, rather than nothing possible in principle.
        </div>
        </div>`;
    }

    if (cat("java")) {
        html += `<div class="settings-section"><span class="panel-section-label">Java Edition</span>
        <div style="padding:8px 0;">RAM, JVM args, resolution, offline mode, and version filters are configured in the Java Edition runtime's own in-game settings.</div>
        ${settingRow("Java Edition runtime", "Built in — no separate app to install", `<button class="btn-sm" id="open-java-edition">Open</button>`)}
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
        ${settingRow("Theme Studio", "Build a fully custom theme — every color, radius and font is editable", `<button class="btn-sm" data-open-panel="themestudio">Open</button>`)}
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
        ${settingRow("Xbox profile", s.xboxGamertag || "Not signed in", `<button class="btn-sm" id="xbox-sign-in-settings">Sign in</button>`)}
        ${settingRow("Skin Library", "Saved skins — apply to your account", `<button class="btn-sm" data-open-panel="skinlibrary">Open</button>`)}
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
        ${glacierStorageRow()}
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
// filter, version rows), but not its download/switch/delete buttons acting
// on THIS list — those are AppX registration + Windows-Update-SOAP-API
// operations (VanillaVersionService.cs) with no Android equivalent at all.
// What Android has instead is real, just a different mechanism: import your
// own Bedrock APK and install it directly (bedrockBuildManagerHtml above,
// BedrockVersionService.kt) — a real PackageInstaller flow, same one
// LauncherUpdateService.kt already uses for self-updates. This list stays
// read-only info (mcVersionInfoRowHtml below) since it has no file behind
// each entry to install, only version metadata from the community DB.

// Real version names/channels come from the same public community DB
// desktop's VanillaVersionService.cs reads (BedrockVersions.fetch(), in
// javaedition.js) — this list itself has no APK behind each entry (just
// metadata), so it stays an info-only row rather than a fake download
// button; bedrockBuildManagerHtml above is where an actual installable APK
// (the user's own) gets managed.
function mcVersionInfoRowHtml(v) {
    return `
    <div class="version-row">
        <div class="version-meta">
            <span class="version-name">Minecraft ${escapeHtml(v.id)}</span>
            <span class="version-sub">${v.channel === "preview" ? "Preview" : "Release"}</span>
        </div>
    </div>`;
}

// Android-only: manages side-loaded Bedrock APKs (BedrockVersionService.kt)
// — no desktop equivalent markup to mirror, since desktop's version
// switching is a Windows Store AppX operation. Kept visually consistent
// with the existing .instance-card pattern (Java Instances, same panel
// family) rather than inventing a new card style.
function bedrockBuildRowHtml(b) {
    return `
    <div class="instance-row">
        <div style="display:flex; flex-direction:column; flex:1; min-width:0;">
            <span style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">${escapeHtml(b.versionName)} <span style="color:var(--text-dim); font-size:11px;">(${b.versionCode})</span></span>
            ${!b.matchesInstalledPackage ? `<span style="color:var(--yellow); font-size:10px;"><i class="fa-solid fa-triangle-exclamation"></i> Package: ${escapeHtml(b.packageName)} — won't replace Bedrock</span>` : ""}
        </div>
        <button class="instance-icon-btn" data-install-bedrock-apk="${escapeHtml(b.fileName)}" data-tooltip="Install"><i class="fa-solid fa-download"></i></button>
        <button class="instance-icon-btn instance-del" data-delete-bedrock-apk="${escapeHtml(b.fileName)}" data-tooltip="Delete"><i class="fa-solid fa-trash"></i></button>
    </div>`;
}

function bedrockBuildManagerHtml(state) {
    return `
    <div class="instance-card">
        <div class="instance-card-head">
            <span><i class="fa-solid fa-box-open"></i> Local builds</span>
            <div style="flex:1"></div>
            <button class="btn-sm" data-backup-bedrock-apk data-tooltip="Back up the currently-installed Bedrock APK">
                <i class="fa-solid fa-shield-halved"></i> Backup now
            </button>
            <button class="btn-sm" data-import-bedrock-apk ${state.importing ? "disabled" : ""}>
                ${state.importing ? `<span class="spinner"></span>` : `<i class="fa-solid fa-plus"></i>`} Import APK
            </button>
        </div>
        ${state.statusMessage ? `<div style="font-size:11px; color:var(--text-dim); padding:4px 2px;">${escapeHtml(state.statusMessage)}</div>` : ""}
        ${state.builds.length === 0
            ? `<div class="stats-empty">No local builds imported yet — import a Bedrock APK to install it directly, bypassing Play Store's always-current version.</div>`
            : state.builds.map(bedrockBuildRowHtml).join("")}
    </div>`;
}

// Full panel-overlay markup, not routed through panelShell(): the desktop
// panel puts the info bar / search / channel tabs BETWEEN .panel-header and
// .panel-body (siblings, not nested inside it), unlike the other panels.
function mcVersionsPanelHtml(channel, filter, versions, loading, bedrockVersions) {
    const filtered = versions.filter(v =>
        (channel === "all" || v.channel === channel) &&
        (!filter || v.id.toLowerCase().includes(filter.toLowerCase())));

    const listHtml = loading
        ? `<div class="versions-loading"><span class="spinner"></span><span>Loading versions…</span></div>`
        : filtered.length === 0
        ? `<div class="versions-loading"><span style="opacity:0.5;">${
            versions.length === 0 ? "No versions found." : `No results for "${filter}".`
        }</span></div>`
        : filtered.map(mcVersionInfoRowHtml).join("");

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
            <span>Play Store keeps Bedrock always current with no built-in rollback — import your own
            APK below to install a specific build instead (a real downgrade needs root; without it,
            Android's own installer only accepts an equal-or-newer version). The read-only list further
            down is the same public version-history database the desktop app reads.</span>
        </div>
        ${bedrockBuildManagerHtml(bedrockVersions)}
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
    const jeActions = `<button class="vcs-btn" data-open-java-edition><i class="fa-solid fa-play"></i> Open</button>`;
    const jeSub = `<span style="color:var(--green);"><i class="fa-solid fa-circle-check"></i> Built in</span>`;
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
        <span>Vanilla and Glacier Client launch through Glacier's built-in Java Edition runtime. Lunar Client and Badlion don't ship an Android build — there's no client to detect or launch here.</span>
    </div>
    <div class="panel-body">
        <span class="panel-section-label">Built-in</span>
        <div class="credits-card credits-card-glacier">
            <div class="credits-card-icon" style="background:var(--accent-bg); color:var(--accent);"><i class="fa-brands fa-java"></i></div>
            <div class="credits-card-meta">
                <span class="credits-card-name">Vanilla (Glacier built-in)</span>
                <span class="credits-card-sub">${jeSub}</span>
            </div>
            <div class="credits-card-actions">
                ${jeActions}
                <button class="vcs-btn vcs-btn-ghost" data-open-panel="javaversions"><i class="fa-solid fa-box-archive"></i> Versions</button>
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
// Install/Launch hand off to the built-in Java Edition runtime (it owns
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
        <button class="vcs-btn" data-launch-java-version="${escapeHtml(v.id)}"><i class="fa-solid fa-play"></i>&nbsp;Launch</button>
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
// Mirrors Pages/Home.razor's "javaprofile" panel. Sign-in itself is real
// (MainActivity.kt's signInMicrosoft() + js/xboxauth.js — same OAuth2/XBL/
// XSTS flow as desktop's LiveAuthService.cs). The 3D skin preview (desktop's
// Components/SkinViewer.razor, backed by skinview3d) is now ported too, via
// js/skinviewer.js's GlacierSkin (same vendored-bundle-first load strategy) —
// 2D static render <-> 3D interactive toggle, Steve/Alex model switch. Cape
// display is left out: Android has no cape/wardrobe data anywhere in
// AndroidBridge or settings for the 3D model to show.
const SKIN_VIEWER_CANVAS_ID = "java-profile-skin-canvas";

function skinViewerHtml(s) {
    const uuid = (s.javaUuid || "").replace(/-/g, "");
    const mode = s.skinViewerMode === "3d" ? "3d" : "2d";
    const model = s.skinViewerModel === "slim" ? "slim" : "default";
    const bodyUrl = `https://mc-heads.net/body/${uuid}/right`;
    const stage = mode === "3d"
        ? `<canvas id="${SKIN_VIEWER_CANVAS_ID}" class="skin-canvas"></canvas>`
        : `<img class="skin-body" src="${bodyUrl}" alt="${escapeHtml(s.javaUsername || "Player")} skin"
             onerror="this.onerror=null;this.src='https://crafatar.com/renders/body/${uuid}?overlay&scale=10';" />`;
    return `
    <div class="skin-viewer">
        <div class="skin-stage">${stage}</div>
        <div class="skin-pills">
            <button class="skin-pill ${mode === "2d" ? "active" : ""}" data-skin-viewer-mode="2d">2D</button>
            <button class="skin-pill ${mode === "3d" ? "active" : ""}" data-skin-viewer-mode="3d">3D</button>
            <span class="skin-pill-sep"></span>
            <button class="skin-pill ${model === "default" ? "active" : ""}" data-skin-viewer-model="default" data-tooltip="Classic model">Steve</button>
            <button class="skin-pill ${model === "slim" ? "active" : ""}" data-skin-viewer-model="slim" data-tooltip="Slim model">Alex</button>
        </div>
    </div>`;
}
function instanceRowHtml(inst, state) {
    if (state.renamingId === inst.id) {
        return `
        <div class="instance-row ${inst.isActive ? "active" : ""}">
            <input class="setting-input instance-rename" value="${escapeHtml(state.renameValue)}" data-rename-instance-input />
            <button class="vcs-btn" data-commit-rename-instance="${inst.id}"><i class="fa-solid fa-check"></i></button>
        </div>`;
    }
    const confirming = state.confirmDeleteId === inst.id;
    return `
    <div class="instance-row ${inst.isActive ? "active" : ""}">
        <button class="instance-name-btn" data-switch-instance="${inst.id}">
            <i class="fa-solid ${inst.isActive ? "fa-circle-check" : "fa-regular fa-circle"}"></i>
            <span>${escapeHtml(inst.name)}</span>
        </button>
        <button class="instance-icon-btn" data-rename-instance="${inst.id}" data-rename-instance-name="${escapeHtml(inst.name)}" data-tooltip="Rename"><i class="fa-solid fa-pen"></i></button>
        ${confirming
            ? `<button class="instance-icon-btn instance-del" data-delete-instance="${inst.id}" data-tooltip="Confirm delete"><i class="fa-solid fa-check"></i></button>
               <button class="instance-icon-btn" data-cancel-delete-instance data-tooltip="Cancel"><i class="fa-solid fa-xmark"></i></button>`
            : `<button class="instance-icon-btn instance-del" data-confirm-delete-instance="${inst.id}" data-tooltip="Delete"><i class="fa-solid fa-trash"></i></button>`}
    </div>`;
}

function javaInstancesCardHtml(state) {
    return `
    <div class="instance-card">
        <div class="instance-card-head">
            <span><i class="fa-solid fa-cubes-stacked"></i> Instances</span>
            <div style="flex:1"></div>
            <button class="btn-sm" data-new-instance><i class="fa-solid fa-plus"></i> New</button>
        </div>
        ${state.instances.map(inst => instanceRowHtml(inst, state)).join("")}
    </div>`;
}

function javaProfilePanelBody() {
    const s = App.state.settings;
    const auth = App.state.msAuth;
    const instancesCard = javaInstancesCardHtml(App.state.javaInstances);

    if (auth.loading) {
        return `<div class="skin-empty"><span class="spinner"></span><span style="margin-top:10px;">Signing in…</span></div>`;
    }

    if (!s.javaUsername) {
        return `<div class="skin-empty">
            <i class="fa-solid fa-user-astronaut"></i>
            <span>Sign in with your Microsoft account to view your Minecraft profile.</span>
            ${auth.error ? `<span class="error-text" style="max-width:280px; text-align:center;">${escapeHtml(auth.error)}</span>` : ""}
            <button class="vcs-btn" id="profile-signin-btn">
                <i class="fa-brands fa-xbox"></i>&nbsp;Sign in
            </button>
        </div>
        ${instancesCard}`;
    }

    return `<div class="profile-actions" style="flex-direction:column; align-items:stretch; gap:10px;">
        ${skinViewerHtml(s)}
        <div style="display:flex; align-items:center; gap:12px;">
            <div>
                <div style="font-weight:600;">${escapeHtml(s.javaUsername)}</div>
                <div style="font-size:11px;color:var(--text-dim);">${escapeHtml(s.xboxGamertag || "")}${s.xboxGamerscore ? ` · ${escapeHtml(s.xboxGamerscore)}G` : ""}</div>
            </div>
        </div>
        <div class="pstat">
            <i class="fa-solid fa-fingerprint"></i>
            <span class="pstat-k">UUID</span>
            <span class="pstat-v">${escapeHtml((s.javaUuid || "").slice(0, 12))}…</span>
        </div>
        <div class="profile-actions">
            <button class="skin-btn" data-open-panel="skinlibrary"><i class="fa-solid fa-shirt"></i> Skin library</button>
            <button class="skin-btn skin-btn-ghost" id="profile-signout-btn"><i class="fa-solid fa-right-from-bracket"></i> Sign out</button>
        </div>
        <div class="profile-actions">
            <button class="skin-btn skin-btn-ghost" data-open-panel="stats"><i class="fa-solid fa-chart-simple"></i> Statistics</button>
            <button class="skin-btn skin-btn-ghost" data-open-panel="logs"><i class="fa-solid fa-file-lines"></i> Logs</button>
        </div>
    </div>
    ${instancesCard}`;
}

// ── Java Screenshots ──────────────────────────────────────────────────────
function javaScreenshotsPanelBody() {
    return emptyState("No screenshots yet", "Press F2 in-game to capture one — they'll show up here once wired to shared storage.", "fa-solid fa-image");
}

// ── Onboarding wizard ─────────────────────────────────────────────────────
// Real markup/classes from Pages/Home.razor's onboarding modal, minus the
// "Import existing .minecraft" step: that assumes a prior install of
// Mojang's official Java launcher on the same machine, which doesn't exist
// on Android at all (Java Edition here only exists inside this app's own
// built-in runtime) — so edition pick + username are the whole wizard.
function onboardingModalHtml(state) {
    if (!state.open) return "";
    const lastStep = 1;
    const step0 = `
        <p style="color:var(--text-dim); font-size:12px; margin:0 0 10px;">Which edition of Minecraft do you play?</p>
        <div style="display:flex; gap:8px;">
            <button class="btn-sm ${state.edition === "bedrock" ? "btn-accent" : ""}" style="flex:1;" data-onboarding-pick-edition="bedrock">
                <i class="fa-solid fa-cube"></i> Bedrock
            </button>
            <button class="btn-sm ${state.edition === "java" ? "btn-accent" : ""}" style="flex:1;" data-onboarding-pick-edition="java">
                <i class="fa-solid fa-mug-hot"></i> Java
            </button>
        </div>
        <small style="color:var(--text-dim); display:block; margin-top:8px;">You can switch editions anytime from the home screen.</small>`;
    const step1 = `
        <label class="modal-input-label">Display name</label>
        <input class="modal-input" id="onboarding-username-input" placeholder="Player" value="${escapeHtml(state.username)}" />
        <small style="color:var(--text-dim); display:block; margin-top:8px;">Shown in the launcher's footer and recent-launches list. Optional.</small>`;

    return `
    <div class="search-overlay">
        <div class="modal-box" style="width:340px;">
            <div class="modal-header">
                <span class="modal-title"><i class="fa-solid fa-snowflake" style="color:var(--accent);margin-right:8px;"></i>Welcome to Glacier</span>
                <button class="modal-close-btn" data-onboarding-skip data-tooltip="Skip setup"><i class="fa-solid fa-xmark"></i></button>
            </div>
            <div class="modal-body">${state.step === 0 ? step0 : step1}</div>
            <div class="modal-actions">
                ${state.step > 0
                    ? `<button class="modal-btn modal-btn-cancel" data-onboarding-back>Back</button>`
                    : `<button class="modal-btn modal-btn-cancel" data-onboarding-skip>Skip</button>`}
                ${state.step < lastStep
                    ? `<button class="modal-btn modal-btn-confirm" data-onboarding-next>Next</button>`
                    : `<button class="modal-btn modal-btn-confirm" data-onboarding-finish><i class="fa-solid fa-check"></i> Finish</button>`}
            </div>
        </div>
    </div>`;
}

// ── Announcement / maintenance banner ────────────────────────────────────
// Real markup from Pages/Home.razor's ANNOUNCEMENT / MAINTENANCE BANNER
// block. A "maintenance" kind has no dismiss button, matching desktop —
// it's meant to stay visible for the duration of the outage it describes.
function announcementBannerHtml(announcement, dismissedId) {
    if (!announcement || announcement.id === dismissedId) return "";
    const icon = announcement.kind === "maintenance" ? "fa-solid fa-triangle-exclamation" : "fa-solid fa-bullhorn";
    return `
    <div class="announcement-banner ${escapeHtml(announcement.kind || "")}">
        <i class="${icon}"></i>
        <div class="announcement-banner-text">
            <span class="announcement-banner-title">${escapeHtml(announcement.title)}</span>
            <span class="announcement-banner-msg">${escapeHtml(announcement.message)}</span>
        </div>
        ${announcement.url ? `<button class="btn-sm" data-open-url="${escapeHtml(announcement.url)}">Learn more</button>` : ""}
        ${announcement.kind !== "maintenance" ? `<button class="announcement-banner-close" data-dismiss-announcement="${escapeHtml(announcement.id)}" data-tooltip="Dismiss"><i class="fa-solid fa-xmark"></i></button>` : ""}
    </div>`;
}

// ── Bedrock Worlds ───────────────────────────────────────────────────────
// Read half of Pages/Home.razor's "bedrockworlds" panel (world-icon markup
// is real: .version-row/.version-icon/.version-meta from the same "worlds"
// block desktop uses) — see BedrockStorage (bedrockstorage.js) and
// BedrockStorageService.kt for how the data itself is fetched.
function bedrockWorldRowHtml(w) {
    const iconHtml = w.iconUri
        ? `<img class="version-icon" src="${w.iconUri}" alt="" />`
        : `<div class="version-icon version-icon-placeholder"><i class="fa-solid fa-globe"></i></div>`;
    const editor = App.state.bedrockWorlds.editing === w.id
        ? levelDatEditorHtml(App.state.bedrockWorlds.levelDat)
        : "";
    return `
    <div class="version-row">
        ${iconHtml}
        <div class="version-meta">
            <span class="version-name">${escapeHtml(w.name)}</span>
            <span class="version-sub">${formatBytes(w.sizeBytes)} · ${formatRelativeTime(w.lastPlayed)}</span>
        </div>
        <button class="icon-btn" data-edit-leveldat="${escapeHtml(w.id)}" data-tooltip="Edit level.dat">
            <i class="fa-solid fa-sliders"></i></button>
    </div>${editor}`;
}

// The level.dat editor (desktop's LevelDatSummary panel), expanded inline
// under whichever world row is being edited. Fields and their value sets
// mirror LevelDatEditorService.cs exactly; the seed is shown but read-only,
// as it is on desktop — changing it on an existing world doesn't regenerate
// already-generated chunks, it just desynchronises new ones.
const LEVELDAT_GAME_TYPES = [["Survival", 0], ["Creative", 1], ["Adventure", 2]];
const LEVELDAT_DIFFICULTIES = [["Peaceful", 0], ["Easy", 1], ["Normal", 2], ["Hard", 3]];

function levelDatEditorHtml(state) {
    if (!state) return "";
    if (state.loading) {
        return `<div class="versions-loading"><span class="spinner"></span><span>Reading level.dat…</span></div>`;
    }
    if (!state.ok) {
        return `<div class="setting-row"><div class="setting-meta">
            <span class="setting-hint" style="color:var(--red);">${escapeHtml(state.error || "Couldn't read level.dat.")}</span>
        </div></div>`;
    }
    const chips = (options, current, attr) => options.map(([label, value]) =>
        `<button class="ts-chip ${value === current ? "selected" : ""}" ${attr}="${value}">${label}</button>`).join("");

    const experimentRows = Object.keys(state.experiments || {}).map(name => `
        <div class="setting-row">
            <div class="setting-meta"><span class="setting-label">${escapeHtml(name)}</span></div>
            <div class="toggle ${state.experiments[name] ? "on" : ""}" data-leveldat-experiment="${escapeHtml(name)}"></div>
        </div>`).join("");

    return `<div class="leveldat-editor" style="padding:4px 12px 12px;">
        <div class="setting-row">
            <div class="setting-meta"><span class="setting-label">Game mode</span></div>
            <div class="ts-chips">${chips(LEVELDAT_GAME_TYPES, state.gameType, "data-leveldat-gametype")}</div>
        </div>
        <div class="setting-row">
            <div class="setting-meta"><span class="setting-label">Difficulty</span></div>
            <div class="ts-chips">${chips(LEVELDAT_DIFFICULTIES, state.difficulty, "data-leveldat-difficulty")}</div>
        </div>
        <div class="setting-row">
            <div class="setting-meta"><span class="setting-label">Cheats</span>
                <span class="setting-hint">commandsEnabled</span></div>
            <div class="toggle ${state.cheats ? "on" : ""}" data-leveldat-cheats></div>
        </div>
        ${state.hasSeed ? `<div class="setting-row">
            <div class="setting-meta"><span class="setting-label">Seed</span>
                <span class="setting-hint">Read-only — changing it desynchronises new chunks from existing ones</span></div>
            <span class="client-card-note">${escapeHtml(String(state.seed))}</span>
        </div>` : ""}
        ${experimentRows ? `<div class="panel-section-label">Experiments</div>${experimentRows}` : ""}
        <div style="display:flex; gap:6px; margin-top:10px;">
            <button class="modal-btn modal-btn-confirm" data-leveldat-save>Save</button>
            <button class="btn-sm" data-leveldat-close>Cancel</button>
            <span class="setting-hint" style="align-self:center;" data-leveldat-status>${escapeHtml(state.status || "")}</span>
        </div>
    </div>`;
}

// Storage-access guidance. The picker is opened directly inside
// Android/data/com.mojang.minecraftpe/files/games/com.mojang via
// EXTRA_INITIAL_URI (see BedrockStorageService.bedrockPickerInitialUri) —
// that path cannot be navigated to by hand, because Android 11+ blocks
// /Android in the picker entirely, so telling the user to "go find it"
// was asking for something the OS forbids.
//
// Android 13 closed the subdirectory loophole this relies on, so there the
// picker still opens in the right place but the grant is refused. Say that
// plainly instead of letting the user retry something that cannot work.
function bedrockAccessHint() {
    return Bridge.bedrockStorageBlockedByPlatform()
        ? `Android 13 and newer block apps from being granted access to
           <code>Android/data</code>, where Minecraft keeps its worlds. Tapping below still
           opens the folder, but the system may refuse the grant — if it does, the only
           non-root options are Shizuku or copying the <code>com.mojang</code> folder to
           ordinary storage with a file manager first.`
        : `Tapping below opens Minecraft's data folder directly — you only need to confirm
           with "Use this folder". Don't navigate elsewhere first: Android blocks
           <code>Android/data</code> from being browsed by hand.`;
}

function bedrockWorldsPanelBody(state) {
    if (!state.hasAccess) {
        return `<div class="empty-state" style="padding:20px 20px 8px;">
            <i class="fa-solid fa-folder-open"></i>
            <span>Grant access to your worlds</span>
            <small>${bedrockAccessHint()}</small>
            <button class="modal-btn modal-btn-confirm" style="margin-top:12px;" data-grant-bedrock-storage>Grant Access</button>
        </div>`;
    }
    if (state.loading) {
        return `<div class="versions-loading"><span class="spinner"></span><span>Loading worlds…</span></div>`;
    }
    if (state.worlds.length === 0) {
        return emptyState("No worlds yet", "Worlds you create in Minecraft show up here.", "fa-solid fa-globe");
    }
    return state.worlds.map(bedrockWorldRowHtml).join("");
}

// ── Bedrock Packs ────────────────────────────────────────────────────────
// Read half of Pages/Home.razor's "bedrockpacks" panel — real markup
// (.ts-chips kind switcher, .version-row pack rows) fed by
// BedrockStorageService.kt's manifest.json reads (see bedrockstorage.js).
const BEDROCK_PACK_KINDS = [
    { id: "resource", label: "Resource", icon: "fa-solid fa-palette" },
    { id: "behavior", label: "Behavior", icon: "fa-solid fa-gears" },
    { id: "skin", label: "Skin", icon: "fa-solid fa-shirt" },
    { id: "resource-dev", label: "Resource (dev)", icon: "fa-solid fa-code" },
    { id: "behavior-dev", label: "Behavior (dev)", icon: "fa-solid fa-code" },
    { id: "skin-dev", label: "Skin (dev)", icon: "fa-solid fa-code" },
];

function bedrockPackRowHtml(p) {
    const iconHtml = p.iconUri
        ? `<img class="version-icon" src="${p.iconUri}" alt="" />`
        : `<div class="version-icon version-icon-placeholder"><i class="fa-solid fa-boxes-stacked"></i></div>`;
    return `
    <div class="version-row">
        ${iconHtml}
        <div class="version-meta">
            <span class="version-name">${escapeHtml(p.name)}</span>
            <span class="version-sub">${formatBytes(p.sizeBytes)}</span>
        </div>
    </div>`;
}

function bedrockPacksPanelBody(state) {
    const chipsHtml = `<div class="ts-chips" style="padding:10px 14px 0;">${BEDROCK_PACK_KINDS.map(k => `
        <button class="ts-chip ${state.kind === k.id ? "selected" : ""}" data-bedrock-pack-kind="${k.id}">
            <i class="${k.icon}"></i> ${k.label}
        </button>`).join("")}</div>`;

    let bodyHtml;
    if (!state.hasAccess) {
        bodyHtml = `<div class="empty-state" style="padding:20px 20px 8px;">
            <i class="fa-solid fa-folder-open"></i>
            <span>Grant access to your packs</span>
            <small>${bedrockAccessHint()}</small>
            <button class="modal-btn modal-btn-confirm" style="margin-top:12px;" data-grant-bedrock-storage>Grant Access</button>
        </div>`;
    } else if (state.loading) {
        bodyHtml = `<div class="versions-loading"><span class="spinner"></span><span>Loading packs…</span></div>`;
    } else if (state.packs.length === 0) {
        bodyHtml = `<div class="empty-state" style="padding:20px 20px 8px;">
            <i class="fa-solid fa-boxes-stacked"></i>
            <span>No ${state.kind} packs yet</span>
            <small>Import a .mcpack, or install one from CurseForge/Modrinth in Addons.</small>
        </div>`;
    } else {
        bodyHtml = state.packs.map(bedrockPackRowHtml).join("");
    }

    return `${chipsHtml}${bodyHtml}`;
}

// ── Bedrock Backups ──────────────────────────────────────────────────────
// Mirrors Pages/Home.razor's "bedrockbackups" panel — create + list +
// delete are real (BedrockBackupService.kt zips worlds/packs read through
// the same SAF grant as Worlds/Packs into this app's own storage, which
// needs no SAF *write* access at all). Restore is not implemented yet —
// writing back into com.mojang through SAF is a meaningfully bigger, more
// dangerous change (a bug there risks wiping real worlds) than a read-only
// listing, so this only offers create/delete for now.
function bedrockBackupRowHtml(b, confirmDeleteName) {
    const confirming = confirmDeleteName === b.fileName;
    return `
    <div class="version-row">
        <div class="version-meta">
            <span class="version-name">${escapeHtml(b.name)}</span>
            <span class="version-sub">${formatBytes(b.sizeBytes)} · ${formatRelativeTime(Math.floor(b.createdAt / 1000))}</span>
        </div>
        <div class="version-actions">
            ${confirming
                ? `<button class="icon-btn icon-btn-ghost" data-tooltip="Confirm delete" data-delete-bedrock-backup="${b.fileName}"><i class="fa-solid fa-check"></i></button>
                   <button class="icon-btn icon-btn-ghost" data-tooltip="Cancel" data-cancel-delete-bedrock-backup><i class="fa-solid fa-xmark"></i></button>`
                : `<button class="icon-btn icon-btn-ghost" data-tooltip="Delete" data-confirm-delete-bedrock-backup="${b.fileName}"><i class="fa-solid fa-trash"></i></button>`}
        </div>
    </div>`;
}

function bedrockBackupsPanelBody(state) {
    const createRowHtml = `
    <div style="display:flex; gap:8px; padding:12px 14px 4px;">
        <button class="btn-sm btn-blue" ${state.creating ? "disabled" : ""} data-create-bedrock-backup>
            ${state.creating ? `<span class="spinner"></span>` : `<i class="fa-solid fa-camera"></i>`} Back up now
        </button>
    </div>`;

    if (!state.hasAccess) {
        return `${createRowHtml}<div class="empty-state" style="padding:20px 20px 8px;">
            <i class="fa-solid fa-folder-open"></i>
            <span>Grant access to back up your worlds/packs</span>
            <small>${bedrockAccessHint()}</small>
            <button class="modal-btn modal-btn-confirm" style="margin-top:12px;" data-grant-bedrock-storage>Grant Access</button>
        </div>`;
    }
    if (state.loading) {
        return `${createRowHtml}<div class="versions-loading"><span class="spinner"></span><span>Loading backups…</span></div>`;
    }
    if (state.backups.length === 0) {
        return `${createRowHtml}<div class="empty-state" style="padding:20px 20px 8px;">
            <i class="fa-solid fa-clock-rotate-left"></i>
            <span>No backups yet</span>
            <small>Snapshot your worlds and packs before a risky install.</small>
        </div>`;
    }
    return `${createRowHtml}${state.backups.map(b => bedrockBackupRowHtml(b, state.confirmDeleteName)).join("")}`;
}

// ── Bedrock Screenshots ──────────────────────────────────────────────────
// Read half of Pages/Home.razor's "bedrockscreenshots" panel — real
// .screenshot-grid/.screenshot-tile markup, fed by
// BedrockStorageService.listScreenshots() (in-game captures under
// com.mojang/Screenshots only; Xbox Game Bar's Captures folder is
// Windows-only and skipped rather than faked).
function bedrockScreenshotsPanelBody(state) {
    if (!state.hasAccess) {
        return `<div class="empty-state" style="padding:28px 20px;">
            <i class="fa-solid fa-folder-open"></i>
            <span>Grant access to your screenshots</span>
            <small>${bedrockAccessHint()}</small>
            <button class="modal-btn modal-btn-confirm" style="margin-top:12px;" data-grant-bedrock-storage>Grant Access</button>
        </div>`;
    }
    if (state.loading) {
        return `<div class="versions-loading"><span class="spinner"></span><span>Loading screenshots…</span></div>`;
    }
    if (state.screenshots.length === 0) {
        return `<div class="empty-state" style="padding:28px 20px;">
            <i class="fa-solid fa-image"></i>
            <span>No screenshots yet</span>
            <small>In-game screenshots show up here.</small>
        </div>`;
    }
    return `<div class="screenshot-grid">${state.screenshots.map(s => `
        <button class="screenshot-tile" data-tooltip="${escapeHtml(s.name)}">
            <img src="${s.uri}" loading="lazy" alt="${escapeHtml(s.name)}" />
        </button>`).join("")}</div>`;
}

// ── Launcher update modal ────────────────────────────────────────────────
// Same markup/classes as Pages/Home.razor's LAUNCHER UPDATE MODAL block.
function updateModalHtml(u, currentVersion) {
    if (!u.modalOpen || !u.info) return "";
    const changelogHtml = u.info.changelog
        ? `<div class="update-changelog">
               <span class="update-changelog-label">What's new</span>
               <div class="update-changelog-body">${escapeHtml(u.info.changelog).replace(/\n/g, "<br>")}</div>
           </div>`
        : "";
    const bodyBottom = u.installing
        ? `<div class="update-progress-wrap">
               <div class="update-progress-bar"><div class="update-progress-fill" style="width:${u.progress}%"></div></div>
               <span class="update-progress-pct">${u.progress}%</span>
           </div>
           <p class="modal-desc" style="margin-top:6px;">Downloading update — you'll be asked to confirm the install once it's done.</p>`
        : `<div class="modal-btn-row" style="margin-top:14px;">
               <button class="modal-btn modal-btn-cancel" data-skip-update>Skip</button>
               <button class="modal-btn modal-btn-cancel" data-close-update>Later</button>
               <button class="modal-btn modal-btn-confirm" data-install-update><i class="fa-solid fa-download"></i> Install</button>
           </div>`;
    return `
    <div class="modal-overlay" data-close-update-backdrop>
        <div class="modal-box update-modal">
            <div class="modal-header">
                <span class="modal-title"><i class="fa-solid fa-rocket" style="color:var(--accent);margin-right:8px;"></i>Update Available</span>
                <button class="modal-close-btn" data-close-update><i class="fa-solid fa-xmark"></i></button>
            </div>
            <div class="modal-body">
                <div class="update-version-row">
                    <div class="update-version-chip current"><span class="uvc-label">Current</span><span class="uvc-tag">v${currentVersion}</span></div>
                    <i class="fa-solid fa-arrow-right" style="color:var(--text-dim); font-size:12px;"></i>
                    <div class="update-version-chip next"><span class="uvc-label">Latest</span><span class="uvc-tag">v${u.info.tag}</span></div>
                </div>
                ${changelogHtml}
                ${bodyBottom}
            </div>
        </div>
    </div>`;
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
// Mirrors Components/StatsPanel.razor. Sessions come from a lightweight
// heuristic (MainActivity.kt's onResume() firing again once the user backs
// out of the launched Bedrock/Java Edition Activity — see
// App.recordLaunchStart()/onResumeFromGame() in app.js) rather than a real
// OS play-time API, same approximation any launcher without one has to
// make. An empty list still renders the honest zero/empty state.
function statsPanelBody(sessions) {
    const formatH = (seconds) => {
        if (seconds <= 0) return "0m";
        const h = Math.floor(seconds / 3600);
        const m = Math.floor((seconds % 3600) / 60);
        return h > 0 ? (m > 0 ? `${h}h ${m}m` : `${h}h`) : `${m}m`;
    };
    const total = sessions.reduce((sum, s) => sum + s.durationSeconds, 0);
    const longest = sessions.reduce((max, s) => Math.max(max, s.durationSeconds), 0);
    const lastPlayed = sessions.length > 0 ? formatRelativeTime(Math.floor(sessions[sessions.length - 1].start / 1000)) : "—";

    const cutoff = Date.now() - 14 * 24 * 60 * 60 * 1000;
    const recent = sessions.filter(s => s.start >= cutoff);
    const recentHtml = recent.length === 0
        ? `<div class="stats-empty">No play sessions recorded yet — launch the game to start tracking.</div>`
        : recent.slice().reverse().map(s => `
            <div class="version-row">
                <div class="version-meta">
                    <span class="version-name">${new Date(s.start).toLocaleDateString()}</span>
                    <span class="version-sub">${formatH(s.durationSeconds)}</span>
                </div>
            </div>`).join("");

    return `
    <div class="stats-cards">
        <div class="stats-card"><i class="fa-solid fa-stopwatch"></i><span class="stats-val">${formatH(total)}</span><span class="stats-label">Total playtime</span></div>
        <div class="stats-card"><i class="fa-solid fa-gamepad"></i><span class="stats-val">${sessions.length}</span><span class="stats-label">Sessions</span></div>
        <div class="stats-card"><i class="fa-solid fa-trophy"></i><span class="stats-val">${formatH(longest)}</span><span class="stats-label">Longest session</span></div>
        <div class="stats-card"><i class="fa-solid fa-calendar-day"></i><span class="stats-val">${lastPlayed}</span><span class="stats-label">Last played</span></div>
    </div>
    <span class="panel-section-label">Last 14 days</span>
    ${recentHtml}`;
}

// ── Logs & Crashes ─────────────────────────────────────────────────────
// Mirrors Components/LogsPanel.razor — real listing/reading via
// LogService.kt (plain java.io.File under the active instance's own game
// dir, same one JavaInstanceService.kt already resolves for everything
// else), redaction + the mclo.gs upload itself in js/logs.js.
function formatLogSize(bytes) {
    if (bytes <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB"];
    let v = bytes, i = 0;
    while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
    return `${Math.round(v * 10) / 10} ${units[i]}`;
}

function logsPanelBody(state) {
    if (state.files.length === 0) {
        return `<div class="stats-empty">No logs or crash reports found for the active instance yet.</div>`;
    }

    const list = `<div class="logs-file-list">
        ${state.files.map(f => `
        <button class="logs-file ${f.path === state.openPath ? "active" : ""} ${f.isCrash ? "crash" : ""}" data-open-log="${escapeHtml(f.path)}">
            <i class="${f.isCrash ? "fa-solid fa-triangle-exclamation" : "fa-solid fa-file-lines"}"></i>
            <span class="logs-file-name">${escapeHtml(f.name)}</span>
            <span class="logs-file-size">${formatLogSize(f.size)}</span>
        </button>`).join("")}
    </div>`;

    if (!state.openPath) return list;

    const fileName = state.openPath.split("/").pop();
    const shareRow = state.shareUrl
        ? `<div class="logs-share-url">
            <i class="fa-solid fa-link"></i>
            <a href="${escapeHtml(state.shareUrl)}" target="_blank" rel="noopener">${escapeHtml(state.shareUrl)}</a>
            <button class="btn-sm" data-copy-log-share><i class="fa-solid fa-copy"></i></button>
        </div>`
        : "";

    return `${list}
    <div class="logs-toolbar">
        <span class="logs-open-name">${escapeHtml(fileName)}</span>
        <div style="flex:1"></div>
        <button class="btn-sm" data-copy-log><i class="fa-solid fa-copy"></i> Copy</button>
        <button class="btn-sm btn-accent" data-share-log ${state.sharing ? "disabled" : ""}>
            ${state.sharing ? `<span class="spinner"></span>` : `<i class="fa-solid fa-share-nodes"></i>`}
            <span>&nbsp;Share to mclo.gs</span>
        </button>
    </div>
    ${shareRow}
    <pre class="logs-view">${escapeHtml(state.content)}</pre>`;
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

// ── Modpacks ───────────────────────────────────────────────────────────
// Mirrors Components/ModpacksPanel.razor: CurseForge/Modrinth source tabs,
// search, and a real result list (icon/author/downloads/summary) using the
// same CurseForge/Modrinth clients the Addons panel already uses. Install
// is still disabled — JavaInstanceService.kt now gives Android a real
// instance-management model (see the "javaprofile" panel's Instances card),
// so the actual remaining gap is porting ModpackInstallService.cs itself:
// downloading the pack's mod files/overrides zip and extracting it into a
// new instance's directory. Nothing to wire the button to truthfully yet.
function formatDownloads(count) {
    if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M downloads`;
    if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K downloads`;
    return `${count} downloads`;
}

function truncate(s, n) {
    if (!s) return "";
    return s.length <= n ? s : s.slice(0, n).trimEnd() + "…";
}

function modpackRowHtml(pack, installingId) {
    // Modrinth packs install for real (overrides + mods into a new Java
    // instance — see ModpackInstallService.kt); CurseForge still needs its
    // own project/file-ID resolution step ported, so stays disabled.
    const installing = installingId === pack.id;
    const installBtn = pack.source === "mr"
        ? `<button class="vcs-btn" ${installing ? "disabled" : ""} data-install-modpack="${pack.id}" data-install-modpack-name="${escapeHtml(pack.title)}" data-tooltip="Installs overrides + mods into a new instance — a Fabric/Forge profile still needs adding manually afterward">
               ${installing ? `<span class="spinner"></span>` : `<i class="fa-solid fa-download"></i>`}&nbsp;Install
           </button>`
        : `<button class="vcs-btn" disabled data-tooltip="CurseForge modpack install isn't ported yet — try the Modrinth tab"><i class="fa-solid fa-download"></i>&nbsp;Install</button>`;
    return `
    <div class="modpack-row">
        ${pack.icon ? `<img class="modpack-icon" src="${escapeHtml(pack.icon)}" loading="lazy" />` : `<div class="modpack-icon modpack-icon-fallback"><i class="fa-solid fa-cubes"></i></div>`}
        <div class="modpack-meta">
            <span class="version-name">${escapeHtml(pack.title)}</span>
            <span class="version-sub">${escapeHtml(pack.author)} · ${formatDownloads(pack.downloads)}</span>
            <span class="modpack-summary">${escapeHtml(truncate(pack.summary, 90))}</span>
        </div>
        ${installBtn}
    </div>`;
}

function modpacksPanelBody(state) {
    const cfUnavailable = state.source === "cf" && !CurseForge.isAvailable();
    let resultsHtml = "";
    if (state.searching) {
        resultsHtml = `<div class="versions-loading"><span class="spinner"></span><span>Searching…</span></div>`;
    } else if (state.error) {
        resultsHtml = `<div class="versions-loading versions-error"><i class="fa-solid fa-triangle-exclamation"></i><span>${escapeHtml(state.error)}</span></div>`;
    } else if (state.results.length === 0 && state.searched) {
        resultsHtml = `<div class="stats-empty">No modpacks found for "${escapeHtml(state.query)}".</div>`;
    }
    resultsHtml += state.results.map(p => modpackRowHtml(p, state.installingId)).join("");

    return `
    <div class="modpack-source-tabs">
        <button class="vcs-btn ${state.source === "cf" ? "active" : ""}" data-modpack-source="cf">CurseForge</button>
        <button class="vcs-btn ${state.source === "mr" ? "active" : ""}" data-modpack-source="mr">Modrinth</button>
    </div>
    <div class="modpack-search">
        <input class="panel-search-input" id="modpack-search-input" placeholder="Search modpacks…" value="${escapeHtml(state.query)}" />
        <button class="btn-sm" data-modpack-search><i class="fa-solid fa-magnifying-glass"></i></button>
    </div>
    ${cfUnavailable ? `<div class="stats-empty">A CurseForge API key is required to browse CurseForge. Add one in Settings, or use Modrinth (no key needed).</div>` : ""}
    ${resultsHtml}`;
}

// ── Theme Studio ───────────────────────────────────────────────────────
// Mirrors Components/ThemeStudioPanel.razor: theme list (create/select/
// duplicate/delete), color editing, shape/effect sliders, custom CSS —
// all genuinely live via ThemeEngine (a port of ThemeService.cs's own
// interop.js functions and ThemeDefinition.cs's BuildCssVars()), not a
// static mockup. Wallpaper picking needs real file access this WebView
// doesn't have a bridge for yet, so that row stays disabled.
const PRESET_SEEDS = [
    { preset: "dark", label: "Dark", bg: "#23272a", bgPanel: "#2c2f33", text: "#ffffff", textDim: "#99aab5" },
    { preset: "darker", label: "Darker", bg: "#1e2124", bgPanel: "#26292c", text: "#ffffff", textDim: "#99aab5" },
    { preset: "midnight", label: "Midnight", bg: "#141520", bgPanel: "#1c1e30", text: "#ffffff", textDim: "#99aab5" },
    { preset: "slate", label: "Slate", bg: "#2a2d35", bgPanel: "#33373f", text: "#ffffff", textDim: "#99aab5" },
    { preset: "ocean", label: "Ocean", bg: "#0e1f2a", bgPanel: "#13293a", text: "#ffffff", textDim: "#99aab5" },
    { preset: "forest", label: "Forest", bg: "#16241a", bgPanel: "#1d2f22", text: "#ffffff", textDim: "#99aab5" },
    { preset: "sunset", label: "Sunset", bg: "#2a1a26", bgPanel: "#33212f", text: "#ffffff", textDim: "#99aab5" },
    { preset: "light", label: "Light", bg: "#f5f6f8", bgPanel: "#ffffff", text: "#0e1116", textDim: "#5b6470" },
];

function newThemeFrom(preset, accent) {
    const seed = PRESET_SEEDS.find(p => p.preset === preset) || PRESET_SEEDS[0];
    return {
        id: `theme-${Date.now()}`,
        name: `${seed.label} Custom`,
        basePreset: seed.preset,
        accent: accent || "#7289da",
        bg: seed.bg, bgPanel: seed.bgPanel, text: seed.text, textDim: seed.textDim,
        red: "#ff5c5c", green: "#43b581", orange: "#faa61a",
        radiusSm: 8, radiusMd: 12, blur: 14, overlayStrength: 1.0,
        fontFamily: "", animationSpeed: 1.0,
        backgroundImage: "", backgroundFit: "cover", backgroundOpacity: 1.0,
        customCss: "",
    };
}

function themeColorCell(label, key, value) {
    return `
    <button type="button" class="ts-color-cell" data-theme-color-key="${key}">
        <span class="ts-color-dot" style="--dot-color:${escapeHtml(value)};"></span>
        <span>${label}</span>
        <input type="color" class="ts-color-native-input" data-theme-color-input="${key}" value="${toHexColor(value)}" style="position:absolute; opacity:0; width:0; height:0;" />
    </button>`;
}

function toHexColor(v) {
    // <input type="color"> only accepts #rrggbb — fall back to accent if we can't parse.
    if (/^#[0-9a-fA-F]{6}$/.test(v)) return v;
    return "#7289da";
}

function themeStudioPanelBody(themes, sel, activeThemeId) {
    if (themes.length === 0) {
        return `<div class="ts-empty">
            <i class="fa-solid fa-palette"></i>
            <span>No custom themes yet</span>
            <small>Start from the current preset and make it yours — every color, radius and font is editable.</small>
            <button class="btn-sm" data-theme-new><i class="fa-solid fa-plus"></i> Create your first theme</button>
        </div>`;
    }

    const chips = themes.map(t => `
        <button class="ts-chip ${sel && t.id === sel.id ? "selected" : ""} ${t.id === activeThemeId ? "active" : ""}" data-theme-select="${t.id}">
            <span class="ts-chip-swatch" style="background:${escapeHtml(t.accent)}"></span>
            ${escapeHtml(t.name)}
            ${t.id === activeThemeId ? `<i class="fa-solid fa-circle-check"></i>` : ""}
        </button>`).join("");

    if (!sel) return `<div class="ts-chips">${chips}</div>`;

    const isActive = sel.id === activeThemeId;
    return `
    <div class="ts-chips">${chips}</div>
    <div class="ts-actions">
        ${isActive
            ? `<button class="btn-sm ts-active-btn" data-theme-deactivate><i class="fa-solid fa-circle-check"></i> Active — click to stop using</button>`
            : `<button class="btn-sm btn-accent" data-theme-activate><i class="fa-solid fa-wand-magic-sparkles"></i> Use this theme</button>`}
        <button class="btn-sm" data-theme-duplicate><i class="fa-solid fa-copy"></i> Duplicate</button>
        <button class="btn-sm ts-danger" data-theme-delete><i class="fa-solid fa-trash"></i> Delete</button>
    </div>

    <div class="panel-section-label">Identity</div>
    <div class="setting-row">
        <div class="setting-meta"><span class="setting-label">Name</span><span class="setting-hint">Shown in the theme list</span></div>
        <input class="setting-input" id="theme-name-input" value="${escapeHtml(sel.name)}" maxlength="40" />
    </div>
    <div class="setting-row">
        <div class="setting-meta"><span class="setting-label">Base preset</span><span class="setting-hint">Inherits component styling — pick Light for light themes</span></div>
        <div style="display:flex; align-items:center; gap:6px;">
            <select class="setting-select" id="theme-base-preset-select">
                ${PRESET_SEEDS.map(p => `<option value="${p.preset}" ${sel.basePreset === p.preset ? "selected" : ""}>${p.label}</option>`).join("")}
            </select>
            <button class="btn-sm" title="Reset colors to this preset's defaults" data-theme-reseed><i class="fa-solid fa-rotate-left"></i></button>
        </div>
    </div>

    <div class="panel-section-label">Colors</div>
    <div class="ts-color-grid">
        ${themeColorCell("Accent", "accent", sel.accent)}
        ${themeColorCell("Background", "bg", sel.bg)}
        ${themeColorCell("Panels", "bgPanel", sel.bgPanel)}
        ${themeColorCell("Text", "text", sel.text)}
        ${themeColorCell("Muted text", "textDim", sel.textDim)}
        ${themeColorCell("Error", "red", sel.red)}
        ${themeColorCell("Success", "green", sel.green)}
        ${themeColorCell("Warning", "orange", sel.orange)}
    </div>

    <div class="panel-section-label">Shape & effects</div>
    <div class="setting-row">
        <div class="setting-meta"><span class="setting-label">Corner radius</span><span class="setting-hint">Small controls / large cards</span></div>
        <div style="display:flex; align-items:center; gap:8px;">
            <input type="range" min="0" max="24" id="theme-radius-sm" value="${sel.radiusSm}" /><span class="ts-slider-val">${sel.radiusSm}px</span>
            <input type="range" min="0" max="32" id="theme-radius-md" value="${sel.radiusMd}" /><span class="ts-slider-val">${sel.radiusMd}px</span>
        </div>
    </div>
    <div class="setting-row">
        <div class="setting-meta"><span class="setting-label">Panel blur</span><span class="setting-hint">Backdrop blur behind panels — 0 disables (fastest)</span></div>
        <div style="display:flex; align-items:center; gap:8px;">
            <input type="range" min="0" max="30" step="2" id="theme-blur" value="${sel.blur}" /><span class="ts-slider-val">${sel.blur}px</span>
        </div>
    </div>
    <div class="setting-row">
        <div class="setting-meta"><span class="setting-label">Overlay strength</span><span class="setting-hint">How much the wallpaper is dimmed toward the background color</span></div>
        <div style="display:flex; align-items:center; gap:8px;">
            <input type="range" min="0" max="100" step="5" id="theme-overlay" value="${Math.round(sel.overlayStrength * 100)}" /><span class="ts-slider-val">${Math.round(sel.overlayStrength * 100)}%</span>
        </div>
    </div>

    <div class="panel-section-label">Background</div>
    <div class="setting-row">
        <div class="setting-meta"><span class="setting-label">Wallpaper</span><span class="setting-hint">Any image on the device — replaces the launcher background</span></div>
        <div style="display:flex; gap:6px;">
            <button class="btn-sm" id="theme-pick-wallpaper"><i class="fa-solid fa-image"></i> Choose</button>
            <button class="btn-sm" id="theme-reset-wallpaper">Reset</button>
        </div>
    </div>

    <div class="panel-section-label">Typography & motion</div>
    <div class="setting-row">
        <div class="setting-meta"><span class="setting-label">Font family</span><span class="setting-hint">Any font the system provides — empty uses the default</span></div>
        <input class="setting-input" id="theme-font-input" placeholder="Segoe UI" value="${escapeHtml(sel.fontFamily)}" />
    </div>
    <div class="setting-row">
        <div class="setting-meta"><span class="setting-label">Animation speed</span><span class="setting-hint">Bundled with the theme — 1x is default</span></div>
        <div style="display:flex; align-items:center; gap:8px;">
            <input type="range" min="0.25" max="2" step="0.25" id="theme-anim-speed" value="${sel.animationSpeed}" /><span class="ts-slider-val">${sel.animationSpeed}x</span>
        </div>
    </div>

    <div class="panel-section-label">Custom CSS</div>
    <div class="ts-css-editor">
        <textarea class="setting-input ts-css-textarea" id="theme-custom-css" spellcheck="false" rows="6" placeholder=".action-btn { border: 1px solid var(--accent); }">${escapeHtml(sel.customCss)}</textarea>
        <div class="ts-css-actions">
            <button class="btn-sm" data-theme-apply-css><i class="fa-solid fa-play"></i> Apply</button>
            <button class="btn-sm" data-theme-reset-css><i class="fa-solid fa-rotate-left"></i> Reset</button>
            <span class="setting-hint">Injected last, so it wins over everything.</span>
        </div>
    </div>`;
}

// Skin Library — real markup from Components/SkinLibraryPanel.razor.
function skinlibCardHtml(s) {
    return `<div class="skinlib-card">
        <div class="skinlib-preview"><img src="${escapeHtml(s.url)}" loading="lazy" style="image-rendering:pixelated;" /></div>
        <span class="skinlib-name">${escapeHtml(s.name)}</span>
        <div class="skinlib-actions">
            <button class="btn-sm btn-accent" data-skinlib-apply="${s.id}" data-tooltip="Apply to your account as ${s.slim ? "slim (Alex)" : "classic (Steve)"}">
                <i class="fa-solid fa-shirt"></i> Apply</button>
            <button class="btn-sm" data-skinlib-apply-alt="${s.id}" data-tooltip="Apply as ${s.slim ? "classic (Steve)" : "slim (Alex)"} instead">${s.slim ? "Classic" : "Slim"}</button>
            <button class="btn-sm ts-danger" data-skinlib-delete="${s.id}" data-tooltip="Remove from library"><i class="fa-solid fa-trash"></i></button>
        </div>
    </div>`;
}

function skinLibraryPanelBody(state) {
    const grid = state.skins.length === 0
        ? `<div class="stats-empty">No saved skins yet. Add a PNG or pull one in by username, then apply it to your signed-in account.</div>`
        : `<div class="skinlib-grid">${state.skins.map(skinlibCardHtml).join("")}</div>`;
    const errorHtml = state.error ? `<div class="stats-empty" style="color:var(--red);">${escapeHtml(state.error)}</div>` : "";

    return `<div class="skinlib-add">
        <input class="panel-search-input" id="skinlib-username-input" placeholder="Add by username (grabs their current skin)…" value="${escapeHtml(state.username)}" />
        <button class="btn-sm" id="skinlib-add-btn" ${state.busy ? "disabled" : ""}><i class="fa-solid fa-user-plus"></i></button>
    </div>
    ${errorHtml}
    ${grid}`;
}

function skinLibraryPanelHtml(state) {
    const signedIn = !!App.state.settings.javaUsername;
    return `<div class="panel-overlay">
        <div class="panel-handle"></div>
        <div class="panel-header">
            <div class="panel-title-wrap"><span class="panel-title">Skin Library</span><div class="panel-title-underline"></div></div>
            <div class="panel-header-actions">
                <button class="btn-sm" id="skinlib-save-current" ${signedIn ? "" : "disabled"} ${signedIn ? "" : `data-tooltip="Sign in with Microsoft (Java) first to save your current skin"`}>
                    <i class="fa-solid fa-floppy-disk"></i> Save current</button>
                <button class="btn-sm" id="skinlib-add-png" data-tooltip="Import a skin PNG from this device">
                    <i class="fa-solid fa-upload"></i> Add PNG</button>
                <button class="panel-back-btn" data-close-panel data-tooltip="Back"><i class="fa-solid fa-chevron-down"></i></button>
            </div>
        </div>
        <div class="panel-body" id="skinlib-body">${skinLibraryPanelBody(state)}</div>
    </div>`;
}

// ── Global search ──────────────────────────────────────────────────────
// Real markup from Pages/Home.razor's search-overlay/search-modal (the
// Ctrl+K command palette). The full desktop list mixes in live client/
// version/server data via BuildDynamicSearchPool() in Home.Search.cs;
// this is the curated subset that maps to panels/actions that actually
// exist on Android, dropping Windows-only entries (fullscreen/F11, tray,
// wallpaper picker, folder shortcuts) and DLL-client selection (no longer
// offered — see clientsPanelBody()).
function searchQuickActions(edition) {
    const common = [
        { icon: "fa-solid fa-play", label: "Launch Game", cat: "Quick Actions", sub: "Start Minecraft", action: "launch" },
        { icon: "fa-solid fa-gear", label: "Open Settings", cat: "Quick Actions", sub: "Appearance, account, system", panel: "settings" },
        { icon: "fa-solid fa-cube", label: "Browse Addons", cat: "Quick Actions", sub: "CurseForge / Modrinth content", panel: "addons" },
        { icon: "fa-solid fa-chart-simple", label: "Statistics", cat: "Navigation", sub: "Playtime and sessions", panel: "stats" },
        { icon: "fa-solid fa-shirt", label: "Skin Library", cat: "Account", sub: "Saved skins — apply to your account", panel: "skinlibrary" },
        { icon: "fa-solid fa-gamepad", label: "Xbox Profile", cat: "Account", sub: "Sign in with Microsoft", action: "xbox" },
        { icon: "fa-brands fa-discord", label: "Discord Account", cat: "Account", sub: "Connect or manage Discord", action: "discord" },
        { icon: "fa-solid fa-download", label: "Downloads", cat: "Navigation", sub: "Active and finished downloads", panel: "downloads" },
        { icon: "fa-solid fa-palette", label: "Theme Studio", cat: "Settings", sub: "Create a fully custom theme", panel: "themestudio" },
        { icon: "fa-solid fa-heart", label: "Credits", cat: "About", sub: "View launcher credits", panel: "credits" },
        { icon: "fa-solid fa-newspaper", label: "News", cat: "About", sub: "Glacier news and release notes", panel: "news" },
    ];
    if (edition === "bedrock") {
        return [
            { icon: "fa-solid fa-puzzle-piece", label: "Manage Clients", cat: "Quick Actions", sub: "Vanilla — see Settings for why that's the only option", panel: "clients" },
            { icon: "fa-solid fa-server", label: "Servers", cat: "Quick Actions", sub: "Quick-launch into saved servers", panel: "servers" },
            { icon: "fa-solid fa-box-archive", label: "MC Versions", cat: "Quick Actions", sub: "Switch Minecraft Bedrock versions", panel: "mcversions" },
            ...common,
        ];
    }
    return [
        { icon: "fa-brands fa-java", label: "Java Launchers", cat: "Java", sub: "Vanilla and Glacier Client", panel: "javaclients" },
        { icon: "fa-solid fa-box-archive", label: "Java Versions", cat: "Java", sub: "Install or launch Java Edition", panel: "javaversions" },
        { icon: "fa-solid fa-cubes", label: "Browse Modpacks", cat: "Java", sub: "Install CurseForge / Modrinth modpacks", panel: "modpacks" },
        { icon: "fa-solid fa-file-lines", label: "Logs & Crashes", cat: "Java", sub: "View logs and crash reports", panel: "logs" },
        { icon: "fa-solid fa-user", label: "Profile", cat: "Java", sub: "Signed-in Java account", panel: "javaprofile" },
        { icon: "fa-solid fa-images", label: "Screenshots", cat: "Java", sub: "Screenshots taken in-game", panel: "javascreenshots" },
        ...common,
    ];
}

function searchResultRowHtml(r, idx, selected) {
    return `<div class="search-result-item ${selected ? "sel" : ""}" data-search-idx="${idx}">
        <div class="search-result-icon"><i class="${r.icon}"></i></div>
        <div class="search-result-text">
            <span class="search-result-label">${escapeHtml(r.label)}</span>
            ${r.sub ? `<span class="search-result-sub">${escapeHtml(r.sub)}</span>` : ""}
        </div>
        <span class="search-result-enter">↵</span>
    </div>`;
}

function searchOverlayHtml(state) {
    const query = (state.query || "").trim().toLowerCase();
    const all = searchQuickActions(state.edition);
    const filtered = query
        ? all.filter(r => r.label.toLowerCase().includes(query) || (r.sub || "").toLowerCase().includes(query))
        : all;

    let resultsHtml;
    if (filtered.length === 0) {
        resultsHtml = `<div class="search-no-results">No results for "<strong>${escapeHtml(state.query)}</strong>"</div>`;
    } else {
        const groups = [];
        for (const r of filtered) {
            let g = groups.find(g => g.cat === r.cat);
            if (!g) { g = { cat: r.cat, items: [] }; groups.push(g); }
            g.items.push(r);
        }
        let flatIdx = 0;
        resultsHtml = groups.map(g => `
            <div class="search-category">${escapeHtml(g.cat)}</div>
            ${g.items.map(r => searchResultRowHtml(r, flatIdx, flatIdx++ === state.selIdx)).join("")}
        `).join("");
    }

    return `<div class="search-overlay" id="search-overlay-backdrop">
        <div class="search-modal" id="search-modal">
            <div class="search-modal-input-row">
                <i class="fa-solid fa-magnifying-glass"></i>
                <input class="search-modal-input" id="search-modal-input" placeholder="Search settings, panels, account..." value="${escapeHtml(state.query)}" />
                <span class="search-modal-esc">ESC</span>
            </div>
            <div class="search-results" id="search-results">${resultsHtml}</div>
        </div>
    </div>`;
}

// ── Notification bell ──────────────────────────────────────────────────
// Real markup from Home.razor's .notif-bell-wrap/.notif-panel. Desktop
// mixes in a NotificationService event log (crash detection, update
// checks, etc.) that doesn't exist here yet, so the notifications list
// stays honestly empty — the bell's badge count and "Downloads" section
// are real, driven by this app's own App.state.downloads.
function notifPanelHtml(downloads) {
    const active = downloads.filter(d => d.status === "downloading");
    const downloadsSection = downloads.length > 0 ? `
        <div class="notif-panel-header">
            <span>Downloads</span>
            <button class="notif-clear-btn" data-open-panel="downloads">View all</button>
        </div>
        <div class="notif-panel-list" style="max-height:160px;">
            ${downloads.slice(0, 5).map(d => `
                <div class="notif-item info">
                    <i class="fa-solid ${d.status === "downloading" ? "fa-arrow-down" : d.status === "complete" ? "fa-circle-check" : "fa-triangle-exclamation"}"></i>
                    <div class="notif-item-body">
                        <div class="notif-item-title" style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">${escapeHtml(d.label)}</div>
                        <div class="notif-item-msg">${d.status}${d.status === "downloading" ? ` · ${Math.round((d.progress || 0) * 100)}%` : ""}</div>
                    </div>
                </div>`).join("")}
        </div>` : "";

    return `<div class="notif-panel" id="notif-panel">
        ${downloadsSection}
        <div class="notif-panel-header"><span>Notifications</span></div>
        <div class="notif-panel-list">
            <div class="notif-empty"><i class="fa-regular fa-bell-slash"></i><span>No notifications</span></div>
        </div>
    </div>`;
}
