import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { body, c, display, ease, r } from "../theme";

const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

export const Downloads = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const slideT = interpolate(frame, [0, 40], [0, 1], { ...clamp, easing: ease });
  const shotScale = interpolate(frame, [0, durationInFrames], [1, 1.03], clamp);

  const eyebrowT = interpolate(frame, [0, 18], [0, 1], { ...clamp, easing: ease });
  const headlineT = interpolate(frame, [6, 26], [0, 1], { ...clamp, easing: ease });
  const subT = interpolate(frame, [18, 38], [0, 1], { ...clamp, easing: ease });

  const barT = interpolate(frame, [40, 170], [0, 1], { ...clamp, easing: ease });
  const done = frame >= 170;
  const readyT = interpolate(frame, [170, 184], [0, 1], { ...clamp, easing: ease });

  return (
    <AbsoluteFill style={{ background: c.bg }}>
      {/* Screenshot with glow, slides in from the left */}
      <div
        style={{
          position: "absolute",
          top: 240,
          left: 70,
          width: 1100,
          opacity: slideT,
          transform: `translateX(${(slideT - 1) * 200}px) perspective(2000px) rotateY(12deg) scale(${shotScale})`,
          transformOrigin: "center center",
        }}
      >
        <div
          style={{
            position: "absolute",
            top: -160,
            left: -160,
            right: -160,
            bottom: -160,
            background:
              "radial-gradient(ellipse at center, rgba(114,137,218,0.25) 0%, rgba(114,137,218,0.10) 40%, rgba(114,137,218,0) 70%)",
            pointerEvents: "none",
          }}
        />
        <div
          style={{
            position: "relative",
            borderRadius: r.md,
            border: "1px solid rgba(255,255,255,0.10)",
            boxShadow:
              "0 30px 80px rgba(0,0,0,0.55), 0 0 120px rgba(114,137,218,0.22), inset 0 1px 0 rgba(255,255,255,0.04)",
            overflow: "hidden",
          }}
        >
          <Img src={staticFile("shots/launchers.png")} style={{ width: 1100, height: "auto", display: "block" }} />
        </div>
      </div>

      {/* Progress card, bottom-left over the screenshot */}
      <div
        style={{
          position: "absolute",
          left: 110,
          bottom: 120,
          width: 560,
          padding: 28,
          borderRadius: r.md,
          background: c.panel,
          border: "1px solid rgba(255,255,255,0.10)",
          boxShadow: "0 24px 60px rgba(0,0,0,0.5), inset 0 1px 0 rgba(255,255,255,0.04)",
          opacity: slideT,
        }}
      >
        <div
          style={{
            fontFamily: body,
            fontWeight: 600,
            fontSize: 28,
            color: c.text,
            marginBottom: 20,
            position: "relative",
            height: 36,
          }}
        >
          <span style={{ position: "absolute", left: 0, top: 0, opacity: 1 - readyT }}>
            Downloading Minecraft 26.3
          </span>
          <span style={{ position: "absolute", left: 0, top: 0, opacity: readyT, color: c.green, whiteSpace: "nowrap" }}>
            Minecraft 26.3 installed
          </span>
          <span
            style={{
              position: "absolute",
              right: 0,
              top: 0,
              padding: "4px 18px",
              borderRadius: r.pill,
              background: c.accent,
              color: c.bg,
              fontWeight: 800,
              fontSize: 22,
              lineHeight: "28px",
              opacity: readyT,
              transform: `scale(${0.8 + 0.2 * readyT})`,
              transformOrigin: "right center",
            }}
          >
            Play
          </span>
        </div>
        <div
          style={{
            position: "relative",
            width: "100%",
            height: 10,
            borderRadius: r.pill,
            background: "rgba(255,255,255,0.08)",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              position: "absolute",
              inset: 0,
              borderRadius: r.pill,
              background: done ? c.green : c.accent,
              transform: `scaleX(${barT})`,
              transformOrigin: "left center",
            }}
          />
        </div>
      </div>

      {/* Right-aligned text column */}
      <div
        style={{
          position: "absolute",
          top: 300,
          right: 120,
          width: 640,
          textAlign: "right",
        }}
      >
        <div
          style={{
            fontFamily: body,
            fontWeight: 800,
            fontSize: 28,
            letterSpacing: "0.18em",
            color: c.accentHover,
            opacity: eyebrowT,
            transform: `translateY(${(1 - eyebrowT) * 20}px)`,
            marginBottom: 16,
          }}
        >
          UNDER THE HOOD
        </div>
        <div
          style={{
            fontFamily: display,
            fontWeight: 700,
            fontSize: 110,
            letterSpacing: "-0.03em",
            lineHeight: 1.0,
            color: c.text,
            opacity: headlineT,
            transform: `translateY(${(1 - headlineT) * 30}px)`,
          }}
        >
          Fast by default.
        </div>
        <div
          style={{
            marginTop: 28,
            marginLeft: "auto",
            maxWidth: 640,
            fontFamily: body,
            fontWeight: 400,
            fontSize: 34,
            lineHeight: 1.4,
            color: c.dim,
            opacity: subT,
            transform: `translateY(${(1 - subT) * 20}px)`,
          }}
        >
          Parallel downloads. Reuses files from launchers you already have.
        </div>
      </div>
    </AbsoluteFill>
  );
};
