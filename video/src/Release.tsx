import { AbsoluteFill, Audio, Sequence, interpolate, staticFile, useCurrentFrame } from "remotion";
import { TransitionSeries, linearTiming } from "@remotion/transitions";
import { fade } from "@remotion/transitions/fade";
import { slide } from "@remotion/transitions/slide";
import { c, ease } from "./theme";
import { Intro } from "./scenes/Intro";
import { Speed } from "./scenes/Speed";
import { Editions } from "./scenes/Editions";
import { Clients } from "./scenes/Clients";
import { Versions } from "./scenes/Versions";
import { Tools } from "./scenes/Tools";
import { Downloads } from "./scenes/Downloads";
import { Outro } from "./scenes/Outro";

// Each scene is sized to its voice-over line (public/vo/*.mp3) plus breathing room.
const scenes = [
  { C: Intro, d: 180, vo: "vo0" },
  { C: Speed, d: 300, vo: "vo1" },
  { C: Editions, d: 255, vo: "vo2" },
  { C: Clients, d: 345, vo: "vo-clients" },
  { C: Versions, d: 240, vo: "vo3" },
  { C: Tools, d: 225, vo: "vo4" },
  { C: Downloads, d: 210, vo: "vo5" },
  { C: Outro, d: 240, vo: "vo6" },
];
const T = 20; // transition overlap
export const TOTAL = scenes.reduce((s, x) => s + x.d, 0) - T * (scenes.length - 1);
const starts = scenes.map((_, i) => scenes.slice(0, i).reduce((s, x) => s + x.d - T, 0));

const timing = linearTiming({ durationInFrames: T, easing: ease });

const Music = () => {
  const f = useCurrentFrame();
  const vol = interpolate(f, [0, 30, TOTAL - 60, TOTAL], [0, 0.35, 0.35, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  return <Audio src={staticFile("music.wav")} volume={vol} />;
};

export const Release = () => (
  <AbsoluteFill style={{ background: c.bg }}>
    <TransitionSeries>
      {scenes.map(({ C, d }, i) => [
        i > 0 ? (
          <TransitionSeries.Transition
            key={`t${i}`}
            timing={timing}
            presentation={i % 2 ? fade() : slide({ direction: "from-right" })}
          />
        ) : null,
        <TransitionSeries.Sequence key={`s${i}`} durationInFrames={d}>
          <C />
        </TransitionSeries.Sequence>,
      ])}
    </TransitionSeries>
    <Music />
    {starts.map((s, i) => (
      <Sequence key={`vo${i}`} from={s + 15}>
        <Audio src={staticFile(`vo/${scenes[i].vo}.mp3`)} volume={1} />
      </Sequence>
    ))}
  </AbsoluteFill>
);
