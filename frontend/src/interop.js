// Ported from the WPF app's wwwroot/js/interop.js. The WebView2-specific
// bridge (chrome.webview.postMessage / Blazor dotNetRef) is replaced with
// Tauri's window API and a `glacier:*` CustomEvent bus that Svelte listens
// to — everything else (scale, theming, panel drag-resize, tab cycler) is
// pure DOM manipulation and ports unchanged.
import { getCurrentWindow } from '@tauri-apps/api/window';

const tauriWindow = getCurrentWindow();

window.startDrag = () => tauriWindow.startDragging();
window.closeWindow = () => tauriWindow.close();
window.minimizeWindow = () => tauriWindow.minimize();
window.minimizeToTray = () => tauriWindow.hide();
window.maximizeWindow = () => tauriWindow.toggleMaximize();
window.toggleFullscreen = async () => {
    const next = !(await tauriWindow.isFullscreen());
    await tauriWindow.setFullscreen(next);
    window._glacierFullscreen = next;
    window.dispatchEvent(new CustomEvent('glacier:fullscreen-changed', { detail: next }));
};

const BASE_W = 880;
const BASE_H = 600;
const MIN_SCALE = 0.75;
const MAX_SCALE = 3.0;

function applyScale() {
    const scaleX = window.innerWidth  / BASE_W;
    const scaleY = window.innerHeight / BASE_H;
    let scale    = Math.max(MIN_SCALE, Math.min(MAX_SCALE, Math.min(scaleX, scaleY)));

    scale = Math.max(0.5, Math.min(MAX_SCALE, scale * (window._glacierUserScale || 1)));
    document.documentElement.style.setProperty('--scale', scale);

    document.body.style.zoom = scale;
}

window.setUiScale = (pct) => {
    const p = Number(pct);
    window._glacierUserScale = (p && p > 0) ? p / 100 : 1;
    applyScale();
};

window.addEventListener('resize', applyScale);

document.addEventListener('DOMContentLoaded', applyScale);

applyScale();

// File drops: Tauri delivers real filesystem paths via its own webview
// drag-drop event (unlike WebView2's File API), wired in App.svelte. These
// two hooks stay so the CSS's `body.drop-active` selector keeps working.
window.setDropActive = (active) => {
    document.body.classList.toggle('drop-active', !!active);
};

window.onHostFileDropped = (path) => {
    document.body.classList.remove('drop-active');
    if (path) window.dispatchEvent(new CustomEvent('glacier:file-dropped', { detail: path }));
};

function kb(name) {
    window.dispatchEvent(new CustomEvent('glacier:kbshortcut', { detail: name }));
}

document.addEventListener('keydown', e => {
    if (e.ctrlKey && !e.shiftKey && e.key === 'k') { e.preventDefault(); kb('search');   return; }
    if (e.ctrlKey && !e.shiftKey && e.key === 'l') { e.preventDefault(); kb('launch');   return; }
    if (e.ctrlKey && !e.shiftKey && e.key === '1') { e.preventDefault(); kb('settings'); return; }
    if (e.ctrlKey && !e.shiftKey && e.key === '2') { e.preventDefault(); kb('clients');  return; }
    if (e.ctrlKey && !e.shiftKey && e.key === '3') { e.preventDefault(); kb('versions'); return; }
    if (e.ctrlKey && !e.shiftKey && e.key === '4') { e.preventDefault(); kb('addons');   return; }
    if (e.ctrlKey && !e.shiftKey && e.key === '5') { e.preventDefault(); kb('servers');  return; }
    if (e.ctrlKey && (e.key === ',' || e.key === '?')) { e.preventDefault(); kb('settings'); return; }
    if (e.ctrlKey && (e.key === 'Tab' || e.code === 'Tab')) { e.preventDefault(); kb('cycle'); return; }
    if (e.ctrlKey && e.shiftKey && (e.key === 'r' || e.key === 'R')) { e.preventDefault(); kb('refresh'); return; }
    if (e.ctrlKey && e.shiftKey && (e.key === 't' || e.key === 'T')) { e.preventDefault(); kb('cycletheme'); return; }
    if (e.ctrlKey && e.shiftKey && (e.key === 'a' || e.key === 'A')) { e.preventDefault(); kb('cycleaccent'); return; }
    if (e.ctrlKey && e.shiftKey && (e.key === 'd' || e.key === 'D')) { e.preventDefault(); kb('diagnostics'); return; }
    if (e.key === 'F11')       { e.preventDefault(); kb('fullscreen'); return; }
    if (e.key === 'Escape')    { kb('escape'); return; }
    if (e.key === 'ArrowDown') { kb('down'); return; }
    if (e.key === 'ArrowUp')   { kb('up'); return; }
    if (e.key === 'Enter')     { kb('enter'); }
});

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

    document.documentElement.classList.toggle('no-blur', Number(px) <= 0);
};

