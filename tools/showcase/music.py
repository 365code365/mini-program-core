#!/usr/bin/env python3
"""给展示视频合成一段 60 秒的电子配乐（纯 numpy，不用任何素材）。

    python3 tools/showcase/music.py out.wav [秒数=60]

120 BPM、4/4，一小节 2 秒，和视频的分镜对齐（每个示例两小节）：
- 0~4s 前奏：铺底和弦 + 上扬噪声（riser），4s 处一记冲击（impact）进主歌；
- 4~52s 主段：底鼓四踩、2/4 拍拍手、八分踩镲、锯齿贝斯跟和弦根音、拨弦琶音 + 延迟；每 8 小节换一次加花；
- 52~56s 网格墙：鼓停一小节只留铺底与琶音，再一记 riser；
- 56~60s 尾声：大和弦 + 冲击，渐弱。
和弦走 Am – F – C – G（i–VI–III–VII），每小节一个。
"""
import sys
import wave

import numpy as np
from scipy.signal import lfilter

SR = 44100
BPM = 120
BEAT = 60 / BPM
BAR = BEAT * 4


def note_hz(n):
    """MIDI 音高 → 频率"""
    return 440.0 * 2 ** ((n - 69) / 12)


# Am F C G：根音（MIDI）与三和弦
CHORDS = [
    (57, [57, 60, 64]),  # A minor
    (53, [53, 57, 60]),  # F
    (48, [48, 52, 55]),  # C
    (55, [55, 59, 62]),  # G
]


def env_adsr(n, a=0.005, d=0.1, s=0.6, r=0.1):
    t = np.arange(n) / SR
    dur = n / SR
    e = np.ones(n) * s
    e[t < a] = t[t < a] / a
    m = (t >= a) & (t < a + d)
    e[m] = 1 - (1 - s) * (t[m] - a) / d
    rel = t > dur - r
    e[rel] *= np.clip((dur - t[rel]) / r, 0, 1)
    return e


def saw(f, n, detune=0.0):
    t = np.arange(n) / SR
    out = np.zeros(n)
    for d in (-detune, 0.0, detune):
        ph = (t * f * (1 + d)) % 1.0
        out += 2 * ph - 1
    return out / 3


def lowpass(x, cutoff):
    """一阶低通（逐样本，cutoff 可以是数组：滤波扫频）"""
    if np.isscalar(cutoff):
        a = 1 - np.exp(-2 * np.pi * cutoff / SR)
        return lfilter([a], [1, -(1 - a)], x)
    # 扫频：按 512 个样本一段、段内定 cutoff 分段滤，段与段之间接上状态
    cutoff = np.broadcast_to(cutoff, x.shape)
    y = np.empty_like(x)
    zi = np.zeros(1)
    for i in range(0, len(x), 512):
        a = 1 - np.exp(-2 * np.pi * float(cutoff[i]) / SR)
        y[i:i + 512], zi = lfilter([a], [1, -(1 - a)], x[i:i + 512], zi=zi)
    return y


def kick(n=int(0.35 * SR)):
    t = np.arange(n) / SR
    f = 50 + 120 * np.exp(-t * 35)
    ph = 2 * np.pi * np.cumsum(f) / SR
    return np.sin(ph) * np.exp(-t * 7) + 0.3 * np.sin(ph * 2) * np.exp(-t * 30)


def clap(n=int(0.25 * SR), rng=np.random.default_rng(1)):
    t = np.arange(n) / SR
    noise = rng.standard_normal(n)
    # 三次短促叠加，像手掌拍
    e = np.zeros(n)
    for off in (0.0, 0.012, 0.024):
        m = t >= off
        e[m] += np.exp(-(t[m] - off) * 60)
    tail = np.exp(-t * 18) * 0.5
    x = noise * (e + tail)
    return x - lowpass(x, 900)  # 去掉低频，留「啪」


def hat(n=int(0.06 * SR), open_=False, rng=np.random.default_rng(2)):
    if open_:
        n = int(0.22 * SR)
    t = np.arange(n) / SR
    x = rng.standard_normal(n)
    x = x - lowpass(x, 6000)
    return x * np.exp(-t * (18 if open_ else 80))


def riser(dur, rng=np.random.default_rng(3)):
    n = int(dur * SR)
    t = np.arange(n) / SR
    x = rng.standard_normal(n)
    x = lowpass(x, 300 + 9000 * (t / dur) ** 2)
    return x * (t / dur) ** 2


def impact(dur=1.6):
    n = int(dur * SR)
    t = np.arange(n) / SR
    boom = np.sin(2 * np.pi * np.cumsum(40 + 60 * np.exp(-t * 8)) / SR) * np.exp(-t * 2.5)
    noise = np.random.default_rng(4).standard_normal(n) * np.exp(-t * 6) * 0.4
    return boom + lowpass(noise, 2500)


