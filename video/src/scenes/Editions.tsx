import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { body, c, display, ease, r } from "../theme";

const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

const editions = [
  { label: "Bedrock", icon: "mc-logo.png", active: false },
  { label: "Java", icon: "mc-java-logo.png", active: true },
  { label: "Horizon", icon: "horizon.svg", active: false },
  { label: "Lunar Client", icon: "lunar.png", active: false },
];

export const Editions = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const headlineT = interpolate(frame, [0, 20], [0, 1], { ...clamp, easing: ease });

  const shotT = interpolate(frame, [10, 40], [0, 1], { ...clamp, easing: ease });
  // Slow Ken Burns push across the whole scene.
  const kenBurns = interpolate(frame, [0, durationInFrames], [1, 1.06], clamp);

  return (
    <AbsoluteFill style={{ background: c.bg }}>
      {/* Layered glow above the screenshot */}
      <div
        style={{
          position: "absolute",
          left: (1920 - 1500) / 2,
          top: 130,
          width: 1500,
          height: 840,
          borderRadius: "50%",
          background:
            "radial-gradient(closest-side, rgba(114,137,218,0.25) 0%, rgba(114,137,218,0.12) 45%, rgba(114,137,218,0) 100%)",
          opacity: shotT,
          pointerEvents: "none",
        }}
      />

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
          opacity: headlineT,
          transform: `translateY(${(1 - headlineT) * 24}px)`,
        }}
      >
        One launcher. Every edition.
      </div>

      <div
        style={{
          position: "absolute",
          top: 200,
          left: (1920 - 1300) / 2,
          width: 1300,
          borderRadius: 18,
          border: "1px solid rgba(255,255,255,0.10)",
          boxShadow: "0 30px 80px rgba(0,0,0,0.55), 0 0 120px rgba(114,137,218,0.22), inset 0 1px 0 rgba(255,255,255,0.04)",
          overflow: "hidden",
          opacity: shotT,
          transform: `scale(${kenBurns})`,
          transformOrigin: "center center",
        }}
      >
        <Img src={staticFile("shots/home.png")} style={{ width: 1300, height: "auto", display: "block" }} />
      </div>

      <div
        style={{
          position: "absolute",
          top: 968,
          left: 0,
          right: 0,
          display: "flex",
          justifyContent: "center",
          gap: 24,
        }}
      >
        {editions.map((e, i) => {
          const start = 70 + i * 8;
          const p = interpolate(frame, [start, start + 14], [0, 1], { ...clamp, easing: ease });
          return (
            <div
              key={e.label}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 14,
                padding: "14px 28px",
                borderRadius: r.pill,
                background: e.active ? c.accent : c.panel,
                border: `1px solid ${c.border}`,
                boxShadow: "inset 0 1px 0 rgba(255,255,255,0.04)",
                opacity: p,
                transform: `translateY(${(1 - p) * 20}px)`,
              }}
            >
              <Img src={staticFile(e.icon)} style={{ width: 40, height: 40, objectFit: "contain" }} />
              <span
                style={{
                  fontFamily: body,
                  fontWeight: 600,
                  fontSize: 30,
                  color: c.text,
                  whiteSpace: "nowrap",
                }}
              >
                {e.label}
              </span>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
