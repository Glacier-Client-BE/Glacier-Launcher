import React from "react";
import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { c, r, ease, display, body } from "../theme";

const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

// Fade + rise (30px) for a text line starting at `start`.
const rise = (frame: number, start: number) => {
  const t = interpolate(frame, [start, start + 18], [0, 1], { ...clamp, easing: ease });
  return {
    opacity: t,
    transform: `translateY(${interpolate(t, [0, 1], [30, 0])}px)`,
  };
};

// Slide in from the right for a card starting at `start`.
const slideIn = (frame: number, start: number) => {
  const t = interpolate(frame, [start, start + 20], [0, 1], { ...clamp, easing: ease });
  return {
    opacity: t,
    transform: `translateX(${interpolate(t, [0, 1], [120, 0])}px)`,
  };
};

const icons: Record<string, React.ReactNode> = {
  bolt: <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" />,
  download: (
    <>
      <path d="M12 3v12" />
      <path d="M7 10l5 5 5-5" />
      <path d="M5 21h14" />
    </>
  ),
  layers: (
    <>
      <path d="M12 3l9 5-9 5-9-5 9-5z" />
      <path d="M3 13l9 5 9-5" />
    </>
  ),
};

const cards = [
  { icon: "bolt", title: "Tauri + Svelte 5", sub: "Native Rust core" },
  { icon: "download", title: "Parallel downloads", sub: "Reuses files from other launchers" },
  { icon: "layers", title: "Redesigned UI", sub: "Cleaner, grouped and faster to use" },
] as const;

const SHOT_WIDTH = 1150;

export const Speed = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  // Screenshot fades in, then drifts slowly left across the whole scene.
  const shotIn = interpolate(frame, [0, 30], [0, 0.9], { ...clamp, easing: ease });
  const drift = interpolate(frame, [0, durationInFrames], [0, -40], clamp);

  return (
    <AbsoluteFill
      style={{
        background: c.bg,
        padding: "120px 140px",
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 80,
        fontFamily: body,
        color: c.text,
        overflow: "hidden",
      }}
    >
      {/* Layered glow behind the screenshot */}
      <div
        style={{
          position: "absolute",
          left: 760,
          top: 120,
          width: 1200,
          height: 840,
          borderRadius: "50%",
          background:
            "radial-gradient(closest-side, rgba(114,137,218,0.25) 0%, rgba(114,137,218,0.12) 45%, rgba(114,137,218,0) 100%)",
          opacity: shotIn,
          pointerEvents: "none",
        }}
      />

      {/* Tilted settings window, behind the cards */}
      <div
        style={{
          position: "absolute",
          left: 880,
          top: 218,
          width: SHOT_WIDTH,
          opacity: shotIn,
          transform: `translateX(${drift}px) perspective(2000px) rotateY(-14deg)`,
          transformOrigin: "center center",
          borderRadius: 18,
          border: "1px solid rgba(255,255,255,0.10)",
          boxShadow: "0 30px 80px rgba(0,0,0,0.55), 0 0 120px rgba(114,137,218,0.22), inset 0 1px 0 rgba(255,255,255,0.04)",
          overflow: "hidden",
        }}
      >
        <Img src={staticFile("shots/settings.png")} style={{ width: SHOT_WIDTH, height: "auto", display: "block" }} />
      </div>

      {/* Left half */}
      <div style={{ width: 820, display: "flex", flexDirection: "column", position: "relative" }}>
        <div
          style={{
            ...rise(frame, 0),
            fontSize: 22,
            fontWeight: 800,
            textTransform: "uppercase",
            letterSpacing: "0.12em",
            color: c.accentHover,
            marginBottom: 28,
          }}
        >
          WHAT'S NEW
        </div>
        <div
          style={{
            ...rise(frame, 8),
            fontFamily: display,
            fontWeight: 700,
            fontSize: 120,
            lineHeight: 1.02,
            letterSpacing: "-0.025em",
            color: c.text,
            margin: 0,
          }}
        >
          Rebuilt in Rust.
        </div>
        <div
          style={{
            ...rise(frame, 16),
            fontSize: 36,
            fontWeight: 400,
            lineHeight: 1.4,
            color: c.dim,
            marginTop: 36,
          }}
        >
          Faster start-up. Lower memory. Built for low-end laptops.
        </div>
      </div>

      {/* Right half: stacked cards, over the screenshot */}
      <div style={{ width: 640, display: "flex", flexDirection: "column", gap: 24, position: "relative" }}>
        {cards.map((card, i) => (
          <div
            key={card.title}
            style={{
              ...slideIn(frame, 40 + i * 12),
              background: c.panel,
              border: `1px solid ${c.border}`,
              boxShadow: "inset 0 1px 0 rgba(255,255,255,0.04)",
              borderRadius: r.md,
              padding: "28px 32px",
              display: "flex",
              alignItems: "center",
              gap: 24,
            }}
          >
            <div
              style={{
                width: 48,
                height: 48,
                borderRadius: 12,
                background: c.accentBg,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                flexShrink: 0,
              }}
            >
              <svg
                width={26}
                height={26}
                viewBox="0 0 24 24"
                fill="none"
                stroke={c.accentHover}
                strokeWidth={2}
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                {icons[card.icon]}
              </svg>
            </div>
            <div style={{ display: "flex", flexDirection: "column" }}>
              <div style={{ fontWeight: 600, fontSize: 30, color: c.text }}>{card.title}</div>
              <div style={{ fontWeight: 400, fontSize: 22, color: c.dim, marginTop: 6 }}>
                {card.sub}
              </div>
            </div>
          </div>
        ))}
      </div>
    </AbsoluteFill>
  );
};
