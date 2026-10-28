#!/usr/bin/env python3
"""给《拳魂》（sample-game/play-fighter）离线合成音效与解说，写成 sample-game/play-fighter/sfx/*.mp3。
    python3 tools/gen-fighter-sfx.py            # 全部重做
    python3 tools/gen-fighter-sfx.py hitH ko     # 只重做这几条
    python3 tools/gen-fighter-sfx.py --wav       # 另存一份 wav 到 target/fighter-sfx/（试听 / 看波形用）

为什么不在游戏里实时合成：几十个振荡器 + 滤波 + 卷积混响每次实时搭一遍，真机一次就是几毫秒（base.js 里记过：
建一个 Oscillator 就 ~25ms），而且 WebAudio 节点图做不出「多层叠加 + 失真 + 混响尾巴」这种厚度。这里用 numpy 一次算好，
游戏里只 decodeAudioData 一次、之后每次一个 BufferSource（~0.6ms）。

一个打击声 = 几层叠起来（格斗游戏音效的通行做法）：
    瞬态（click，1~3ms 的宽频脉冲：「啪」的那一下）+ 身体（低频正弦往下滑：「嘭」）+ 质感（带通噪声：衣服、皮肉）
    → 软削波失真（tanh，让它「脏」、听着有劲）→ 短混响（合成的房间脉冲响应，卷积）→ 归一到 -1dBFS。
解说用 macOS 的 say（英文男声 Daniel），降调 15%、压缩、过一点失真和大混响：街机格斗的「ROUND ONE / FIGHT / K.O.」腔调。
没有 say 的机器跳过解说（游戏里缺哪条就不放那条）。
依赖：numpy、scipy、ffmpeg（libmp3lame）。
"""
import os
import subprocess
import sys
import tempfile

import numpy as np
from scipy import signal

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'sample-game', 'play-fighter', 'sfx')
WAVDIR = os.path.join(ROOT, 'target', 'fighter-sfx')
SR = 44100
RNG = np.random.default_rng(7)          # 固定种子：重跑出同样的文件（不会每次都改动包里的二进制）


# ───────── 基本积木 ─────────
def t_(dur):
    return np.arange(int(SR * dur)) / SR


def noise(dur):
    return RNG.standard_normal(int(SR * dur))


def env(dur, a=0.002, d=0.2, curve=4.0):
    """起音 a 秒线性上去，之后按指数衰减，dur 内衰到 ~e^-curve"""
    t = t_(dur)
    e = np.exp(-curve * np.maximum(t - a, 0) / max(d, 1e-4))
    e[t < a] = t[t < a] / a
    return e


def sweep(f0, f1, dur, shape=2.0):
    """指数滑音的相位（f0 → f1），返回 sin"""
    t = t_(dur)
    k = (t / dur) ** (1 / shape)
    f = f0 * (f1 / f0) ** k
    return np.sin(2 * np.pi * np.cumsum(f) / SR)


def bp(x, lo, hi, order=2):
    return signal.sosfilt(signal.butter(order, [lo, hi], 'bandpass', fs=SR, output='sos'), x)


def lp(x, f, order=2):
    return signal.sosfilt(signal.butter(order, f, 'lowpass', fs=SR, output='sos'), x)


def hp(x, f, order=2):
    return signal.sosfilt(signal.butter(order, f, 'highpass', fs=SR, output='sos'), x)


def sweep_filter(x, f0, f1, q=3.0, kind='bp', blocks=64):
    """随时间扫频的滤波（分块，每块换一组系数）：呼呼的挥拳声、往上冲的升龙都靠它"""
    out = np.zeros_like(x)
    n = len(x)
    edges = np.linspace(0, n, blocks + 1).astype(int)
    zi = None
    for i in range(blocks):
        k = i / max(blocks - 1, 1)
        f = f0 * (f1 / f0) ** k
        if kind == 'bp':
            sos = signal.butter(2, [f / (1 + 0.5 / q), min(f * (1 + 0.5 / q), SR / 2 - 100)], 'bandpass', fs=SR, output='sos')
        else:
            sos = signal.butter(2, min(f, SR / 2 - 100), 'lowpass', fs=SR, output='sos')
        if zi is None:
            zi = signal.sosfilt_zi(sos) * 0
        seg, zi = signal.sosfilt(sos, x[edges[i]:edges[i + 1]], zi=zi)
        out[edges[i]:edges[i + 1]] = seg
    return out