window.setCustomBackground = (filePath) => {
    const bg = document.querySelector('.window-bg');
    if (!bg) return;
    if (filePath) {
        const fileName = filePath.replace(/\\/g, '/').split('/').pop();
        bg.style.backgroundImage = `url('https://glacier-files.local/${encodeURIComponent(fileName)}?t=${Date.now()}')`;
    } else {
        bg.style.backgroundImage = "url('../images/bg.jpg')";
    }
};

window.setCompactMode = (enabled) => {
    document.documentElement.classList.toggle('compact', !!enabled);
};

window._glacierAnimEnabled = true;
window._glacierAnimSpeed   = 1;

function recomputeNoAnimations() {
    document.documentElement.classList.toggle('no-animations',
        !window._glacierAnimEnabled || window._glacierAnimSpeed <= 0);
}

window.setAnimationsEnabled = (enabled) => {
    window._glacierAnimEnabled = !!enabled;
    recomputeNoAnimations();
};

window.setAnimationSpeed = (speed) => {
    const s = Math.max(0, Math.min(3, Number(speed) || 0));
    window._glacierAnimSpeed = s;
    document.documentElement.style.setProperty('--anim-mult', s > 0 ? String(1 / s) : '1');
    recomputeNoAnimations();
};

// Takes a plain callback. The original received a Blazor `dotNetRef` and
// invoked its `ApplyPendingNav` method; under Tauri there is no .NET object
// to call back into, so the navigation is passed straight in.
window.glacierViewTransition = (apply) => {
    const root = document.documentElement;
    if (!document.startViewTransition
        || root.classList.contains('no-animations')
        || document.querySelector('.panel-overlay.dragging')) {
        return apply();
    }
    root.classList.add('vt-active');
    let vt;
    try {
        vt = document.startViewTransition(async () => {
            const mutated = new Promise((resolve) => {
                const mo = new MutationObserver(() => { mo.disconnect(); resolve(); });
                mo.observe(document.body, { childList: true, subtree: true, attributes: true });
                setTimeout(() => { mo.disconnect(); resolve(); }, 250);
            });
            await apply();
            await mutated;
        });
    } catch {
        root.classList.remove('vt-active');
        return apply();
    }
    return vt.finished.catch(() => { }).finally(() => {
        document.querySelectorAll('.panel-overlay').forEach(el => { el.style.animation = 'none'; });
        root.classList.remove('vt-active');
    });
};

window.applyStoredSettings = (accent, theme, blur, customBg, compact, animations, animSpeed) => {
    if (accent) window.setAccentColor(accent);
    if (theme)  window.setTheme(theme);
    if (blur != null) window.setBlurIntensity(blur);
    if (customBg) window.setCustomBackground(customBg);
    if (compact != null) window.setCompactMode(compact);
    if (animations != null) window.setAnimationsEnabled(animations);
    if (animSpeed != null) window.setAnimationSpeed(animSpeed);
    applyScale();
};

window._glacierThemeKeys = [];
window.setThemeVars = (map) => {
    const root = document.documentElement;
    for (const k of window._glacierThemeKeys) root.style.removeProperty(k);
    window._glacierThemeKeys = [];
    if (!map) return;
    for (const [k, v] of Object.entries(map)) {
        root.style.setProperty(k, v);
        window._glacierThemeKeys.push(k);
    }
};

window.clearThemeVars = () => window.setThemeVars(null);

window.setFont = (family) => {
    document.body.style.fontFamily = family
        ? family + ", 'Segoe UI', system-ui, sans-serif"
        : '';
};

window.setCustomCss = (css) => {
    let tag = document.getElementById('glacier-custom-css');
    if (!css) { if (tag) tag.remove(); return; }
    if (!tag) {
        tag = document.createElement('style');
        tag.id = 'glacier-custom-css';
        document.head.appendChild(tag);
    }
    tag.textContent = css;
};

