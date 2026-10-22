#!/usr/bin/env python3
"""出帧节拍：屏上每个 vsync 显示的是引擎的哪一帧（Flutter 宿主 · Android）。

引擎画进 Flutter 的外部纹理，Flutter 光栅线程取纹理时拿「此刻最新的那张」。两边各跟各的 vsync：
引擎帧交得比 Flutter 取纹理晚一点，这个 vsync 就重复上一张，下一个 vsync 一次跳两张 ——
引擎侧帧耗时、每帧位移都均匀，屏上照样一顿一顿。这个脚本把三路日志对起来看屏上实际的那一串：

  🎞E <帧时间 ms> <页面滚动> <操作菜单顶边>   引擎（Rust，MINI_PACE）
  🎞  <vsync µs> <交出 µs>                     宿主（Kotlin，pump 返回 = 已交给 Surface）
  🎞F <vsync µs> <光栅开始 µs> <光栅结束 µs>   Flutter（FrameTiming）

用法：
  adb shell setprop debug.mini.pace 1   # 重开小程序生效
  adb logcat -c; …操作…; adb logcat -d -v raw -s mini-render:I flutter:I > pace.log
  python3 tools/pace-check.py pace.log [--latch-ms 0.5] [--field scroll|sheet]

判据：屏上每个 vsync 换一张、且每张都比上一张新一帧（「重复」「跳帧」都是 0）；
显示出来的位移变异系数与引擎侧接近。
"""
import re
import statistics
import sys


def parse(path):
    eng, host, flut = {}, [], []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.strip()
        m = re.search(r"🎞E (\d+) (-?[\d.]+) (-?[\d.]+)", line)
        if m:
            eng[int(m.group(1))] = (float(m.group(2)), float(m.group(3)))
            continue
        m = re.search(r"🎞F (\d+) (\d+) (\d+)", line)
        if m:
            flut.append(tuple(int(m.group(i)) for i in (1, 2, 3)))
            continue
        m = re.search(r"🎞 (\d+) (\d+)", line)
        if m:
            host.append((int(m.group(1)), int(m.group(2))))
    return eng, host, flut


def cv(xs):
    xs = [x for x in xs]
    if len(xs) < 3:
        return float("nan")
    m = statistics.mean(xs)
    return statistics.pstdev(xs) / abs(m) * 100 if m else float("nan")


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        sys.exit(2)
    path = args[0]
    latch_ms = float(args[args.index("--latch-ms") + 1]) if "--latch-ms" in args else 0.0
    field = args[args.index("--field") + 1] if "--field" in args else "scroll"
    eng, host, flut = parse(path)
    # 引擎帧：(交出 µs, vsync µs, 值)，按交出时刻排
    frames = []
    for vs, done in host:
        e = eng.get(round(vs / 1000)) or eng.get(vs // 1000)
        if e is None:
            continue
        val = e[0] if field == "scroll" else e[1]
        frames.append((done, vs, val))
    frames.sort()
    if len(frames) < 5 or not flut:
        print(f"❌ 数据不够：引擎帧 {len(frames)}、Flutter 帧 {len(flut)}（debug.mini.pace 开了吗？重开过小程序吗？）")
        sys.exit(1)
    # 只看「值在变」的那一段（拖动 / 动画进行中）
    moving = [i for i in range(1, len(frames)) if frames[i][2] != frames[i - 1][2]]
    if not moving:
        print("❌ 这段日志里值没变过（没在滑 / 没弹菜单）")
        sys.exit(1)
    t0, t1 = frames[moving[0] - 1][0], frames[moving[-1]][0]
    latch = latch_ms * 1000
    # Flutter（3.32，ImageReaderSurfaceProducer）的取图规则：交上来的图进一个队列，**超过 2 张丢最老的**；
    # 光栅线程画纹理时取队首那张（不是最新那张），队列空了就沿用上一张。--latest 按「取最新」算（旧版行为）
    latest = "--latest" in args
    shown = []  # (Flutter vsync, 显示的引擎帧下标, 交出到取用的余量 µs)
    queue, nxt, cur = [], 0, None
    for vs, rs, rf in sorted(flut):
        while nxt < len(frames) and frames[nxt][0] <= rs + latch:
            queue.append(nxt)
            if len(queue) > 2:
                queue.pop(0)
            nxt += 1
        if latest:
            cur = queue[-1] if queue else cur
            queue = []
        elif queue:
            cur = queue.pop(0)
        if cur is not None and t0 <= rs <= t1 + 20_000:
            shown.append((vs, cur, rs + latch - frames[cur][0]))
    if len(shown) < 5:
        print(f"❌ 动起来的那段里只对上 {len(shown)} 个 Flutter 帧")
        sys.exit(1)
    steps = [b[1] - a[1] for a, b in zip(shown, shown[1:])]
    gaps = [(b[0] - a[0]) / 1000 for a, b in zip(shown, shown[1:])]
    period = statistics.median(gaps)
    eng_moving = [f for f in frames if t0 <= f[0] <= t1]
    eng_steps = [b[2] - a[2] for a, b in zip(eng_moving, eng_moving[1:])]
    disp = [frames[b[1]][2] - frames[a[1]][2] for a, b in zip(shown, shown[1:])]
    near = sum(1 for s in shown if s[2] < 1000)
    print(f"引擎 {len(eng_moving)} 帧 / Flutter {len(shown)} 帧（vsync 间隔中位数 {period:.2f}ms，"
          f"Flutter 自己漏掉的 vsync {sum(1 for g in gaps if g > period * 1.5)} 次）")
    print(f"屏上每个 vsync：换一张 {steps.count(1)}、重复上一张 {steps.count(0)}、跳过 ≥1 张 {sum(1 for s in steps if s >= 2)}")
    print(f"引擎帧交出 → 被 Flutter 取走的余量：中位数 {statistics.median(s[2] for s in shown) / 1000:.2f}ms，"
          f"不到 1ms 的 {near} 帧（这些帧再晚一点就赶不上）")
    print(f"每帧位移变异系数：引擎侧 {cv(eng_steps):.0f}% → 屏上 {cv(disp):.0f}%（{field}）")


if __name__ == "__main__":
    main()
