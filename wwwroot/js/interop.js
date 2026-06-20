// ── Core window controls ──────────────────────────────────────
window.startDrag      = () => postToHost('startDrag');
window.closeWindow    = () => postToHost('close');
window.minimizeWindow = () => postToHost('minimize');
window.minimizeToTray = () => postToHost('minimizeToTray');
window.maximizeWindow   = () => postToHost('maximize');
window.toggleFullscreen = () => postToHost('fullscreen');

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
    // Push the most-recent fullscreen state we know about (host may have sent it
    // before .NET was ready to receive callbacks)
    if (window._glacierFullscreen != null) {
        try { dotNetRef.invokeMethodAsync('OnFullscreenChanged', !!window._glacierFullscreen); } catch { }
    }
};

// Host → page bridge: WPF posts "fullscreen:on" / "fullscreen:off" when the window
// enters or exits fullscreen mode. We buffer the latest value and forward it to
// .NET so the maximize/fullscreen icons can update.
if (window.chrome?.webview) {
    window.chrome.webview.addEventListener('message', e => {
        const msg = e.data;
        if (typeof msg !== 'string') return;
        if (msg === 'fullscreen:on' || msg === 'fullscreen:off') {
            window._glacierFullscreen = (msg === 'fullscreen:on');
            if (window._glacierDotNet) {
                try { window._glacierDotNet.invokeMethodAsync('OnFullscreenChanged', window._glacierFullscreen); } catch { }
            }
        }
    });
}

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

// ── File picker (Image / Wallpaper) ───────────────────────
window.pickImageFile = async (dotNetRef) => {
    return new Promise(resolve => {
        const input = document.createElement('input');
        input.type   = 'file';
        input.accept = 'image/*';
        input.style.display = 'none';
        document.body.appendChild(input);
        input.onchange = () => {
            const file = input.files[0];
            if (file) dotNetRef.invokeMethodAsync('OnWallpaperPicked', file.path || file.name);
            document.body.removeChild(input);
            resolve();
        };
        input.oncancel = () => { document.body.removeChild(input); resolve(); };
        input.click();
    });
};

// ── File picker (Skin PNG) ────────────────────────────────────
window.pickSkinFile = async (dotNetRef) => {
    return new Promise(resolve => {
        const input = document.createElement('input');
        input.type   = 'file';
        input.accept = 'image/png,.png';
        input.style.display = 'none';
        document.body.appendChild(input);
        input.onchange = () => {
            const file = input.files[0];
            if (file) dotNetRef.invokeMethodAsync('OnSkinPicked', file.path || file.name);
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
        .filter(f => ['.dll', '.zip', '.jar'].some(ext => f.name.toLowerCase().endsWith(ext)));
    if (!files.length) return;
    if (files[0].path && window._glacierDotNet)
        window._glacierDotNet.invokeMethodAsync('OnFileDropped', files[0].path);
});

// ── Keyboard shortcuts ────────────────────────────────────────
document.addEventListener('keydown', e => {
    if (!window._glacierDotNet) return;
    const dn = window._glacierDotNet;
    if (e.ctrlKey && !e.shiftKey && e.key === 'k') { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'search');   return; }
    if (e.ctrlKey && !e.shiftKey && e.key === 'l') { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'launch');   return; }
    if (e.ctrlKey && !e.shiftKey && e.key === '1') { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'settings'); return; }
    if (e.ctrlKey && !e.shiftKey && e.key === '2') { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'clients');  return; }
    if (e.ctrlKey && !e.shiftKey && e.key === '3') { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'versions'); return; }
    if (e.ctrlKey && !e.shiftKey && e.key === '4') { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'addons');   return; }
    if (e.ctrlKey && !e.shiftKey && e.key === '5') { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'servers');  return; }
    if (e.ctrlKey && (e.key === ',' || e.key === '?')) { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'settings'); return; }
    if (e.ctrlKey && (e.key === 'Tab' || e.code === 'Tab')) { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'cycle'); return; }
    if (e.ctrlKey && e.shiftKey && (e.key === 'r' || e.key === 'R')) { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'refresh'); return; }
    if (e.ctrlKey && e.shiftKey && (e.key === 't' || e.key === 'T')) { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'cycletheme'); return; }
    if (e.ctrlKey && e.shiftKey && (e.key === 'a' || e.key === 'A')) { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'cycleaccent'); return; }
    if (e.ctrlKey && e.shiftKey && (e.key === 'd' || e.key === 'D')) { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'diagnostics'); return; }
    if (e.key === 'F11')       { e.preventDefault(); dn.invokeMethodAsync('KbShortcut', 'fullscreen'); return; }
    if (e.key === 'Escape')    { dn.invokeMethodAsync('KbShortcut', 'escape'); return; }
    if (e.key === 'ArrowDown') { dn.invokeMethodAsync('KbShortcut', 'down'); return; }
    if (e.key === 'ArrowUp')   { dn.invokeMethodAsync('KbShortcut', 'up'); return; }
    if (e.key === 'Enter')     { dn.invokeMethodAsync('KbShortcut', 'enter'); }
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
    // 0 px backdrop-filter still triggers a GPU layer + readback on every paint.
    // Toggling the no-blur class strips the property entirely so integrated
    // graphics don't pay for an effect the user can't see anyway.
    document.documentElement.classList.toggle('no-blur', Number(px) <= 0);
};

