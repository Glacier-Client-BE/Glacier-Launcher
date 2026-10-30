import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { body, c, display, ease, r } from "../theme";

const FADE_OUT_FRAMES = 25;

const HEADLINE_WORDS = "Glacier Launcher v2".split(" ");
const WORD_STAGGER = 4;
const WORD_DURATION = 25;
const WORD_RISE = 24;

const fadeRise = (frame: number, start: number, end: number) => {
  const t = interpolate(frame, [start, end], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: ease,
  });
  return { opacity: t, transform: `translateY(${(1 - t) * 30}px)` };
};

const fade = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, end], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: ease,
  });

export const Outro = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const last = durationInFrames - 1;

  const logoOpacity = fade(frame, 0, 20);
  const subline = fadeRise(frame, 30, 55);
  const button = fadeRise(frame, 45, 70);
  const caption = fade(frame, 65, 80);

  // Whole-scene drift, matching the intro.
  const driftP = interpolate(frame, [0, last], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: ease,
  });
  const driftScale = 1 + 0.08 * driftP;
  const driftOpacity = 0.7 + 0.3 * driftP;

  // Final frames fade the whole composition to the background color.
  const outroFade = interpolate(frame, [last - FADE_OUT_FRAMES + 1, last], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: ease,
  });

  return (
    <AbsoluteFill style={{ backgroundColor: c.bg, overflow: "hidden" }}>
      {/* Layered glow: main accent, cool counter-tint, faint green offset. */}
      <AbsoluteFill style={{ transform: `scale(${driftScale})`, opacity: driftOpacity, pointerEvents: "none" }}>
        <div
          style={{
            position: "absolute",
            left: "50%",
            top: "50%",
            width: 1600,
            height: 1600,
            transform: "translate(-50%, -50%)",
            background: "radial-gradient(circle, rgba(114, 137, 218, 0.24) 0%, rgba(114, 137, 218, 0) 65%)",
          }}
        />
        <div
          style={{
            position: "absolute",
            left: "38%",
            top: "45%",
            width: 1400,
            height: 1400,
            transform: "translate(-50%, -50%)",
            background: "radial-gradient(circle, rgba(114, 137, 218, 0.18) 0%, rgba(114, 137, 218, 0) 70%)",
          }}
        />
        <div
          style={{
            position: "absolute",
            left: "66%",
            top: "60%",
            width: 1400,
            height: 1400,
            transform: "translate(-50%, -50%)",
            background: "radial-gradient(circle, rgba(30, 36, 70, 0.55) 0%, rgba(30, 36, 70, 0) 70%)",
          }}
        />
      </AbsoluteFill>

      <AbsoluteFill
        style={{
          alignItems: "center",
          justifyContent: "center",
          flexDirection: "column",
          textAlign: "center",
        }}
      >
        <Img
          src={staticFile("logo.png")}
          style={{ width: 120, height: 120, opacity: logoOpacity, marginBottom: 40 }}
        />

        <div
          style={{
            display: "flex",
            gap: "0.25em",
            fontFamily: display,
            fontWeight: 700,
            fontSize: 120,
            letterSpacing: "-0.025em",
            lineHeight: 1.05,
            color: c.text,
            whiteSpace: "nowrap",
          }}
        >
          {HEADLINE_WORDS.map((word, i) => {
            const start = 10 + i * WORD_STAGGER;
            const p = interpolate(frame, [start, start + WORD_DURATION], [0, 1], {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
              easing: ease,
            });
            return (
              <span
                key={`${word}-${i}`}
                style={{
                  display: "inline-block",
                  opacity: p,
                  transform: `translateY(${(1 - p) * WORD_RISE}px)`,
                }}
              >
                {word}
              </span>
            );
          })}
        </div>

        <div
          style={{
            fontFamily: body,
            fontWeight: 400,
            fontSize: 40,
            color: c.dim,
            marginTop: 20,
            opacity: subline.opacity,
            transform: subline.transform,
          }}
        >
          Free. Open. Out now.
        </div>

        <div
          style={{
            marginTop: 56,
            display: "inline-block",
            backgroundColor: c.accent,
            color: c.bg,
            fontFamily: body,
            fontWeight: 800,
            fontSize: 36,
            padding: "22px 48px",
            borderRadius: r.sm,
            boxShadow: `0 0 40px ${c.accentGlow}`,
            opacity: button.opacity,
            transform: button.transform,
            whiteSpace: "nowrap",
          }}
        >
          Download now &nbsp;→
        </div>

        <div
          style={{
            marginTop: 28,
            fontFamily: body,
            fontWeight: 400,
            fontSize: 24,
            color: c.dim,
            opacity: caption,
          }}
        >
          Link in the description
        </div>
      </AbsoluteFill>

      {/* Final frames fade the whole composition to the background color. */}
      <AbsoluteFill style={{ backgroundColor: c.bg, opacity: outroFade, pointerEvents: "none" }} />
    </AbsoluteFill>
  );
};
