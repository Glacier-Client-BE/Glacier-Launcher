"""Generates public/music.wav: a 96 BPM chill-electronic bed (pads, sub bass,
plucked arp, soft drums). Usage: python scripts/music.py [seconds]"""
import sys, wave
import numpy as np
from scipy.signal import butter, sosfilt

sr = 44100; bpm = 96; beat = 60 / bpm; bar = 4 * beat
T = float(sys.argv[1]) if len(sys.argv) > 1 else 64.0
N = int(sr * T); t = np.arange(N) / sr
hz = lambda m: 440 * 2 ** ((m - 69) / 12)
lp = lambda x, f: sosfilt(butter(2, f, 'low', fs=sr, output='sos'), x)
hp = lambda x, f: sosfilt(butter(2, f, 'high', fs=sr, output='sos'), x)
rng = np.random.default_rng(1)
prog = [[53, 57, 60, 64], [57, 60, 64, 67], [48, 55, 59, 64], [55, 59, 62, 64]]
seg = 2 * bar; start_drums = 4 * bar; nb = int(T / beat)

L = np.zeros(N); R = np.zeros(N)
pump = np.ones(N)
for b in range(nb):
    s = b * beat
    if s < start_drums: continue
    i = (t >= s) & (t < s + beat); pump[i] = 0.45 + 0.55 * (1 - np.exp(-(t[i] - s) * 9))
for k in range(int(T / seg) + 1):
    ch = prog[k % 4]; s = k * seg; e = s + seg
    env = np.clip((t - s) / 0.6, 0, 1) * np.clip((e + 0.8 - t) / 0.8, 0, 1); m = env > 0
    for n in ch:
        for det, pan in ((-0.08, 0.2), (0.08, 0.8), (0, 0.5)):
            v = (2 * ((t[m] * hz(n + det)) % 1) - 1) * env[m] * 0.035
            L[m] += v * (1 - pan); R[m] += v * pan
L = lp(L, 1400) * pump; R = lp(R, 1400) * pump
bass = np.zeros(N)
for b in range(int(T / (beat / 2))):
    s = b * beat / 2
    if s < 2 * bar: continue
    root = prog[int(s / seg) % 4][0] - 12
    i = (t >= s) & (t < s + beat / 2 * 0.9); tt = t[i] - s
    bass[i] += np.sin(2 * np.pi * hz(root) * tt) * np.exp(-tt * 3) * 0.22
bass = lp(bass, 300); L += bass; R += bass
arp = np.zeros(N); pat = [0, 2, 1, 3, 2, 1, 3, 2]
for b in range(int(T / (beat / 4))):
    s = b * beat / 4
    if s < bar * 2 or s > T - bar: continue
    f = hz(prog[int(s / seg) % 4][pat[b % 8]] + 12)
    i = (t >= s) & (t < s + 0.4); tt = t[i] - s
    arp[i] += (np.sin(2 * np.pi * f * tt) + 0.3 * np.sin(4 * np.pi * f * tt)) * np.exp(-tt * 9) * 0.05
d = int(beat * 0.75 * sr)
aL = arp.copy(); aR = np.zeros(N); aR[d:] += 0.5 * arp[:-d]; aL[2 * d:] += 0.3 * arp[:-2 * d]
L += aL; R += aR
kick = np.zeros(N); hat = np.zeros(N); clap = np.zeros(N)
for b in range(nb):
    s = b * beat
    if s < start_drums or s > T - bar: continue
    i = (t >= s) & (t < s + 0.35); tt = t[i] - s
    kick[i] += np.sin(2 * np.pi * (45 + 90 * np.exp(-tt * 30)) * tt) * np.exp(-tt * 8) * 0.5
    s2 = s + beat / 2; i = (t >= s2) & (t < s2 + 0.06)
    hat[i] += rng.standard_normal(i.sum()) * np.exp(-(t[i] - s2) * 70) * 0.06
    if b % 2 == 1 and s >= 8 * bar:
        i = (t >= s) & (t < s + 0.25)
        clap[i] += rng.standard_normal(i.sum()) * np.exp(-(t[i] - s) * 18) * 0.12
hat = hp(hat, 7000); clap = lp(hp(clap, 900), 5000)
L += kick + hat * 0.8 + clap; R += kick + hat * 1.1 + clap
for x in (L, R):
    for dl, g in ((0.031, 0.25), (0.047, 0.2), (0.071, 0.15), (0.113, 0.1)):
        k = int(dl * sr); x[k:] += g * lp(x[:-k], 3000)
fade = np.clip(t / 1.5, 0, 1) * np.clip((T - t) / 3.5, 0, 1)
st = np.stack([L * fade, R * fade], 1)
st = np.tanh(st / np.abs(st).max() * 1.6) / np.tanh(1.6) * 0.85
w = wave.open('public/music.wav', 'wb'); w.setnchannels(2); w.setsampwidth(2); w.setframerate(sr)
w.writeframes((st * 32767).astype('<i2').tobytes()); w.close()
