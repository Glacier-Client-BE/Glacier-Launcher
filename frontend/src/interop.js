// Ported from the WPF app's wwwroot/js/interop.js. The WebView2-specific
// bridge (chrome.webview.postMessage / Blazor dotNetRef) is replaced with
// Tauri's window API and a `glacier:*` CustomEvent bus that Svelte listens
// to — everything else (scale, theming, panel drag-resize, tab cycler) is
// pure DOM manipulation and ports unchanged.
import { getCurrentWindow } from '@tauri-apps/api/window';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';

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

// Feature 37: a handful of the shortcuts below ("search", "launch",
// "fullscreen") can be rebound from Settings. Rebindings are persisted as a
// JSON string in `LauncherSettings.KeyboardShortcuts` (opaque to the Rust
// side) and applied here via `setKeyboardShortcuts`. Everything else stays
// hardcoded, same as before.
window._glacierShortcuts = {};

/** Normalizes a KeyboardEvent into a combo string like "ctrl+shift+k". */
function comboFromEvent(e) {
    const parts = [];
    if (e.ctrlKey) parts.push('ctrl');
    if (e.altKey) parts.push('alt');
    if (e.shiftKey) parts.push('shift');
    const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;
    if (!['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) parts.push(key);
    return parts.join('+');
}

window.setKeyboardShortcuts = (json) => {
    try { window._glacierShortcuts = JSON.parse(json || '{}') || {}; }
    catch { window._glacierShortcuts = {}; }
};

/** Rebindable-action default combos, mirrored in SettingsView.svelte. */
const REBINDABLE_DEFAULTS = { search: 'ctrl+k', launch: 'ctrl+l', fullscreen: 'f11' };

function matchesAction(e, action) {
    const combo = window._glacierShortcuts[action] || REBINDABLE_DEFAULTS[action];
    return combo === comboFromEvent(e);
}

document.addEventListener('keydown', e => {
    // Skip rebindable-action matching while a rebind capture UI owns the
    // keydown (it stops propagation itself), and skip while typing in a
    // text field so single-letter custom bindings don't fire mid-sentence.
    const inField = /^(input|textarea)$/i.test(e.target?.tagName || '') && !window._glacierCapturingShortcut;

    if (!inField && matchesAction(e, 'search'))     { e.preventDefault(); kb('search');   return; }
    if (!inField && matchesAction(e, 'launch'))     { e.preventDefault(); kb('launch');   return; }
    if (!inField && matchesAction(e, 'fullscreen')) { e.preventDefault(); kb('fullscreen'); return; }

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
    // Ctrl+F focuses the filter box of the open panel (Ctrl+K stays global search).
    if (e.ctrlKey && !e.shiftKey && !e.altKey && (e.key === 'f' || e.key === 'F')) {
        const box = [...document.querySelectorAll('.panel-search-input')].find(el => el.offsetParent !== null);
        if (box) { e.preventDefault(); box.focus(); box.select(); return; }
    }
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

window.setTooltipDelay = (ms) => {
    const n = Math.max(0, Number(ms) || 0);
    document.documentElement.style.setProperty('--tooltip-delay', n + 'ms');
};

window.setBlurIntensity = (px) => {
    document.documentElement.style.setProperty('--blur', px + 'px');

    document.documentElement.classList.toggle('no-blur', Number(px) <= 0);
};

// TODO(UI polish feature 39 - live/animated wallpaper): intentionally
// skipped. The target hardware (Celeron N5100) has no headroom for a
// per-frame animated background; static wallpaper only.
window.setCustomBackground = (filePath) => {
    const bg = document.querySelector('.window-bg');
    if (!bg) return;
    if (filePath) {
        // Tauri's asset protocol (enabled with scope: ["**"] in
        // tauri.conf.json) serves any local file the user picked, without
        // needing to copy it into the launcher's own folder first.
        bg.style.backgroundImage = `url('${convertFileSrc(filePath)}')`;
    } else {
        bg.style.backgroundImage = "url('../images/bg.jpg')";
    }
};

window.setCompactMode = (enabled) => {
    document.documentElement.classList.toggle('compact', !!enabled);
    // Feature 36: also expose it as a `data-density` attribute so any new
    // CSS (list rows, etc.) can target `[data-density="compact"]` alongside
    // the existing `.compact` class rules.
    if (enabled) document.documentElement.setAttribute('data-density', 'compact');
    else document.documentElement.removeAttribute('data-density');
};

window.setPanelSide = (side) => {
    document.documentElement.setAttribute('data-panel-side', side === 'left' ? 'left' : 'right');
};

window.setPanelWidth = (px) => {
    const n = Number(px);
    if (n > 0) document.documentElement.style.setProperty('--sidebar-w-user', n + 'px');
    else document.documentElement.style.removeProperty('--sidebar-w-user');
};

window.setAccessibilityMode = (enabled) => {
    window._glacierA11y = !!enabled;
    document.documentElement.classList.toggle('a11y-mode', !!enabled);
    recomputeNoAnimations();
};

window._glacierAnimEnabled = true;
window._glacierAnimSpeed   = 1;
window._glacierA11y        = false;

function recomputeNoAnimations() {
    document.documentElement.classList.toggle('no-animations',
        window._glacierA11y || !window._glacierAnimEnabled || window._glacierAnimSpeed <= 0);
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
        || root.classList.contains('no-animations')) {
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

window.applyStoredSettings = (accent, theme, blur, customBg, compact, animations, animSpeed, panelSide, panelWidth) => {
    if (accent) window.setAccentColor(accent);
    if (theme)  window.setTheme(theme);
    if (blur != null) window.setBlurIntensity(blur);
    if (customBg) window.setCustomBackground(customBg);
    if (compact != null) window.setCompactMode(compact);
    if (animations != null) window.setAnimationsEnabled(animations);
    if (animSpeed != null) window.setAnimationSpeed(animSpeed);
    if (panelSide) window.setPanelSide(panelSide);
    if (panelWidth != null) window.setPanelWidth(panelWidth);
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
    invoke('open_url', { url }).catch(() => {});
};

window.focusSearchInput = () => {
    setTimeout(() => { const el = document.querySelector('.search-modal-input'); if (el) el.focus(); }, 50);
};

(function () {
    // The panel sidebar is a fixed ~460px width (see .panel-overlay), so any
    // strip of tabs wide enough to overflow it needs the same prev/next
    // paging — not just the bottom `.panel-tabs` strip, but the in-panel
    // `.versions-client-switcher` strip too (Utilities' 6 sub-tabs, in
    // particular, don't all fit). Each strip sizes its own page: how many
    // tabs actually fit is measured live from the rendered items rather than
    // a shared guess, so a strip on its own full-width line (like the
    // Client Library version filter) pages fewer times than a narrower one
    // squeezed next to a search box.
    // `.switcher-wrap` bars (CurseForge/Modrinth category filters, etc.) opt
    // out of paging entirely — they already handle overflow by wrapping to a
    // second row via CSS, so adding JS-driven arrows on top of that fights
    // the wrap and hides tabs that would otherwise just drop to the next
    // line, which is what made those bars look inconsistent with the ones
    // that actually page.
    const STRIPS = [
        { bar: '.panel-tabs', item: '.panel-tab' },
        { bar: '.versions-client-switcher:not(.switcher-wrap)', item: '.vcs-btn' },
        { bar: '.mcv-filter-bar .mcv-channel-tabs', item: '.mcv-channel-tab' },
    ];

    function tabsOf(bar, itemSelector) {
        return Array.from(bar.querySelectorAll(':scope > ' + itemSelector));
    }

    // How many tabs fit per page, measured from the bar's actual content
    // width and the first tab's actual rendered width (+ gap) — not a fixed
    // guess, so it adapts to however wide the strip's own line is.
    function pageSizeFor(bar, tabs) {
        if (tabs.length === 0) return 1;
        const barStyle = getComputedStyle(bar);
        const gap = parseFloat(barStyle.columnGap || barStyle.gap || '0') || 0;

        // .vcs-btn uses flex: 1 1 0, so a tab's rendered width is
        // bar.clientWidth divided by however many tabs are CURRENTLY
        // visible — not its true share of all of them. If a previous pass
        // already (wrongly) hid every tab but one, that lone survivor
        // stretches to fill the whole bar, gets sampled here, and reports a
        // width of ~bar.clientWidth. That collapses perPageNoArrows to 1,
        // which hides all-but-one again — a self-reinforcing lock that nothing
        // (not even the ResizeObserver below, since the bar's own box never
        // changes size) could ever break out of. Reset every tab to visible
        // before measuring so the sample always reflects the honest N-way
        // split; refreshCyclerButtons re-applies the real per-tab display
        // right after this returns.
        const prevDisplay = tabs.map(t => t.style.display);
        tabs.forEach(t => { t.style.display = ''; });
        const sample = tabs.find(t => t.getBoundingClientRect().width > 0) || tabs[0];
        const tabWidth = sample.getBoundingClientRect().width || 1;
        tabs.forEach((t, i) => { t.style.display = prevDisplay[i]; });

        // First check whether every tab fits on one page with NO arrows at
        // all (bar.clientWidth is the container's own width regardless of
        // its children, so this is the true "no cycler needed" width). Only
        // when that fails do we reserve room for the prev/next buttons and
        // recompute — otherwise a strip that could show all 2-3 tabs at
        // once was being forced down to 1 by unconditionally reserving
        // arrow space it would never actually need.
        const perPageNoArrows = Math.floor((bar.clientWidth + gap) / (tabWidth + gap));
        if (perPageNoArrows >= tabs.length) return tabs.length;

        const arrowsReserved = 2 * 34 + 2 * gap; // prev + next .panel-tabs-arrow, when both showing
        const available = bar.clientWidth - arrowsReserved;
        const perPage = Math.floor((available + gap) / (tabWidth + gap));
        return Math.max(1, perPage);
    }

    function refreshCyclerButtons(bar, itemSelector) {
        const prev = bar.querySelector(':scope > .panel-tabs-arrow.prev');
        const next = bar.querySelector(':scope > .panel-tabs-arrow.next');
        if (!prev || !next) return;

        const tabs = tabsOf(bar, itemSelector);
        const pageSize = pageSizeFor(bar, tabs);
        const totalPages = Math.max(1, Math.ceil(tabs.length / pageSize));
        let page = parseInt(bar.dataset.page || '0', 10);

        const activeIdx = tabs.findIndex(t => t.classList.contains('active'));
        if (activeIdx >= 0) {
            const activePage = Math.floor(activeIdx / pageSize);
            if (Math.floor(activeIdx / pageSize) !== page && bar.dataset.userPaged !== String(page)) {
                page = activePage;
            }
        }
        page = Math.min(Math.max(page, 0), totalPages - 1);
        bar.dataset.page = String(page);
        bar.dataset.pageSize = String(pageSize);

        tabs.forEach((tab, i) => {
            tab.style.display = Math.floor(i / pageSize) === page ? '' : 'none';
        });

        const overflowing = totalPages > 1;
        prev.style.display = overflowing && page > 0 ? 'flex' : 'none';
        next.style.display = overflowing && page < totalPages - 1 ? 'flex' : 'none';
    }

    function ensureTabCyclers() {
        for (const { bar: barSelector, item: itemSelector } of STRIPS) {
            document.querySelectorAll(barSelector + ':not(.cycler-ready)').forEach(bar => {
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
                    refreshCyclerButtons(bar, itemSelector);
                });

                const next = document.createElement('button');
                next.type = 'button';
                next.className = 'panel-tabs-arrow next';
                next.title = 'Next tabs';
                next.innerHTML = '<i class="fa-solid fa-chevron-right"></i>';
                next.addEventListener('click', e => {
                    e.stopPropagation();
                    const pageSize = parseInt(bar.dataset.pageSize || '1', 10);
                    const totalPages = Math.max(1, Math.ceil(tabsOf(bar, itemSelector).length / pageSize));
                    const p = Math.min(totalPages - 1, parseInt(bar.dataset.page || '0', 10) + 1);
                    bar.dataset.page = String(p);
                    bar.dataset.userPaged = String(p);
                    refreshCyclerButtons(bar, itemSelector);
                });

                bar.insertBefore(prev, bar.firstChild);
                bar.appendChild(next);
                refreshCyclerButtons(bar, itemSelector);

                // A bar is often inserted mid slide-in animation (or before
                // its panel is done laying out), so the very first
                // measurement can lock in a bogus pageSize (e.g. 1) that
                // then never corrects itself since nothing but a window
                // resize used to trigger a re-measure. Watch the bar itself
                // so it re-pages as soon as its real size settles.
                sizeObserver.observe(bar);
            });

            document.querySelectorAll(barSelector + '.cycler-ready').forEach(bar => refreshCyclerButtons(bar, itemSelector));
        }
    }

    const sizeObserver = new ResizeObserver(entries => {
        for (const entry of entries) {
            const bar = entry.target;
            const strip = STRIPS.find(s => bar.matches(s.bar));
            if (strip) refreshCyclerButtons(bar, strip.item);
        }
    });

    let cyclerFrame = 0;
    const tabsObserver = new MutationObserver(() => {
        if (cyclerFrame) return;
        cyclerFrame = requestAnimationFrame(() => {
            cyclerFrame = 0;
            ensureTabCyclers();
        });
    });
    tabsObserver.observe(document.body, { childList: true, subtree: true });
    window.addEventListener('resize', ensureTabCyclers);
    ensureTabCyclers();
})();