def pad(x, n):
    return np.pad(x, (0, max(0, n - len(x))))


def mix(*layers):
    n = max(len(l) for l in layers)
    return sum(pad(l, n) for l in layers)


def at(x, sec):
    """把 x 推后 sec 秒"""
    return np.concatenate([np.zeros(int(SR * sec)), x])


def drive(x, k):
    """软削波：k 越大越脏"""
    return np.tanh(x * k) / np.tanh(k)


def click(dur=0.003, bright=6000):
    return hp(noise(dur) * np.hanning(int(SR * dur)), bright, 1) * 2.5


_IR = {}


def reverb(x, size=0.6, wet=0.25, damp=4500, pre=0.012):
    """卷积混响：脉冲响应 = 指数衰减的噪声（左右不相关就是立体声；这里单声道够用），越往后越暗（低通逐段收紧）"""
    key = (size, damp, pre)
    if key not in _IR:
        n = int(SR * size)
        ir = noise(size) * np.exp(-6.9 * t_(size) / size)
        ir = np.concatenate([lp(ir[: n // 3], damp), lp(ir[n // 3: 2 * n // 3], damp * 0.6), lp(ir[2 * n // 3:], damp * 0.35)])
        ir = np.concatenate([np.zeros(int(SR * pre)), ir])
        _IR[key] = ir / np.sqrt((ir ** 2).sum())
    w = signal.fftconvolve(x, _IR[key])
    return pad(x, len(w)) * (1 - wet * 0.5) + w * wet


def master(x, peak=0.89, tail_db=-60):
    """归一到 -1dBFS；去掉尾巴上听不见的静音；开头结尾各 2ms 淡入淡出防爆音"""
    x = hp(x - np.mean(x), 80, 2)            # 80Hz 以下手机放不出，留着只占动态余量、让别的频段变小声
    x = x / (np.max(np.abs(x)) + 1e-9) * peak
    thr = peak * 10 ** (tail_db / 20)
    idx = np.where(np.abs(x) > thr)[0]
    x = x[: (idx[-1] + int(SR * 0.02)) if len(idx) else len(x)]
    f = int(SR * 0.002)
    x[:f] *= np.linspace(0, 1, f)
    x[-f:] *= np.linspace(1, 0, f)
    return x


# ───────── 音效 ─────────
# 手机外放基本放不出 150Hz 以下（实测第一版的重击 88% 的能量在 150Hz 以下：手机上只剩「哒」一声，听着就是廉价的电子音）。
# 所以「嘭」的身体放在 150~600Hz 起滑，150Hz 以下只留一点（耳机里有底），低频的分量靠 drive 失真生出的谐波「暗示」出来，
# 「啪」的质感放在 1~4kHz（手机喇叭最响的频段）。gen 完 analyze() 会打出每条的分频能量，<150Hz 超过 45% 的打印警告
def whoosh(dur, f0, f1, q=2.2, level=1.0):
    x = sweep_filter(noise(dur), f0, f1, q) * np.sin(np.pi * np.clip(t_(dur) / dur, 0, 1)) ** 1.6
    return x * level


def thump(f0, f1, dur, curve=5):
    return sweep(f0, f1, dur, 1.5) * env(dur, 0.001, dur, curve)


def S_swingL():
    return master(whoosh(0.13, 900, 2600, 2.5))


def S_swingH():
    return master(mix(whoosh(0.24, 380, 1700, 1.8), 0.25 * lp(noise(0.24), 300) * env(0.24, 0.05, 0.2, 3)))


def S_hitL():
    body = thump(420, 150, 0.08, 4)
    skin = bp(noise(0.07), 900, 4200) * env(0.07, 0.001, 0.05, 5)
    slap = bp(noise(0.04), 300, 1200) * env(0.04, 0.0005, 0.025, 5)
    x = mix(click(0.002, 4000) * 0.9, body * 0.8, skin * 1.0, slap * 0.9, thump(120, 60, 0.1) * 0.25)
    return master(reverb(drive(x, 2.2), 0.25, 0.12))


def S_hitH():
    body = thump(320, 95, 0.2, 4)
    sub = thump(90, 40, 0.3, 4) * 0.3
    crack = bp(noise(0.06), 250, 1100) * env(0.06, 0.0005, 0.04, 5)
    crunch = bp(noise(0.18), 900, 3500) * env(0.18, 0.001, 0.12, 4)
    snap = hp(noise(0.03), 3000) * env(0.03, 0.0005, 0.02, 6)
    x = mix(click(0.003, 3000) * 1.2, body * 0.9, sub, crack * 1.2, crunch * 0.9, snap * 0.6)
    return master(reverb(drive(x, 3.2), 0.5, 0.2))


def S_block():
    # 前臂硬挡：一声闷响 + 几个不成谐波的共振峰快速衰减（「梆」）
    t = t_(0.18)
    modes = sum(a * np.sin(2 * np.pi * f * t) * np.exp(-t * d) for f, a, d in ((610, 1, 38), (1480, 0.6, 55), (2390, 0.4, 70), (3710, 0.25, 90)))
    x = mix(click(0.002, 2500), modes * 0.6, thump(220, 120, 0.06) * 0.6, bp(noise(0.05), 1500, 6000) * env(0.05, 0.0005, 0.03, 6) * 0.4)
    return master(reverb(drive(x, 1.6), 0.3, 0.15))


def crackle(dur, rate, bright=3000):
    """噼啪：泊松分布的短脉冲（火焰的爆裂、电弧的跳火）"""
    n = int(SR * dur)
    x = np.zeros(n)
    k = RNG.poisson(rate * dur)
    for p in RNG.integers(0, max(1, n - 400), k):
        L = int(RNG.integers(40, 300))
        x[p:p + L] += RNG.standard_normal(L) * np.exp(-np.arange(L) / (L / 4)) * RNG.uniform(0.3, 1)
    return hp(x, bright, 1)


def S_fire():
    dur = 0.75
    roar = sweep_filter(noise(dur), 300, 1800, 0.8, 'lp') * env(dur, 0.04, 0.5, 3)
    roar = sweep_filter(noise(dur), 600, 3200, 0.8, 'lp') * env(dur, 0.03, 0.5, 3)
    rumble = bp(noise(dur), 120, 520) * env(dur, 0.02, 0.6, 3) * 1.6
    launch = thump(300, 90, 0.2) * 0.8
    x = mix(roar, rumble, launch, crackle(dur, 70, 2000) * env(dur, 0.01, 0.6, 2.5) * 0.8)
    return master(reverb(drive(x, 2.5), 0.6, 0.22))


def S_ice():
    dur = 0.7
    t = t_(dur)
    # 冰晶：几个高频不协和泛音（玻璃感）+ 电弧噼啪 + 嘶嘶的高频噪声扫上去
    bell = sum(np.sin(2 * np.pi * f * t) * np.exp(-t * d) * a for f, a, d in ((1870, 1, 6), (2960, 0.7, 8), (4410, 0.5, 10), (6230, 0.3, 12)))
    hiss = sweep_filter(noise(dur), 2500, 7000, 2) * env(dur, 0.01, 0.5, 3)
    zap = crackle(dur, 140, 2500) * env(dur, 0.005, 0.45, 2.5)
    buzz = signal.sawtooth(2 * np.pi * 90 * t) * (RNG.random(len(t)) > 0.6) * env(dur, 0.01, 0.3, 4) * 0.25
    x = mix(bell * 0.35, hiss * 0.8, zap * 1.2, lp(buzz, 1800), thump(300, 120, 0.12) * 0.5)
    return master(reverb(drive(x, 1.8), 0.9, 0.3, 8000))


def S_rise():
    dur = 0.5
    up = sweep_filter(noise(dur), 250, 3500, 1.4) * env(dur, 0.01, 0.4, 2.5)
    tone = sweep(220, 660, dur, 1) * env(dur, 0.01, 0.4, 3) * 0.4
    x = mix(click(0.003, 2000), up, tone, crackle(dur, 60, 2200) * 0.8, thump(320, 110, 0.18) * 0.8)
    return master(reverb(drive(x, 2.6), 0.5, 0.2))


def S_superStart():
    # 超必杀起手：倒放的混响「吸」上去 → 一记重锤 + 金属的长尾音
    dur = 0.55
    suck = reverb(sweep_filter(noise(dur), 500, 6000, 1.5) * np.linspace(0, 1, int(SR * dur)) ** 2, 0.4, 0.4)[: int(SR * dur)]
    t = t_(1.2)
    gong = sum(np.sin(2 * np.pi * f * t) * np.exp(-t * d) * a for f, a, d in ((196, 1, 2.2), (293, 0.6, 2.8), (467, 0.45, 3.5), (811, 0.3, 4.5), (1230, 0.2, 6)))
    hit = mix(click(0.004, 1500), thump(240, 55, 0.45, 3) * 1.0, gong * 0.6)
    x = mix(suck * 0.7, at(hit, dur - 0.02))
    return master(reverb(drive(x, 1.8), 1.4, 0.32))


def S_super():
    dur = 1.3
    t = t_(dur)
    beam = sweep_filter(noise(dur), 400, 2200, 0.7, 'lp') * env(dur, 0.03, 1.1, 2)
    beam = sweep_filter(noise(dur), 700, 3500, 0.7, 'lp') * env(dur, 0.03, 1.1, 2)
    sub = (np.sin(2 * np.pi * 55 * t) + 0.8 * np.sin(2 * np.pi * 165 * t) + 0.5 * np.sin(2 * np.pi * 275 * t)) * env(dur, 0.03, 1.1, 2) * 0.35
    wobble = 1 + 0.35 * np.sin(2 * np.pi * 11 * t)
    x = mix(thump(300, 90, 0.5, 3) * 1.0, beam * wobble * 1.3, sub, crackle(dur, 90, 1800) * 0.9 * env(dur, 0.02, 1, 2))
    return master(reverb(drive(x, 3.0), 1.2, 0.25))


def S_clash():
    dur = 0.5
    x = mix(click(0.004, 1500), thump(300, 70, 0.3) * 0.9, bp(noise(dur), 400, 5000) * env(dur, 0.001, 0.25, 4),
            crackle(dur, 120, 1800) * env(dur, 0.01, 0.3, 3) * 0.8)
    return master(reverb(drive(x, 3), 0.9, 0.3))


def S_ko():
    # 终结一击：超重打击 + 爆炸的低频 + 很长的混响尾巴（街机 K.O. 那一下全场回声）
    dur = 0.9
    boom = bp(noise(dur), 90, 900) * env(dur, 0.002, 0.7, 3) * 2
    x = mix(click(0.004, 2000) * 1.4, thump(300, 80, 0.6, 3) * 1.1, boom, bp(noise(0.3), 500, 3500) * env(0.3, 0.001, 0.2, 4) * 1.2,
            thump(80, 30, 0.6, 3) * 0.3)
    return master(reverb(drive(x, 3.5), 2.2, 0.45, 3500))


def S_thud():
    x = mix(thump(260, 95, 0.25, 4), bp(noise(0.35), 150, 900) * env(0.35, 0.005, 0.3, 4) * 0.8, click(0.003, 1500) * 0.5)
    return master(reverb(drive(x, 2), 0.4, 0.15))


def S_step():
    x = mix(thump(240, 110, 0.05) * 0.6, bp(noise(0.08), 300, 1800) * env(0.08, 0.002, 0.06, 4) * 0.8)
    return master(x, peak=0.5)


def S_dash():
    return master(mix(whoosh(0.22, 500, 2200, 1.6), S_step() * 0.6))


def S_ui():
    t = t_(0.09)
    x = (np.sin(2 * np.pi * 1320 * t) + 0.4 * np.sin(2 * np.pi * 2640 * t)) * env(0.09, 0.001, 0.07, 5)
    return master(reverb(x, 0.3, 0.15), peak=0.6)


def S_select():
    t = t_(0.3)
    x = mix(sweep(300, 900, 0.18) * env(0.18, 0.002, 0.15, 3) * 0.6, whoosh(0.3, 600, 4000, 2) * 0.6, click(0.002, 3000))
    return master(reverb(drive(x, 1.5), 0.6, 0.25))


# ───────── 解说（macOS say） ─────────
VOICE = {
    'v_round1': 'Round one.', 'v_round2': 'Round two.', 'v_round3': 'Final round.',
    'v_fight': 'Fight!', 'v_ko': 'K. O.', 'v_perfect': 'Perfect!', 'v_win': 'You win!', 'v_lose': 'You lose.',
    'v_timeover': 'Time over.', 'v_draw': 'Draw game.', 'v_title': 'Fist soul!',
}


def say(text):
    with tempfile.TemporaryDirectory() as d:
        aiff, wav = os.path.join(d, 'v.aiff'), os.path.join(d, 'v.wav')
        if subprocess.run(['say', '-v', 'Daniel', '-r', '150', '-o', aiff, text], capture_output=True).returncode:
            return None
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', aiff, '-ac', '1', '-ar', str(SR), wav], check=True)
        sr, x = __import__('scipy.io.wavfile', fromlist=['read']).read(wav)
    x = x.astype(np.float64) / 32768
    # 降调 15%（重采样拉长：变低变粗）→ 带通 → 压缩（先 drive 再归一）→ 大厅混响
    x = signal.resample(x, int(len(x) * 1.18))
    x = bp(x, 90, 6500)
    x = drive(x / (np.max(np.abs(x)) + 1e-9), 2.4)
    x = mix(x, at(x, 0.09) * 0.18)                 # 一点点短回声：体育馆的拍击回声
    return master(reverb(x, 1.6, 0.3, 5000))


# ───────── 输出 ─────────
def write(name, x, keep_wav):
    os.makedirs(OUT, exist_ok=True)
    pcm = (np.clip(x, -1, 1) * 32767).astype(np.int16)
    with tempfile.NamedTemporaryFile(suffix='.wav', delete=False) as f:
        tmp = f.name
    __import__('scipy.io.wavfile', fromlist=['write']).write(tmp, SR, pcm)
    # 单声道 44.1k、VBR 质量 5（~64kbps）：打击声的瞬态还在，一条十几 KB
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', tmp, '-codec:a', 'libmp3lame', '-q:a', '5', os.path.join(OUT, name + '.mp3')], check=True)
    if keep_wav:
        os.makedirs(WAVDIR, exist_ok=True)
        os.replace(tmp, os.path.join(WAVDIR, name + '.wav'))
    else:
        os.remove(tmp)
    return len(x) / SR


def analyze(names):
    """每条的分频能量：手机外放判据是 <150Hz 不超过 45%"""
    for n in names:
        p = os.path.join(OUT, n + '.mp3')
        if not os.path.isfile(p):
            continue
        x = np.frombuffer(subprocess.run(['ffmpeg', '-v', 'error', '-i', p, '-f', 's16le', '-ac', '1', '-ar', str(SR), '-'],
                                         capture_output=True).stdout, np.int16) / 32768
        F = np.abs(np.fft.rfft(x)) ** 2
        fr = np.fft.rfftfreq(len(x), 1 / SR)
        lo = F[fr < 150].sum() / F.sum() * 100
        mid = F[(fr >= 300) & (fr < 4000)].sum() / F.sum() * 100
        if lo > 45:
            print(f'  ⚠ {n}：<150Hz 占 {lo:.0f}%（手机外放听不见），300Hz~4kHz 占 {mid:.0f}%')


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    keep = '--wav' in sys.argv
    sfx = {k[2:]: v for k, v in globals().items() if k.startswith('S_')}
    names = args or (list(sfx) + list(VOICE))
    done = []
    for n in names:
        if n in sfx:
            x = sfx[n]()
        elif n in VOICE:
            x = say(VOICE[n])
            if x is None:
                print(f'  跳过 {n}（没有 say）')
                continue
        else:
            sys.exit(f'没有这条：{n}')
        d = write(n, x, keep)
        done.append(f'{n} {d:.2f}s')
    analyze(names)
    total = sum(os.path.getsize(os.path.join(OUT, f)) for f in os.listdir(OUT))
    print('生成：' + '，'.join(done))
    print(f'sfx/ 共 {len(os.listdir(OUT))} 个文件 {total // 1024}KB')


if __name__ == '__main__':
    main()
