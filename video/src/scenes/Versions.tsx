import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { body, c, display, ease, r } from "../theme";

const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

const windowStyle = {
  borderRadius: r.md,
  border: "1px solid rgba(255,255,255,0.10)",
  overflow: "hidden",
  display: "block",
  boxShadow:
    "0 30px 80px rgba(0,0,0,0.55), 0 0 120px rgba(114,137,218,0.22), inset 0 1px 0 rgba(255,255,255,0.04)",
} as const;

export const Versions = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const drift = interpolate(frame, [0, durationInFrames], [0, -30], { ...clamp, easing: ease });

  const eyebrowT = interpolate(frame, [0, 18], [0, 1], { ...clamp, easing: ease });
  const line1T = interpolate(frame, [6, 26], [0, 1], { ...clamp, easing: ease });
  const line2T = interpolate(frame, [16, 36], [0, 1], { ...clamp, easing: ease });
  const sublineT = interpolate(frame, [38, 58], [0, 1], { ...clamp, easing: ease });

  const backT = interpolate(frame, [0, 40], [0, 1], { ...clamp, easing: ease });
  const frontT = interpolate(frame, [70, 100], [0, 1], { ...clamp, easing: ease });
  const frontSlide = (1 - frontT) * 260;

  const riseStyle = (p: number, distance: number) => ({
    opacity: p,
    transform: `translateY(${(1 - p) * distance}px)`,
  });

  return (
    <AbsoluteFill style={{ background: c.bg, overflow: "hidden" }}>
      {/* Layered glow behind the windows */}
      <div
        style={{
          position: "absolute",
          left: 760,
          top: 120,
          width: 1100,
          height: 860,
          borderRadius: "50%",
          background:
            "radial-gradient(circle at 50% 50%, rgba(114,137,218,0.25) 0%, rgba(114,137,218,0.12) 35%, rgba(114,137,218,0) 70%)",
          opacity: backT,
          filter: "blur(40px)",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: 1000,
          top: 320,
          width: 700,
          height: 620,
          borderRadius: "50%",
          background:
            "radial-gradient(circle at 50% 50%, rgba(114,137,218,0.25) 0%, rgba(114,137,218,0) 65%)",
          opacity: frontT,
        }}
      />

      {/* Text column */}
      <div
        style={{
          position: "absolute",
          left: 120,
          top: 0,
          bottom: 0,
          width: 640,
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
        }}
      >
        <div
          style={{
            fontFamily: body,
            fontWeight: 600,
            fontSize: 22,
            letterSpacing: "0.12em",
            textTransform: "uppercase",
            color: c.accentHover,
            opacity: eyebrowT,
            transform: `translateY(${(1 - eyebrowT) * 14}px)`,
            marginBottom: 28,
          }}
        >
          VERSIONS & MODS
        </div>

        <div
          style={{
            fontFamily: display,
            fontWeight: 700,
            fontSize: 104,
            letterSpacing: "-0.025em",
            lineHeight: 1.02,
            color: c.text,
          }}
        >
          <div style={riseStyle(line1T, 40)}>Every version.</div>
          <div style={riseStyle(line2T, 40)}>Every mod.</div>
        </div>

        <div
          style={{
            fontFamily: body,
            fontWeight: 400,
            fontSize: 34,
            lineHeight: 1.4,
            color: c.dim,
            maxWidth: 640,
            marginTop: 36,
            ...riseStyle(sublineT, 24),
          }}
        >
          Releases, snapshots, Fabric, Forge, Quilt and Modrinth — in a few clicks.
        </div>
      </div>

      {/* Back window: versions */}
      <div
        style={{
          position: "absolute",
          left: 860,
          top: 150,
          width: 1000,
          opacity: backT,
          transform: `translateX(${drift}px) translateY(${(1 - backT) * 30}px)`,
        }}
      >
        <Img
          src={staticFile("shots/versions.png")}
          style={{ ...windowStyle, width: 1000, height: "auto" }}
        />
      </div>

      {/* Front window: mods, slides in from the right */}
      <div
        style={{
          position: "absolute",
          left: 780,
          top: 500,
          width: 900,
          opacity: frontT,
          transform: `translateX(${drift + frontSlide}px)`,
        }}
      >
        <Img
          src={staticFile("shots/mods.png")}
          style={{ ...windowStyle, width: 900, height: "auto" }}
        />
      </div>
    </AbsoluteFill>
  );
};