window.setBackgroundFit = (fit, opacity) => {
    const bg = document.querySelector('.window-bg');
    if (!bg) return;
    switch (fit) {
        case 'contain':
            bg.style.backgroundSize = 'contain';
            bg.style.backgroundRepeat = 'no-repeat';
            bg.style.backgroundPosition = 'center';
            break;
        case 'tile':
            bg.style.backgroundSize = 'auto';
            bg.style.backgroundRepeat = 'repeat';
            bg.style.backgroundPosition = 'top left';
            break;
        case 'center':
            bg.style.backgroundSize = 'auto';
            bg.style.backgroundRepeat = 'no-repeat';
            bg.style.backgroundPosition = 'center';
            break;
        default:
            bg.style.backgroundSize = 'cover';
            bg.style.backgroundRepeat = 'no-repeat';
            bg.style.backgroundPosition = 'center';
            break;
    }
    const op = Number(opacity);
    bg.style.opacity = isFinite(op) && op >= 0 && op <= 1 ? String(op) : '';
};

window.cpickFraction = (el, clientX, clientY) => {
    const r = el.getBoundingClientRect();
    const s = r.width  ? (clientX - r.left) / r.width  : 0;
    const v = r.height ? 1 - (clientY - r.top) / r.height : 0;
    return [Math.min(1, Math.max(0, s)), Math.min(1, Math.max(0, v))];
};

window.cpickCapture = (el, pointerId) => { try { el.setPointerCapture(pointerId); } catch (e) {} };

window.openDiscordOAuth = (clientId) => {
    const redirect = encodeURIComponent('https://glacierclient.xyz/auth');
    const url = `https://discord.com/api/oauth2/authorize?client_id=${clientId}&redirect_uri=${redirect}&response_type=token&scope=identify`;
    // TODO: swap for @tauri-apps/plugin-opener once that plugin is added to
    // src-tauri — window.open works for now since this just needs to launch
    // the system browser, not embed the OAuth flow.
    window.open(url, '_blank');
};

window.focusSearchInput = () => {
    setTimeout(() => { const el = document.querySelector('.search-modal-input'); if (el) el.focus(); }, 50);
};

window.glacierSkin = (function () {
    let libPromise = null;
    const viewers = {};

    function updatePauseState(canvasId) {
        const v = viewers[canvasId];
        if (v) v.renderPaused = document.hidden || !!v._glacierOffscreen;
    }

    document.addEventListener('visibilitychange', () => {
        for (const id of Object.keys(viewers)) updatePauseState(id);
    });

    const io = ('IntersectionObserver' in window)
        ? new IntersectionObserver(entries => {
            for (const e of entries) {
                const v = viewers[e.target.id];
                if (v) { v._glacierOffscreen = !e.isIntersecting; updatePauseState(e.target.id); }
            }
        }, { threshold: 0.01 })
        : null;

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
            if (await loadScript('/js/skinview3d.bundle.js') && window.skinview3d) return true;
            if (await loadScript('https://cdn.jsdelivr.net/npm/skinview3d@3/bundles/skinview3d.bundle.js') && window.skinview3d) return true;
            libPromise = null;
            return false;
        })();
        return libPromise;
    }

    return {
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
                if (capeUrl) { try { await viewer.loadCape(capeUrl); } catch (e) {   } }
                viewers[canvasId] = viewer;
                if (io) io.observe(canvas);
                updatePauseState(canvasId);
                return true;
            } catch (e) {
                return false;
            }
        },

        setModel: async function (canvasId, skinUrl, model) {
            const v = viewers[canvasId];
            if (!v) return false;
            try { await v.loadSkin(skinUrl, { model: model === 'slim' ? 'slim' : 'default' }); return true; }
            catch (e) { return false; }
        },

        setCape: async function (canvasId, capeUrl, mode) {
            const v = viewers[canvasId];
            if (!v) return false;
            try {
                if (mode === 'off' || !capeUrl) { v.resetCape(); return true; }
                await v.loadCape(capeUrl, { backEquipment: mode === 'elytra' ? 'elytra' : 'cape' });
                return true;
            } catch (e) { return false; }
        },

        probe: function (url) {
            return new Promise(resolve => {
                const img = new Image();
                img.onload  = () => resolve(true);
                img.onerror = () => resolve(false);
                img.src = url;
            });
        },
        dispose: function (canvasId) {
            if (io) { const c = document.getElementById(canvasId); if (c) io.unobserve(c); }
            if (viewers[canvasId]) { try { viewers[canvasId].dispose(); } catch (e) {} delete viewers[canvasId]; }
        }
    };
})();

