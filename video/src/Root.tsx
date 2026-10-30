import { Composition } from "remotion";
import { Release, TOTAL } from "./Release";
import { FPS } from "./theme";

export const Root = () => (
  <Composition id="Release" component={Release} durationInFrames={TOTAL} fps={FPS} width={1920} height={1080} />
);
