import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { body, c, display, ease, r } from "../theme";

const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

const headline = "All your tools. One place.".split(" ");

const chips = ["World backups", "Vanilla Tweaks", "Screenshots"];

export const Tools = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const shotT = interpolate(frame, [10, 45], [0, 1], { ...clamp, easing: ease });
  const shotScale = interpolate(frame, [0, durationInFrames], [1, 1.04], clamp);
  const glowOpacity = 0.4 + 0.6 * shotT;

  return (
    <AbsoluteFill style={{ background: c.bg }}>
      <div
        style={{
          position: "absolute",
          top: 56,
          left: 0,
          right: 0,
          textAlign: "center",
          fontFamily: display,
          fontWeight: 700,
          fontSize: 96,
          letterSpacing: "-0.025em",
          lineHeight: 1.05,
          color: c.text,
        }}
      >
        {headline.map((word, i) => {
          const p = interpolate(frame, [i * 5, i * 5 + 16], [0, 1], { ...clamp, easing: ease });
          return (
            <span
              key={word + i}
              style={{
                display: "inline-block",
                marginRight: "0.25em",
                opacity: p,
                transform: `translateY(${(1 - p) * 30}px)`,
              }}
            >
              {word}
            </span>
          );
        })}
      </div>

      <div
        style={{
          position: "absolute",
          top: 230 - 160,
          left: (1920 - 1400) / 2 - 160,
          width: 1400 + 320,
          height: 1000 + 320,
          background:
            "radial-gradient(ellipse at center, rgba(114,137,218,0.25) 0%, rgba(114,137,218,0.10) 40%, rgba(114,137,218,0) 70%)",
          opacity: glowOpacity,
          pointerEvents: "none",
        }}
      />

      {/* Smaller profile window, offset behind the tweaks window */}
      <div
        style={{
          position: "absolute",
          top: 330,
          left: (1920 - 1400) / 2 - 120,
          width: 620,
          borderRadius: r.md,
          border: "1px solid rgba(255,255,255,0.10)",
          boxShadow: "0 30px 80px rgba(0,0,0,0.55), inset 0 1px 0 rgba(255,255,255,0.04)",
          overflow: "hidden",
          opacity: shotT,
          transform: `translateY(${(1 - shotT) * 80}px)`,
        }}
      >
        <Img src={staticFile("shots/profile.png")} style={{ width: 620, height: "auto", display: "block" }} />
      </div>

      <div
        style={{
          position: "absolute",
          top: 230,
          left: (1920 - 1400) / 2,
          width: 1400,
          borderRadius: r.md,
          border: "1px solid rgba(255,255,255,0.10)",
          boxShadow:
            "0 30px 80px rgba(0,0,0,0.55), 0 0 120px rgba(114,137,218,0.22), inset 0 1px 0 rgba(255,255,255,0.04)",
          overflow: "hidden",
          opacity: shotT,
          transform: `translateY(${(1 - shotT) * 60}px) scale(${shotScale})`,
          transformOrigin: "center center",
        }}
      >
        <Img src={staticFile("shots/tweaks.png")} style={{ width: 1400, height: "auto", display: "block" }} />
      </div>

      <div
        style={{
          position: "absolute",
          top: 940,
          left: 0,
          right: 0,
          display: "flex",
          justifyContent: "center",
          gap: 24,
        }}
      >
        {chips.map((label, i) => {
          const start = 80 + i * 6;
          const p = interpolate(frame, [start, start + 14], [0, 1], { ...clamp, easing: ease });
          const accent = label === "Vanilla Tweaks";
          return (
            <div
              key={label}
              style={{
                padding: "12px 26px",
                borderRadius: r.pill,
                background: accent ? c.accent : c.item,
                border: "1px solid rgba(255,255,255,0.10)",
                boxShadow: "inset 0 1px 0 rgba(255,255,255,0.04)",
                fontFamily: body,
                fontWeight: 600,
                fontSize: 26,
                color: accent ? c.bg : c.text,
                whiteSpace: "nowrap",
                opacity: p,
                transform: `translateY(${(1 - p) * 20}px)`,
              }}
            >
              {label}
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
