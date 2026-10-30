import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { body, c, display, ease, r } from "../theme";

const clamp = { extrapolateLeft: "clamp", extrapolateRight: "clamp" } as const;

// Bedrock holds the first beat; Java cross-slides in at this frame.
const SWITCH = 150;
const CROSS = 35;

const windowStyle = {
  borderRadius: r.md,
  border: "1px solid rgba(255,255,255,0.10)",
  overflow: "hidden",
  display: "block",
  boxShadow:
    "0 30px 80px rgba(0,0,0,0.55), 0 0 140px rgba(114,137,218,0.30), inset 0 1px 0 rgba(255,255,255,0.04)",
} as const;

type Chip = { label: string; logo: string; small?: boolean };

const bedrockChips: Chip[] = [
  { label: "Flarial", logo: "clients/flarial.svg" },
  { label: "Latite", logo: "clients/latite.png" },
];

const javaChips: Chip[] = [
  { label: "Essential", logo: "clients/essential.webp" },
  { label: "Dawn", logo: "clients/dawn.png" },
  { label: "NoRisk", logo: "clients/norisk.png" },
  { label: "Horizon", logo: "clients/horizon.svg", small: true },
];

const ClientChip = ({ chip, p }: { chip: Chip; p: number }) => (
  <div
    style={{
      display: "flex",
      alignItems: "center",
      gap: chip.small ? 16 : 20,
      width: 420,
      boxSizing: "border-box",
      padding: chip.small ? "12px 24px" : "18px 28px",
      borderRadius: r.pill,
      background: c.panel,
      border: `1px solid ${c.border}`,
      boxShadow: "inset 0 1px 0 rgba(255,255,255,0.04)",
      opacity: p * (chip.small ? 0.7 : 1),
      transform: `translateY(${(1 - p) * 20}px)`,
    }}
  >
    <Img
      src={staticFile(chip.logo)}
      style={{ width: chip.small ? 28 : 36, height: chip.small ? 28 : 36, objectFit: "contain", flex: "none" }}
    />
    <span
      style={{
        fontFamily: body,
        fontWeight: 600,
        fontSize: chip.small ? 26 : 30,
        color: c.text,
        whiteSpace: "nowrap",
      }}
    >
      {chip.label}
    </span>
  </div>
);

type BeatProps = {
  enterAt: number;
  exitAt?: number;
  shot: string;
  label: string;
  labelLogo: string;
  chips: Chip[];
};

// One edition beat: zoomed screenshot window on the left, glass chips on the right.
const Beat = ({ enterAt, exitAt, shot, label, labelLogo, chips }: BeatProps) => {
  const frame = useCurrentFrame();

  const inT = interpolate(frame, [enterAt, enterAt + CROSS], [0, 1], { ...clamp, easing: ease });
  const outT =
    exitAt === undefined ? 1 : interpolate(frame, [exitAt, exitAt + CROSS], [1, 0], { ...clamp, easing: ease });
  const vis = inT * outT;
  if (vis <= 0) return null;

  // Enters from the right, leaves to the left.
  const shift = (1 - inT) * 60 - (1 - outT) * 60;

  return (
    <>
      <div
        style={{
          position: "absolute",
          left: 120,
          top: 290,
          width: 1150,
          height: 650,
          ...windowStyle,
          opacity: vis,
          transform: `translateX(${shift}px)`,
        }}
      >
        <Img
          src={staticFile(shot)}
          style={{
            width: 1150,
            height: 650,
            display: "block",
            objectFit: "cover",
            transform: "scale(1.25)",
            transformOrigin: "85% 5%",
          }}
        />
      </div>

      <div
        style={{
          position: "absolute",
          left: 1340,
          top: 330,
          width: 420,
          display: "flex",
          flexDirection: "column",
          gap: 22,
          opacity: vis,
          transform: `translateX(${shift * 0.6}px)`,
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 14,
            marginBottom: 8,
            fontFamily: body,
            fontWeight: 600,
            fontSize: 28,
            letterSpacing: "0.02em",
            color: c.accentHover,
          }}
        >
          <Img src={staticFile(labelLogo)} style={{ width: 28, height: 28, objectFit: "contain" }} />
          {label}
        </div>

        {chips.map((chip, i) => {
          const start = enterAt + 30 + i * 12;
          const p = interpolate(frame, [start, start + 18], [0, 1], { ...clamp, easing: ease });
          return <ClientChip key={chip.label} chip={chip} p={p} />;
        })}
      </div>
    </>
  );
};

export const Clients = () => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();

  const drift = interpolate(frame, [0, durationInFrames], [-20, 20], { ...clamp, easing: ease });
  const eyebrowT = interpolate(frame, [0, 18], [0, 1], { ...clamp, easing: ease });
  const headlineWords = ["Pick", "your", "client."];

  return (
    <AbsoluteFill style={{ background: c.bg, overflow: "hidden" }}>
      {/* Layered accent glow behind the windows */}
      <div
        style={{
          position: "absolute",
          left: 200 + drift,
          top: 270,
          width: 1000,
          height: 700,
          borderRadius: "50%",
          background:
            "radial-gradient(circle at 50% 50%, rgba(114,137,218,0.28) 0%, rgba(114,137,218,0.12) 40%, rgba(114,137,218,0) 70%)",
          filter: "blur(30px)",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: 1280,
          top: 300,
          width: 560,
          height: 620,
          borderRadius: "50%",
          background:
            "radial-gradient(circle at 50% 50%, rgba(114,137,218,0.16) 0%, rgba(114,137,218,0) 65%)",
          transform: `translateX(${-drift}px)`,
        }}
      />

      {/* Eyebrow */}
      <div
        style={{
          position: "absolute",
          top: 56,
          left: 0,
          right: 0,
          textAlign: "center",
          fontFamily: body,
          fontWeight: 600,
          fontSize: 22,
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          color: c.accentHover,
          opacity: eyebrowT,
          transform: `translateY(${(1 - eyebrowT) * 14}px)`,
        }}
      >
        BEDROCK + JAVA
      </div>

      {/* Headline, per-word rise */}
      <div
        style={{
          position: "absolute",
          top: 96,
          left: 0,
          right: 0,
          textAlign: "center",
          fontFamily: display,
          fontWeight: 700,
          fontSize: 100,
          letterSpacing: "-0.025em",
          lineHeight: 1.05,
          color: c.text,
        }}
      >
        {headlineWords.map((w, i) => {
          const p = interpolate(frame, [4 + i * 6, 22 + i * 6], [0, 1], { ...clamp, easing: ease });
          return (
            <span
              key={w}
              style={{
                display: "inline-block",
                marginRight: "0.28em",
                opacity: p,
                transform: `translateY(${(1 - p) * 40}px)`,
              }}
            >
              {w}
            </span>
          );
        })}
      </div>

      <Beat
        enterAt={0}
        exitAt={SWITCH}
        shot="shots/bedrock-clients.png"
        label="Bedrock"
        labelLogo="mc-logo.png"
        chips={bedrockChips}
      />
      <Beat
        enterAt={SWITCH}
        shot="shots/java-clients.png"
        label="Java"
        labelLogo="mc-java-logo.png"
        chips={javaChips}
      />
    </AbsoluteFill>
  );
};