window.setCustomBackground = (filePath) => {
    const bg = document.querySelector('.window-bg');
    if (!bg) return;
    if (filePath) {
        // Custom backgrounds are saved into ~/Glacier Launcher, which the WPF host
        // maps to https://glacier-files.local/ — file:// URLs are blocked by WebView2,
        // so we serve through the virtual host instead. Cache-bust by mtime stand-in
        // (Date.now()) so picking a new image with the same filename refreshes.
        const fileName = filePath.replace(/\\/g, '/').split('/').pop();
        bg.style.backgroundImage = `url('https://glacier-files.local/${encodeURIComponent(fileName)}?t=${Date.now()}')`;
    } else {
        // Document-relative form: matches the inline style set in Razor and
        // resolves the same way under both `dotnet run` and the published exe.
        bg.style.backgroundImage = "url('../images/bg.jpg')";
    }
};

window.setCompactMode = (enabled) => {
    document.documentElement.classList.toggle('compact', !!enabled);
};

window.setAnimationsEnabled = (enabled) => {
    document.documentElement.classList.toggle('no-animations', !enabled);
};

window.applyStoredSettings = (accent, theme, blur, customBg, compact, animations) => {
    if (accent) window.setAccentColor(accent);
    if (theme)  window.setTheme(theme);
    if (blur != null) window.setBlurIntensity(blur);
    if (customBg) window.setCustomBackground(customBg);
    if (compact != null) window.setCompactMode(compact);
    if (animations != null) window.setAnimationsEnabled(animations);
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

// ── 3D skin + cape viewer (skinview3d: vendored first, CDN fallback, then static) ──
window.glacierSkin = (function () {
    let libPromise = null;
    const viewers = {};

    function loadScript(src) {
        return new Promise(resolve => {
            const s = document.createElement('script');
            s.src = src;
            s.onload  = () => resolve(true);
            s.onerror = () => resolve(false);
            document.head.appendChild(s);
        });
    }

    function ensureLib() {
        if (window.skinview3d) return Promise.resolve(true);
        if (libPromise) return libPromise;
        libPromise = (async () => {
            // Vendored copy works offline; the CDN is only a backstop.
            if (await loadScript('js/lib/skinview3d.bundle.js') && window.skinview3d) return true;
            if (await loadScript('https://cdn.jsdelivr.net/npm/skinview3d@3/bundles/skinview3d.bundle.js') && window.skinview3d) return true;
            libPromise = null;
            return false;
        })();
        return libPromise;
    }

    return {
        // Returns true if the 3D viewer initialised; false → caller shows the static render.
        // model: 'slim' (Alex) | anything else (Steve/classic).
        render: async function (canvasId, skinUrl, capeUrl, model) {
            try {
                if (!await ensureLib()) return false;
                const canvas = document.getElementById(canvasId);
                if (!canvas) return false;
                if (viewers[canvasId]) { try { viewers[canvasId].dispose(); } catch (e) {} delete viewers[canvasId]; }

                const w = canvas.clientWidth  || 240;
                const h = canvas.clientHeight || 340;
                const viewer = new skinview3d.SkinViewer({ canvas, width: w, height: h });
                try { await viewer.loadSkin(skinUrl, { model: model === 'slim' ? 'slim' : 'default' }); }
                catch (e) { return false; }
                viewer.autoRotate      = true;
                viewer.autoRotateSpeed = 0.55;
                viewer.zoom            = 0.85;
                try { viewer.animation = new skinview3d.WalkingAnimation(); viewer.animation.speed = 0.6; } catch (e) {}
                if (capeUrl) { try { await viewer.loadCape(capeUrl); } catch (e) { /* no cape texture */ } }
                viewers[canvasId] = viewer;
                return true;
            } catch (e) {
                return false;
            }
        },
        // Swap the arm model (Steve/Alex) without rebuilding the viewer.
        setModel: async function (canvasId, skinUrl, model) {
            const v = viewers[canvasId];
            if (!v) return false;
            try { await v.loadSkin(skinUrl, { model: model === 'slim' ? 'slim' : 'default' }); return true; }
            catch (e) { return false; }
        },
        // mode: 'cape' | 'elytra' | 'off'
        setCape: async function (canvasId, capeUrl, mode) {
            const v = viewers[canvasId];
            if (!v) return false;
            try {
                if (mode === 'off' || !capeUrl) { v.resetCape(); return true; }
                await v.loadCape(capeUrl, { backEquipment: mode === 'elytra' ? 'elytra' : 'cape' });
                return true;
            } catch (e) { return false; }
        },
        // Cheap presence check: an <img> load succeeds only if the cape texture exists.
        probe: function (url) {
            return new Promise(resolve => {
                const img = new Image();
                img.onload  = () => resolve(true);
                img.onerror = () => resolve(false);
                img.src = url;
            });
        },
        dispose: function (canvasId) {
            if (viewers[canvasId]) { try { viewers[canvasId].dispose(); } catch (e) {} delete viewers[canvasId]; }
        }
    };
})();

// ── Panel drag handle (resize / dismiss via the top bar) ─────
// Uses event delegation so it works for any .panel-handle that ever
// appears in the DOM, including ones added later by Blazor.
(function () {
    const SNAP_POINTS_PCT = [40, 78, 95];   // sane heights to snap to
    const DISMISS_PCT     = 18;             // drag below this → close panel
    const MIN_PCT         = 12;             // never let the panel get smaller than this mid-drag

    let drag = null;

    document.addEventListener('pointerdown', e => {
        if (e.button !== 0) return;
        const handle = e.target.closest('.panel-handle');
        if (!handle) return;
        const overlay = handle.closest('.panel-overlay');
        if (!overlay) return;

        e.preventDefault();
        drag = {
            overlay,
            handle,
            pointerId: e.pointerId,
            startY: e.clientY,
            startH: overlay.getBoundingClientRect().height,
            viewportH: window.innerHeight,
        };
        try { handle.setPointerCapture(e.pointerId); } catch { }
        overlay.classList.add('dragging');
    });

    document.addEventListener('pointermove', e => {
        if (!drag || e.pointerId !== drag.pointerId) return;
        const dy = e.clientY - drag.startY;          // positive = moved down
        const newH = drag.startH - dy;               // shrink as user drags down
        const pct = Math.max(0, (newH / drag.viewportH) * 100);
        // clamp visually so we don't draw a sub-zero panel — actual dismissal happens on release
        drag.overlay.style.height = Math.max(MIN_PCT, Math.min(95, pct)) + '%';
        drag.lastPct = pct;
    });

    function endDrag(e) {
        if (!drag) return;
        if (e && e.pointerId !== drag.pointerId) return;
        const overlay = drag.overlay;
        const pct = drag.lastPct ?? (overlay.getBoundingClientRect().height / drag.viewportH) * 100;
        overlay.classList.remove('dragging');
        try { drag.handle.releasePointerCapture(drag.pointerId); } catch { }
        drag = null;

        if (pct < DISMISS_PCT) {
            // Animate out and close. Easiest path: route through KbShortcut('escape')
            // which already handles closing the active panel.
            overlay.style.height = '';
            if (window._glacierDotNet) {
                try { window._glacierDotNet.invokeMethodAsync('KbShortcut', 'escape'); } catch { }
            }
            return;
        }
        // Snap to the nearest sensible height
        const snap = SNAP_POINTS_PCT.reduce(
            (best, p) => Math.abs(p - pct) < Math.abs(best - pct) ? p : best,
            SNAP_POINTS_PCT[0]
        );
        overlay.style.height = snap + '%';
    }

    document.addEventListener('pointerup', endDrag);
    document.addEventListener('pointercancel', endDrag);
})();
