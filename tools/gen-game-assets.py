#!/usr/bin/env python3
"""生成小游戏夹具的音频素材（game-doc/07 §6；素材不进仓库，sample-game/*/assets/ 在 .gitignore 里）。

    python3 tools/gen-game-assets.py      # 写 sample-game/g-audio/assets/，打印 sha256

- a440.wav：440Hz 正弦，48kHz 单声道 16bit，1.000s，幅度 0.5
- click.wav：1ms 满幅方波 + 9ms 静音（延迟测量 / 并发音效）
- bgm.wav：10s C 大三和弦（261.63 / 329.63 / 392.00 Hz，各 0.2），44.1kHz 立体声 16bit
- bgm.mp3 / bgm.m4a / bgm.ogg：有 ffmpeg 时从 bgm.wav 编出（参数写在下面）
"""
import hashlib, math, shutil, struct, subprocess, wave
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "sample-game/g-audio/assets"


def write_wav(path, rate, channels, frames):
    with wave.open(str(path), "wb") as w:
        w.setnchannels(channels)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(b"".join(struct.pack("<" + "h" * channels, *[max(-32768, min(32767, int(round(v * 32767))))] * channels) for v in frames))


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    write_wav(OUT / "a440.wav", 48000, 1, (0.5 * math.sin(2 * math.pi * 440 * i / 48000) for i in range(48000)))
    write_wav(OUT / "click.wav", 48000, 1, (1.0 if i < 24 else -1.0 if i < 48 else 0.0 for i in range(480)))
    write_wav(OUT / "bgm.wav", 44100, 2, (sum(0.2 * math.sin(2 * math.pi * f * i / 44100) for f in (261.63, 329.63, 392.0)) for i in range(441000)))
    if shutil.which("ffmpeg"):
        for ext, args in (("mp3", ["-b:a", "128k"]), ("m4a", ["-c:a", "aac", "-b:a", "128k"]), ("ogg", ["-c:a", "libvorbis", "-q:a", "4"])):
            subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", str(OUT / "bgm.wav"), *args, str(OUT / f"bgm.{ext}")], check=False)
    for p in sorted(OUT.iterdir()):
        print(hashlib.sha256(p.read_bytes()).hexdigest()[:16], p.name)


if __name__ == "__main__":
    main()
