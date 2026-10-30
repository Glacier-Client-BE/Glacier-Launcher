import React from "react";
import { AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { body, c, display, ease, r } from "../theme";

const clamp = {
	extrapolateLeft: "clamp",
	extrapolateRight: "clamp",
} as const;

// Final vertical position of the logo block after it moves up for the title beat.
const LOGO_SHIFT = -260;

// Headline words rise and fade in one after another.
const TITLE_WORDS = "Glacier Launcher".split(" ");
const WORD_START = 50;
const WORD_STAGGER = 4;
const WORD_DURATION = 25;
const WORD_RISE = 24;

export const Intro = () => {
	const frame = useCurrentFrame();
	const { durationInFrames } = useVideoConfig();
	const last = durationInFrames - 1;

	// Beat 1: logo fades and scales in, eyebrow follows.
	const logoP = interpolate(frame, [0, 30], [0, 1], { ...clamp, easing: ease });
	const logoScale = interpolate(frame, [0, 30], [0.9, 1], { ...clamp, easing: ease });
	const eyebrowP = interpolate(frame, [20, 45], [0, 1], { ...clamp, easing: ease });

	// Beat 2: logo block moves up to make room for the title.
	const shiftP = interpolate(frame, [50, 85], [0, 1], { ...clamp, easing: ease });
	const logoY = LOGO_SHIFT * shiftP;

	// Thin light line under the headline grows out from the centre.
	const lineP = interpolate(frame, [85, 120], [0, 1], { ...clamp, easing: ease });

	// Version badge follows the title.
	const badgeP = interpolate(frame, [110, 135], [0, 1], { ...clamp, easing: ease });
	const badgeY = (1 - badgeP) * 16;

	// Whole-scene drift across the scene length.
	const driftP = interpolate(frame, [0, last], [0, 1], { ...clamp, easing: ease });
	const driftScale = 1 + 0.08 * driftP;
	const driftOpacity = 0.7 + 0.3 * driftP;

	// Faint background screenshot settles slowly.
	const bgScale = interpolate(frame, [0, last], [1.1, 1.0], { ...clamp, easing: ease });

	return (
		<AbsoluteFill style={{ backgroundColor: c.bg, overflow: "hidden" }}>
			{/* Faint product screenshot behind everything. */}
			<Img
				src={staticFile("shots/home.png")}
				style={{
					position: "absolute",
					inset: 0,
					width: "100%",
					height: "100%",
					objectFit: "cover",
					opacity: 0.12,
					transform: `scale(${bgScale})`,
					transformOrigin: "center",
				}}
			/>

			{/* Dark vignette over the screenshot. */}
			<AbsoluteFill
				style={{
					background: `radial-gradient(ellipse at center, rgba(10, 11, 20, 0) 35%, ${c.bg} 100%)`,
					pointerEvents: "none",
				}}
			/>

			{/* Layered glow: main accent, cool counter-tint, faint green offset. */}
			<AbsoluteFill style={{ transform: `scale(${driftScale})`, opacity: driftOpacity, pointerEvents: "none" }}>
				<div
					style={{
						position: "absolute",
						left: "50%",
						top: 320,
						width: 1600,
						height: 1600,
						transform: "translate(-50%, -50%)",
						background: "radial-gradient(circle, rgba(114, 137, 218, 0.26) 0%, rgba(114, 137, 218, 0) 65%)",
					}}
				/>
				<div
					style={{
						position: "absolute",
						left: "38%",
						top: 420,
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
						top: 760,
						width: 1400,
						height: 1400,
						transform: "translate(-50%, -50%)",
						background: "radial-gradient(circle, rgba(67, 181, 129, 0.08) 0%, rgba(67, 181, 129, 0) 70%)",
					}}
				/>
			</AbsoluteFill>

			<AbsoluteFill>
				{/* Logo and eyebrow: centered at start, then shifted up. */}
				<div
					style={{
						position: "absolute",
						left: 0,
						right: 0,
						top: 0,
						bottom: 0,
						display: "flex",
						flexDirection: "column",
						alignItems: "center",
						justifyContent: "center",
						gap: 28,
						transform: `translateY(${logoY}px)`,
					}}
				>
					<div style={{ opacity: logoP, transform: `scale(${logoScale})` }}>
						<Img src={staticFile("logo.png")} style={{ height: 200, width: "auto", display: "block" }} />
					</div>
					<div
						style={{
							opacity: eyebrowP,
							fontFamily: body,
							fontSize: 22,
							fontWeight: 600,
							letterSpacing: "0.12em",
							textTransform: "uppercase",
							color: c.accentHover,
						}}
					>
						GLACIER PRODUCTIONS
					</div>
				</div>

				{/* Title, rule and badge sit below the shifted logo block. */}
				<div
					style={{
						position: "absolute",
						left: 0,
						right: 0,
						top: 560,
						display: "flex",
						flexDirection: "column",
						alignItems: "center",
						gap: 32,
					}}
				>
					<div
						style={{
							display: "flex",
							gap: "0.28em",
							fontFamily: display,
							fontWeight: 700,
							fontSize: 140,
							lineHeight: 1,
							letterSpacing: "-0.025em",
							color: c.text,
							whiteSpace: "nowrap",
						}}
					>
						{TITLE_WORDS.map((word, i) => {
							const start = WORD_START + i * WORD_STAGGER;
							const p = interpolate(frame, [start, start + WORD_DURATION], [0, 1], {
								...clamp,
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
							width: 900,
							height: 1,
							background: "linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.25), transparent)",
							transform: `scaleX(${lineP})`,
							transformOrigin: "center",
						}}
					/>
					<div
						style={{
							opacity: badgeP,
							transform: `translateY(${badgeY}px)`,
							backgroundColor: c.accent,
							color: c.text,
							border: `1px solid ${c.border}`,
							boxShadow: "inset 0 1px 0 rgba(255, 255, 255, 0.04)",
							borderRadius: r.pill,
							padding: "14px 40px",
							fontFamily: body,
							fontSize: 40,
							fontWeight: 800,
						}}
					>
						v2.0.0
					</div>
				</div>
			</AbsoluteFill>
		</AbsoluteFill>
	);
};
