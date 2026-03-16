// ── Core window controls ──────────────────────────────────────
window.startDrag      = () => postToHost('startDrag');
window.closeWindow    = () => postToHost('close');
window.minimizeWindow = () => postToHost('minimize');
window.minimizeToTray = () => postToHost('minimizeToTray');
window.maximizeWindow = () => postToHost('maximize');

function postToHost(msg) {
    if (window.chrome?.webview) window.chrome.webview.postMessage(msg);
}

// ── Dynamic UI scaling ────────────────────────────────────────
// Scale the entire UI proportionally when the window is resized.
// Base design size is 740 × 500. We zoom up (or down) linearly.
const BASE_W = 740;
const BASE_H = 500;
const MIN_SCALE = 0.75;
const MAX_SCALE = 3.0;

function applyScale() {
    const scaleX = window.innerWidth  / BASE_W;
    const scaleY = window.innerHeight / BASE_H;
    const scale  = Math.max(MIN_SCALE, Math.min(MAX_SCALE, Math.min(scaleX, scaleY)));
    document.documentElement.style.setProperty('--scale', scale);
    // Use CSS zoom — supported in WebView2 (Chromium) and scales everything
    // including fonts, borders, shadows without any pixel math in CSS.
    document.body.style.zoom = scale;
}

window.addEventListener('resize', applyScale);
// Run once immediately when the page loads
document.addEventListener('DOMContentLoaded', applyScale);
// Also run after Blazor bootstraps (in case DOMContentLoaded already fired)
applyScale();

// ── .NET interop handle ───────────────────────────────────────
window.glacierRegisterDropHandler = (dotNetRef) => {
    window._glacierDotNet = dotNetRef;
    // Re-apply scale now that .NET is ready
    applyScale();
};

// ── File picker (DLL) ─────────────────────────────────────────
window.pickDllFile = async (dotNetRef) => {
    return new Promise(resolve => {
        const input = document.createElement('input');
        input.type   = 'file';
        input.accept = '.dll';
        input.style.display = 'none';
        document.body.appendChild(input);
        input.onchange = () => {
            const file = input.files[0];
            if (file) dotNetRef.invokeMethodAsync('OnDllDropped', file.path || file.name);
            document.body.removeChild(input);
            resolve();
        };
        input.oncancel = () => { document.body.removeChild(input); resolve(); };
        input.click();
    });
};

// ── Drag-and-drop DLL ─────────────────────────────────────────
document.addEventListener('dragover', e => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
    document.body.classList.add('drop-active');
});
document.addEventListener('dragleave', e => {
    if (!e.relatedTarget || e.relatedTarget === document.documentElement)
        document.body.classList.remove('drop-active');
});
document.addEventListener('drop', e => {
    e.preventDefault();
    document.body.classList.remove('drop-active');
    const files = Array.from(e.dataTransfer.files || [])
        .filter(f => f.name.toLowerCase().endsWith('.dll'));
    if (!files.length) return;
    if (files[0].path && window._glacierDotNet)
        window._glacierDotNet.invokeMethodAsync('OnDllDropped', files[0].path);
});

// ── Keyboard shortcuts ────────────────────────────────────────
document.addEventListener('keydown', e => {
    if (!window._glacierDotNet) return;
    if (e.ctrlKey && e.key === 'k') { e.preventDefault(); window._glacierDotNet.invokeMethodAsync('KbShortcut', 'search'); return; }
    if (e.ctrlKey && e.key === 'l') { e.preventDefault(); window._glacierDotNet.invokeMethodAsync('KbShortcut', 'launch'); return; }
    if (e.ctrlKey && e.key === '1') { e.preventDefault(); window._glacierDotNet.invokeMethodAsync('KbShortcut', 'settings'); return; }
    if (e.ctrlKey && e.key === '2') { e.preventDefault(); window._glacierDotNet.invokeMethodAsync('KbShortcut', 'clients'); return; }
    if (e.ctrlKey && e.key === '3') { e.preventDefault(); window._glacierDotNet.invokeMethodAsync('KbShortcut', 'versions'); return; }
    if (e.key === 'Escape')    { window._glacierDotNet.invokeMethodAsync('KbShortcut', 'escape'); return; }
    if (e.key === 'ArrowDown') { window._glacierDotNet.invokeMethodAsync('KbShortcut', 'down'); return; }
    if (e.key === 'ArrowUp')   { window._glacierDotNet.invokeMethodAsync('KbShortcut', 'up'); return; }
    if (e.key === 'Enter')     { window._glacierDotNet.invokeMethodAsync('KbShortcut', 'enter'); }
});

// ── Clipboard ─────────────────────────────────────────────────
window.copyToClipboard = async (text) => {
    try { await navigator.clipboard.writeText(text); return true; }
    catch {
        const ta = document.createElement('textarea');
        ta.value = text; ta.style.cssText = 'position:fixed;opacity:0;';
        document.body.appendChild(ta); ta.select();
        const ok = document.execCommand('copy');
        document.body.removeChild(ta); return ok;
    }
};

// ── CSS variable setters ──────────────────────────────────────
window.setAccentColor = (color) => {
    const hex = color.replace('#', '');
    const r = parseInt(hex.slice(0,2),16), g = parseInt(hex.slice(2,4),16), b = parseInt(hex.slice(4,6),16);
    document.documentElement.style.setProperty('--accent', color);
    document.documentElement.style.setProperty('--accent-hover', color);
    document.documentElement.style.setProperty('--accent-glow', `rgba(${r},${g},${b},0.42)`);
    document.documentElement.style.setProperty('--accent-bg',   `rgba(${r},${g},${b},0.10)`);
};

window.setTheme = (preset) => {
    document.documentElement.setAttribute('data-theme', preset);
};

window.setBlurIntensity = (px) => {
    document.documentElement.style.setProperty('--blur', px + 'px');
};

window.applyStoredSettings = (accent, theme, blur) => {
    if (accent) window.setAccentColor(accent);
    if (theme)  window.setTheme(theme);
    if (blur != null) window.setBlurIntensity(blur);
    applyScale();
};

window.openDiscordOAuth = (clientId) => {
    const redirect = encodeURIComponent('https://glacierclient.xyz/auth');
    const url = `https://discord.com/api/oauth2/authorize?client_id=${clientId}&redirect_uri=${redirect}&response_type=token&scope=identify`;
    postToHost('openUrl:' + url);
};

window.focusSearchInput = () => {
    setTimeout(() => { const el = document.querySelector('.search-modal-input'); if (el) el.focus(); }, 50);
};