(function () {
    const SNAP_POINTS_PCT = [40, 78, 95];
    const DISMISS_PCT     = 18;
    const MIN_PCT         = 12;

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
        const dy = e.clientY - drag.startY;
        const newH = drag.startH - dy;
        const pct = Math.max(0, (newH / drag.viewportH) * 100);

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
            overlay.style.height = '';
            if (window._glacierDotNet) {
                try { window._glacierDotNet.invokeMethodAsync('KbShortcut', 'escape'); } catch { }
            }
            return;
        }

        const snap = SNAP_POINTS_PCT.reduce(
            (best, p) => Math.abs(p - pct) < Math.abs(best - pct) ? p : best,
            SNAP_POINTS_PCT[0]
        );
        overlay.style.height = snap + '%';
    }

    document.addEventListener('pointerup', endDrag);
    document.addEventListener('pointercancel', endDrag);
})();

(function () {
    const PAGE_SIZE = 4;

    function tabsOf(bar) {
        return Array.from(bar.querySelectorAll(':scope > .panel-tab'));
    }

    function refreshCyclerButtons(bar) {
        const prev = bar.querySelector(':scope > .panel-tabs-arrow.prev');
        const next = bar.querySelector(':scope > .panel-tabs-arrow.next');
        if (!prev || !next) return;

        const tabs = tabsOf(bar);
        const totalPages = Math.max(1, Math.ceil(tabs.length / PAGE_SIZE));
        let page = parseInt(bar.dataset.page || '0', 10);

        const activeIdx = tabs.findIndex(t => t.classList.contains('active'));
        if (activeIdx >= 0) {
            const activePage = Math.floor(activeIdx / PAGE_SIZE);
            if (Math.floor(activeIdx / PAGE_SIZE) !== page && bar.dataset.userPaged !== String(page)) {
                page = activePage;
            }
        }
        page = Math.min(Math.max(page, 0), totalPages - 1);
        bar.dataset.page = String(page);

        tabs.forEach((tab, i) => {
            tab.style.display = Math.floor(i / PAGE_SIZE) === page ? '' : 'none';
        });

        const overflowing = totalPages > 1;
        prev.style.display = overflowing && page > 0 ? 'flex' : 'none';
        next.style.display = overflowing && page < totalPages - 1 ? 'flex' : 'none';
    }

    function ensureTabCyclers() {
        document.querySelectorAll('.panel-tabs:not(.cycler-ready)').forEach(bar => {
            bar.classList.add('cycler-ready');
            bar.dataset.page = '0';

            const prev = document.createElement('button');
            prev.type = 'button';
            prev.className = 'panel-tabs-arrow prev';
            prev.title = 'Previous tabs';
            prev.innerHTML = '<i class="fa-solid fa-chevron-left"></i>';
            prev.addEventListener('click', e => {
                e.stopPropagation();
                const p = Math.max(0, parseInt(bar.dataset.page || '0', 10) - 1);
                bar.dataset.page = String(p);
                bar.dataset.userPaged = String(p);
                refreshCyclerButtons(bar);
            });

            const next = document.createElement('button');
            next.type = 'button';
            next.className = 'panel-tabs-arrow next';
            next.title = 'Next tabs';
            next.innerHTML = '<i class="fa-solid fa-chevron-right"></i>';
            next.addEventListener('click', e => {
                e.stopPropagation();
                const totalPages = Math.max(1, Math.ceil(tabsOf(bar).length / PAGE_SIZE));
                const p = Math.min(totalPages - 1, parseInt(bar.dataset.page || '0', 10) + 1);
                bar.dataset.page = String(p);
                bar.dataset.userPaged = String(p);
                refreshCyclerButtons(bar);
            });

            bar.insertBefore(prev, bar.firstChild);
            bar.appendChild(next);
            refreshCyclerButtons(bar);
        });

        document.querySelectorAll('.panel-tabs.cycler-ready').forEach(refreshCyclerButtons);
    }

    const tabsObserver = new MutationObserver(() => ensureTabCyclers());
    tabsObserver.observe(document.body, { childList: true, subtree: true });
    window.addEventListener('resize', ensureTabCyclers);
    ensureTabCyclers();
})();