def place(buf, clip, at, gain=1.0):
    i = int(at * SR)
    if i >= len(buf):
        return
    j = min(len(buf), i + len(clip))
    buf[i:j] += clip[: j - i] * gain


def render(total=60.0):
    n = int(total * SR)
    drums = np.zeros(n)
    bass = np.zeros(n)
    pad = np.zeros(n)
    arp = np.zeros(n)
    fx = np.zeros(n)
    bars = int(total / BAR)
    k, c = kick(), clap()
    hc, ho = hat(), hat(open_=True)
    for b in range(bars):
        t0 = b * BAR
        root, triad = CHORDS[b % 4]
        intro = b < 2
        breakdown = b == 26
        outro = b >= 28
        # 铺底：三和弦的锯齿 + 低通，整小节
        seg = int(BAR * SR)
        chord = sum(saw(note_hz(m + 12), seg, 0.006) for m in triad) / 3
        chord = lowpass(chord, 1400 if not intro else 500 + 900 * b) * env_adsr(seg, 0.3, 0.2, 0.9, 0.4)
        place(pad, chord, t0, 0.35)
        # 琶音：十六分音符，和弦音上下走，带一点力度变化
        if not intro:
            seq = triad + [triad[0] + 12, triad[1] + 12, triad[2] + 12, triad[0] + 12, triad[2]]
            step = BEAT / 4
            for s in range(16):
                m = seq[s % len(seq)] + 12
                nn = int(step * SR * 0.9)
                tt = np.arange(nn) / SR
                tone = (np.sign(np.sin(2 * np.pi * note_hz(m) * tt)) * 0.5 + np.sin(2 * np.pi * note_hz(m) * tt)) * np.exp(-tt * 14)
                place(arp, tone, t0 + s * step, 0.22 if s % 4 else 0.3)
        # 贝斯：八分音符根音，反拍跳八度
        if not intro and not breakdown:
            for e8 in range(8):
                m = root - 12 + (12 if e8 % 2 else 0)
                nn = int(BEAT / 2 * SR)
                tone = lowpass(saw(note_hz(m), nn, 0.003), 500) * env_adsr(nn, 0.003, 0.08, 0.5, 0.05)
                place(bass, tone, t0 + e8 * BEAT / 2, 0.9)
        # 鼓
        if not intro and not breakdown:
            for q in range(4):
                place(drums, k, t0 + q * BEAT, 1.0)
                if q in (1, 3):
                    place(drums, c, t0 + q * BEAT, 0.55)
            for e8 in range(8):
                place(drums, ho if e8 % 2 else hc, t0 + e8 * BEAT / 2, 0.18 if e8 % 2 else 0.12)
            # 每 8 小节最后一拍加花：十六分底鼓 + 拍手
            if b % 8 == 7:
                for s in range(4):
                    place(drums, k, t0 + 3 * BEAT + s * BEAT / 4, 0.6)
                    place(drums, c, t0 + 3 * BEAT + s * BEAT / 4, 0.25)
    # 效果：开头 riser → 4s 冲击；网格墙前后；尾声冲击
    place(fx, riser(3.6), 0.4, 0.5)
    place(fx, impact(), 4.0, 0.9)
    place(fx, riser(1.9), 52.0 + 0.1, 0.45)
    place(fx, impact(), 56.0, 1.0)
    # 侧链：底鼓一响，铺底与琶音让一让（泵感）
    duck = np.ones(n)
    for b in range(2, bars):
        if b == 26:
            continue
        for q in range(4):
            i = int((b * BAR + q * BEAT) * SR)
            m = int(0.25 * SR)
            j = min(n, i + m)
            duck[i:j] = np.minimum(duck[i:j], 0.35 + 0.65 * (np.arange(j - i) / m))
    mix = drums * 0.8 + bass * 0.55 + (pad + arp) * duck + fx
    # 尾声渐弱
    t = np.arange(n) / SR
    mix *= np.clip((total - t) / 3.0, 0, 1)
    # 简单立体声：琶音与镲左右错开一点
    left = mix + 0.12 * np.roll(arp * duck, int(0.011 * SR))
    right = mix + 0.12 * np.roll(arp * duck, int(0.017 * SR))
    st = np.stack([left, right], axis=1)
    st /= np.max(np.abs(st)) + 1e-9
    st = np.tanh(st * 1.4) / np.tanh(1.4)  # 软限幅
    return (st * 0.92 * 32767).astype(np.int16)


def main():
    out = sys.argv[1]
    total = float(sys.argv[2]) if len(sys.argv) > 2 else 60.0
    pcm = render(total)
    with wave.open(out, "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())
    print(f"{out}：{total:.0f}s {SR}Hz 立体声")


if __name__ == "__main__":
    main()
