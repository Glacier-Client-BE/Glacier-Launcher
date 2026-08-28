// Live theme application — adapted near-verbatim from the desktop app's own
// wwwroot/js/interop.js (already plain browser JS, no Blazor dependency),
// plus a JS port of Models/ThemeDefinition.cs's BuildCssVars() so custom
// themes compute the exact same derived CSS variables (accent glow/hover,
// background overlays) the desktop app does.
const ThemeEngine = {
    _appliedKeys: [],

    parseColor(color) {
        try {
            const s = (color || "").trim();
            if (s.toLowerCase().startsWith("rgb")) {
                const inner = s.slice(s.indexOf("(") + 1, s.indexOf(")"));
                const [r, g, b, a] = inner.split(",").map(v => v.trim());
                return { r: parseInt(r), g: parseInt(g), b: parseInt(b), a: a !== undefined ? parseFloat(a) : 1.0 };
            }
            let h = s.replace("#", "");
            if (h.length === 3) h = h.split("").map(c => c + c).join("");
            const r = parseInt(h.slice(0, 2), 16), g = parseInt(h.slice(2, 4), 16), b = parseInt(h.slice(4, 6), 16);
            const a = h.length >= 8 ? parseInt(h.slice(6, 8), 16) / 255 : 1.0;
            return { r, g, b, a };
        } catch (e) {
            return { r: 114, g: 137, b: 218, a: 1.0 };
        }
    },

    lighten(color, amount) {
        const { r, g, b, a } = this.parseColor(color);
        const L = (c) => Math.max(0, Math.min(255, Math.round(c + (255 - c) * amount)));
        const lr = L(r), lg = L(g), lb = L(b);
        if (a >= 0.999) return `#${lr.toString(16).padStart(2, "0")}${lg.toString(16).padStart(2, "0")}${lb.toString(16).padStart(2, "0")}`;
        return `rgba(${lr},${lg},${lb},${a})`;
    },

    // Mirrors ThemeDefinition.BuildCssVars() exactly.
    buildCssVars(t) {
        const { r: ar, g: ag, b: ab } = this.parseColor(t.accent);
        const { r: br, g: bg2, b: bb } = this.parseColor(t.bg);
        const { r: tr, g: tg, b: tb } = this.parseColor(t.text);
        const s = Math.max(0, Math.min(1, t.overlayStrength));
        const lightText = (tr + tg + tb) / 3.0 > 128;
        const F = (v) => Number(v.toFixed(3));

        return {
            "--accent": t.accent,
            "--accent-hover": this.lighten(t.accent, 0.12),
            "--accent-glow": `rgba(${ar},${ag},${ab},0.42)`,
            "--accent-bg": `rgba(${ar},${ag},${ab},0.10)`,
            "--bg": t.bg,
            "--bg-panel": t.bgPanel,
            "--bg-item": `rgba(${tr},${tg},${tb},${lightText ? "0.04" : "0.05"})`,
            "--bg-item-hover": `rgba(${tr},${tg},${tb},${lightText ? "0.075" : "0.09"})`,
            "--text": t.text,
            "--text-dim": t.textDim,
            "--red": t.red,
            "--green": t.green,
            "--orange": t.orange,
            "--r-sm": `${t.radiusSm}px`,
            "--r-md": `${t.radiusMd}px`,
            "--overlay-top": `rgba(${br},${bg2},${bb},${F(0.55 * s)})`,
            "--overlay-mid": `rgba(${br},${bg2},${bb},${F(0.20 * s)})`,
            "--overlay-bot": `rgba(${br},${bg2},${bb},${F(0.85 * s)})`,
        };
    },

    setThemeVars(map) {
        const root = document.documentElement;
        for (const k of this._appliedKeys) root.style.removeProperty(k);
        this._appliedKeys = [];
        if (!map) return;
        for (const [k, v] of Object.entries(map)) {
            root.style.setProperty(k, v);
            this._appliedKeys.push(k);
        }
    },

    clearThemeVars() {
        this.setThemeVars(null);
    },

    setBasePreset(preset) {
        document.documentElement.setAttribute("data-theme", preset === "dark" ? "" : preset);
    },

    setBlurIntensity(px) {
        document.documentElement.style.setProperty("--blur", px + "px");
        document.documentElement.classList.toggle("no-blur", Number(px) <= 0);
    },

    setAnimationSpeed(speed) {
        const sp = Math.max(0, Math.min(3, Number(speed) || 0));
        document.documentElement.style.setProperty("--anim-mult", sp > 0 ? String(1 / sp) : "1");
        document.documentElement.classList.toggle("no-animations", sp <= 0);
    },

    setFont(family) {
        document.body.style.fontFamily = family ? `${family}, 'Segoe UI', system-ui, sans-serif` : "";
    },

    setCustomCss(css) {
        let tag = document.getElementById("glacier-custom-css");
        if (!css) { if (tag) tag.remove(); return; }
        if (!tag) {
            tag = document.createElement("style");
            tag.id = "glacier-custom-css";
            document.head.appendChild(tag);
        }
        tag.textContent = css;
    },

    // Wallpaper picking needs real file access this WebView doesn't have a
    // bridge for yet — theme wallpaper stays the default background.
    apply(t) {
        this.setBasePreset(t.basePreset);
        this.setThemeVars(this.buildCssVars(t));
        this.setFont(t.fontFamily);
        this.setBlurIntensity(t.blur);
        this.setAnimationSpeed(t.animationSpeed);
        this.setCustomCss(t.customCss);
    },

    clear() {
        this.setBasePreset("dark");
        this.clearThemeVars();
        this.setFont("");
        this.setCustomCss("");
    },
};
