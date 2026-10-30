// v2 palette matches the Glacier website. Tokens from the Glacier Productions design system (tokens.json v2).
import { loadFont as loadRoboto } from "@remotion/google-fonts/Roboto";
import { loadFont as loadJakarta } from "@remotion/google-fonts/PlusJakartaSans";
import { Easing } from "remotion";

export const display = loadRoboto("normal", { weights: ["700", "900"] }).fontFamily;
export const body = loadJakarta("normal", { weights: ["400", "600", "800"] }).fontFamily;

export const c = {
  accent: "#7289da",
  accentHover: "#8ea0e0",
  accentGlow: "rgba(114, 137, 218, 0.32)",
  accentBg: "rgba(114, 137, 218, 0.10)",
  bg: "#0a0b14",
  panel: "#121526",
  inset: "#0c0d18",
  item: "rgba(255, 255, 255, 0.03)",
  border: "rgba(255, 255, 255, 0.10)",
  text: "#eaecf6",
  dim: "#9aa1b8",
  green: "#43b581",
  orange: "#faa61a",
};

export const r = { sm: 12, md: 18, lg: 28, pill: 999 };

// The one motion curve the design system uses everywhere.
export const ease = Easing.bezier(0.32, 0.72, 0, 1);

export const FPS = 30;
export const SCENE = 150; // 5s per scene
