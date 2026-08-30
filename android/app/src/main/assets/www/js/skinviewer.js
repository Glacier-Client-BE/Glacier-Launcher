// Interactive 3D skin preview for the Java Profile panel — Android port of
// wwwroot/js/interop.js's `window.glacierSkin` (backs Components/SkinViewer.razor).
// Same skinview3d library, same vendored-first/CDN-fallback load strategy, same
// WebGL-loop pause on visibilitychange/IntersectionObserver so a hidden/offscreen
// canvas doesn't keep spinning. Android has no cape/wardrobe data (no cape URL
// anywhere in AndroidBridge or settings), so the cape-cycling piece of the
// desktop component is intentionally left out — this only ports the skin model
// itself (2D static render <-> 3D interactive, Steve/Alex arm model).
window.GlacierSkin = (function () {
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
            s.onload = () => resolve(true);
            s.onerror = () => resolve(false);
            document.head.appendChild(s);
        });
    }

    function ensureLib() {
        if (window.skinview3d) return Promise.resolve(true);
        if (libPromise) return libPromise;
        libPromise = (async () => {
            if (await loadScript('js/lib/skinview3d.bundle.js') && window.skinview3d) return true;
            if (await loadScript('https://cdn.jsdelivr.net/npm/skinview3d@3/bundles/skinview3d.bundle.js') && window.skinview3d) return true;
            libPromise = null;
            return false;
        })();
        return libPromise;
    }

    return {
        // Returns true if the 3D viewer initialised; false -> caller falls back to the static render.
        render: async function (canvasId, skinUrl, model) {
            try {
                if (!await ensureLib()) return false;
                const canvas = document.getElementById(canvasId);
                if (!canvas) return false;
                if (viewers[canvasId]) { try { viewers[canvasId].dispose(); } catch (e) {} delete viewers[canvasId]; }

                const w = canvas.clientWidth || 240;
                const h = canvas.clientHeight || 340;
                const viewer = new skinview3d.SkinViewer({ canvas, width: w, height: h });
                try { await viewer.loadSkin(skinUrl, { model: model === 'slim' ? 'slim' : 'default' }); }
                catch (e) { return false; }
                viewer.autoRotate = true;
                viewer.autoRotateSpeed = 0.55;
                viewer.zoom = 0.85;
                try { viewer.animation = new skinview3d.WalkingAnimation(); viewer.animation.speed = 0.6; } catch (e) {}
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
        dispose: function (canvasId) {
            if (io) { const c = document.getElementById(canvasId); if (c) io.unobserve(c); }
            if (viewers[canvasId]) { try { viewers[canvasId].dispose(); } catch (e) {} delete viewers[canvasId]; }
        }
    };
})();
