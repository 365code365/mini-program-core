#!/usr/bin/env python3
"""并行跑全部 Rust 测试（替代 `cargo test --release` 那一步）：把测试切成很多个**进程**，排满所有核。

为什么要它：`cargo test` 一个测试二进制接一个跑，二进制里的用例又被 `SERIAL` 锁串起来（包根目录、数据目录、
组件模板是进程级的，同一进程里并行会串数据），11 核的机器上实际只用一个核 —— 2026-11-05 一轮 707s，
其中 game_e2e 一个二进制就 237s。进程之间不共享那些全局状态，所以按进程切开就能放心并行：

- 每个测试二进制按上一轮的耗时切成若干**分片**（每片约 `--shard` 秒，默认 6s），一片一个进程，
  传 `--exact <用例名…>` 只跑那几个；没有上一轮耗时时按最近一批 `target/test_batches/*/logs/test.log` 估；
- 所有分片按「估计最长的先跑」排进 `--workers` 个并发槽（LPT 调度，默认按核数与内存取）；
- 失败的用例逐个单独再跑一次：再跑过了记成「偶发」（照样列出来，不算通过也不算失败），再跑还失败才算失败；
- 耗时写回 `target/test_timings.json`（下一轮切片、排序用），每片的输出在 `target/_fast/<片>.log`。

    python3 tools/test-fast.py                    # 全部（先 `cargo test --release --no-run` 编好）
    python3 tools/test-fast.py --changed          # 只跑改动涉及的（git 工作区相对 HEAD，规则见 select_by_changes）
    python3 tools/test-fast.py --only sample_     # 二进制名包含 sample_ 的
    python3 tools/test-fast.py --workers 6

退出码：0 = 没有失败（可能有偶发）；1 = 有失败；2 = 编译失败。
只用标准库。
"""
import argparse
import concurrent.futures as cf
import json
import math
import os
import re
import subprocess
import sys
import time

def ensure_native():
    """这台机器 PATH 里的 python3 是 x86_64 的（/Library/Frameworks/Python.framework 3.7，跑在 Rosetta 下）。
    从它起的进程一路往下都偏好 x86_64：测试起的 Chrome 也是 Rosetta 版，SwiftShader 慢 5 倍（3D 游戏冷启动 4s → 22s）。
    发现自己在 Rosetta 里就换成原生的 python3 重来一遍"""
    if sys.platform != 'darwin' or os.environ.get('MINI_FAST_NATIVE'):
        return
    try:
        translated = subprocess.run(['sysctl', '-n', 'sysctl.proc_translated'], capture_output=True, text=True).stdout.strip() == '1'
    except OSError:
        return
    if not translated:
        return
    for py in ('/opt/homebrew/bin/python3', '/usr/bin/python3'):
        if os.path.exists(py):
            os.environ['MINI_FAST_NATIVE'] = '1'
            os.execv('/usr/bin/arch', ['arch', '-arm64', py, os.path.abspath(__file__)] + sys.argv[1:])


ensure_native()

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'target', '_fast')
TIMINGS = os.path.join(ROOT, 'target', 'test_timings.json')
# 起 headless Chrome 的用例（小游戏高性能模式）：一个用例一个 Chrome，同时最多 --chrome 个（多了内存与 GPU 进程扛不住，
# 2026-11-06 第一版不限，十来个 Chrome 一起起，交换区吃满、44 个用例超时）
HEAVY_RE = re.compile(r'^web\d?_cases::')
RESULT_RE = re.compile(r'^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored')
FAIL_RE = re.compile(r'^test (\S.*?) \.\.\. FAILED$')
PANIC_RE = re.compile(r"^thread '([^']+)' .*panicked at")


def log(msg):
    print(msg, flush=True)


def build():
    """编测试二进制，返回 [(名字, 可执行文件)]。名字：lib 是 `lib`，集成测试是文件名（不带 .rs），bin 是 `bin:<名>`"""
    cmd = ['cargo', 'test', '--release', '--no-run', '--message-format=json-render-diagnostics']
    p = subprocess.Popen(cmd, cwd=ROOT, stdout=subprocess.PIPE, stderr=None, text=True)
    exes = []
    for line in p.stdout:
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get('reason') != 'compiler-artifact' or not m.get('profile', {}).get('test') or not m.get('executable'):
            continue
        t = m['target']
        kinds = t['kind']
        name = 'lib' if {'lib', 'rlib', 'cdylib', 'staticlib'} & set(kinds) else t['name'] if 'test' in kinds else f"{kinds[0]}:{t['name']}"
        exes.append((name, m['executable']))
    if p.wait() != 0:
        sys.exit(2)
    return sorted(set(exes))


def list_tests(exe):
    r = subprocess.run([exe, '--list', '--format', 'terse'], cwd=ROOT, capture_output=True, text=True)
    return [l[:-len(': test')] for l in r.stdout.splitlines() if l.endswith(': test')]


def load_timings():
    try:
        with open(TIMINGS, encoding='utf-8') as f:
            return json.load(f)
    except (OSError, ValueError):
        return {}


def seed_from_batches():
    """没有耗时表时：从最近一批全量回归的 test.log 里取每个二进制的 finished in"""
    d = os.path.join(ROOT, 'target', 'test_batches')
    try:
        batches = sorted(x for x in os.listdir(d) if os.path.isfile(os.path.join(d, x, 'logs', 'test.log')))
    except OSError:
        return {}
    if not batches:
        return {}
    seed, cur = {}, None
    with open(os.path.join(d, batches[-1], 'logs', 'test.log'), encoding='utf-8', errors='replace') as f:
        for line in f:
            m = re.search(r'Running (?:unittests )?(\S+?)(?:\.rs)? \(', line)
            if m:
                src = m.group(1)
                cur = 'lib' if src.endswith('src/lib') else os.path.basename(src) if src.startswith('tests/') else None
                continue
            m = re.search(r'finished in ([\d.]+)s', line)
            if m and cur:
                seed[cur] = {'total': float(m.group(1)), 'tests': {}}
                cur = None
    return seed


def plan(exes, timings, shard_s):
    """[(估计秒数, 二进制名, 可执行文件, [用例])]，一项一个进程"""
    jobs = []
    # 列用例也并行（77 个二进制一个个 --list 要几十秒：每个进程起来先映射几十 MB）
    with cf.ThreadPoolExecutor(os.cpu_count() or 4) as pool:
        listed = dict(zip([n for n, _ in exes], pool.map(lambda ne: list_tests(ne[1]), exes)))
    for name, exe in exes:
        tests = listed[name]
        heavy = [t for t in tests if HEAVY_RE.match(t)]
        tests = [t for t in tests if not HEAVY_RE.match(t)]
        per0 = timings.get(name, {}).get('tests', {})
        for t in heavy:
            jobs.append((per0.get(t, 8.0), name, exe, [t], True))
        if not tests:
            continue
        info = timings.get(name, {})
        per = info.get('tests', {})
        total = info.get('total', 2.0)
        avg = total / max(len(tests), 1)
        est = {t: per.get(t, avg) for t in tests}
        n = max(1, min(len(tests), math.ceil(sum(est.values()) / shard_s)))
        # 分片：估计最长的用例先放，每次放进当前最空的那一片
        shards = [[0.0, []] for _ in range(n)]
        for t in sorted(tests, key=lambda t: -est[t]):
            s = min(shards, key=lambda s: s[0])
            s[0] += est[t]
            s[1].append(t)
        for secs, ts in shards:
            if ts:
                jobs.append((secs, name, exe, ts, False))
    return sorted(jobs, key=lambda j: -j[0])


def child_env():
    env = dict(os.environ)
    # 与 cargo test 一致：有的用例在运行时读这几个
    env.setdefault('CARGO_MANIFEST_DIR', ROOT)
    env.setdefault('CARGO_PKG_NAME', 'mini-render')
    env['RUST_BACKTRACE'] = env.get('RUST_BACKTRACE', '0')
    return env


def run(job, idx, threads, env, timeout):
    secs, name, exe, tests, _heavy = job
    logf = os.path.join(OUT, f'{idx:04d}_{name.replace(":", "_")}.log')
    args = [exe, '--exact', '--test-threads', str(threads)] + tests
    t0 = time.time()
    with open(logf, 'w', encoding='utf-8') as f:
        try:
            r = subprocess.run(args, cwd=ROOT, env=env, stdout=f, stderr=subprocess.STDOUT, timeout=timeout)
            rc = r.returncode
        except subprocess.TimeoutExpired:
            rc = 124
            f.write(f'\n⏱ 超时 {timeout}s\n')
    dt = time.time() - t0
    passed = failed = 0
    fails = []
    with open(logf, encoding='utf-8', errors='replace') as f:
        for line in f:
            m = RESULT_RE.match(line)
            if m:
                passed, failed = int(m.group(1)), int(m.group(2))
            m = FAIL_RE.match(line.rstrip())
            if m:
                fails.append(m.group(1))
    if rc != 0 and not fails:
        # 崩了（段错误 / 超时 / abort）：这一片里还没报结果的全算失败
        fails = list(tests)
        failed = len(tests)
    return {'name': name, 'exe': exe, 'tests': tests, 'secs': dt, 'passed': passed, 'failed': failed, 'fails': fails, 'log': logf, 'rc': rc}


def select_by_changes(exes):
    """按 git 工作区相对 HEAD 改了哪些文件挑二进制。规则从宽：说不清的一律全跑"""
    r = subprocess.run(['git', 'status', '--porcelain'], cwd=ROOT, capture_output=True, text=True)
    paths = [l[3:].strip().strip('"') for l in r.stdout.splitlines() if len(l) > 3]
    names = {n for n, _ in exes}
    want = set()
    for p in paths:
        if p.startswith(('doc/', 'site/', 'game-doc/', '.agents/', '.kiro/', 'README', 'sdk/', 'tools/mini-store/')) or p.endswith('.md'):
            continue  # 文档、落地页、移动端 SDK 源码：不影响 Rust 测试
        m = re.match(r'sample/([^/]+)/', p)
        if m:
            t = f"sample_{m.group(1).replace('-', '_')}_e2e"
            want.add(t if t in names else '*samples*')
            continue
        if p.startswith(('sample-game/', 'src/game/', 'tests/game_')):
            want.update({'game_e2e', 'lib'})
            continue
        if p.startswith('tests/') and p.endswith('.rs') and '/' not in p[6:]:
            want.add(os.path.basename(p)[:-3])
            continue
        if p.startswith('tests/sample_harness/'):
            want.add('*samples*')
            continue
        return exes  # 引擎源码、Cargo.toml、测试夹具…：全跑
    picked = [(n, e) for n, e in exes if n in want or ('*samples*' in want and n.startswith('sample_'))]
    return picked


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--workers', type=int, default=0)
    ap.add_argument('--shard', type=float, default=6.0, help='每片的目标秒数')
    ap.add_argument('--threads', type=int, default=2, help='每个进程里 libtest 的线程数')
    ap.add_argument('--only', default='', help='只跑名字里含这个串的二进制')
    ap.add_argument('--changed', action='store_true', help='只跑改动涉及的')
    ap.add_argument('--quick', action='store_true', help='不跑起 Chrome 的用例（小游戏高性能模式，约 40 个）')
    ap.add_argument('--chrome', type=int, default=3, help='同时最多几个起 Chrome 的用例')
    ap.add_argument('--no-retry', action='store_true')
    ap.add_argument('--timeout', type=int, default=600, help='一片最多跑多久')
    a = ap.parse_args()

    t_all = time.time()
    exes = build()  # 编好了的话 cargo 只做一次新鲜度检查，照样给出可执行文件
    if a.only:
        exes = [(n, e) for n, e in exes if a.only in n]
    if a.changed:
        exes = select_by_changes(exes)
        log(f"按改动挑了 {len(exes)} 个测试二进制：{' '.join(n for n, _ in exes) if len(exes) < 12 else '（全部）'}")
    if not exes:
        log('测试：通过 0，失败 0（没有要跑的）')
        return 0
    timings = load_timings() or seed_from_batches()
    ncpu = os.cpu_count() or 4
    workers = a.workers or ncpu
    os.makedirs(OUT, exist_ok=True)
    for f in os.listdir(OUT):
        os.remove(os.path.join(OUT, f))
    t_plan = time.time()
    jobs = plan(exes, timings, a.shard)
    if a.quick:
        skipped = sum(1 for j in jobs if j[4])
        jobs = [j for j in jobs if not j[4]]
        if skipped:
            log(f"--quick：跳过 {skipped} 个起 Chrome 的用例")
    log(f"{len(exes)} 个测试二进制 → {len(jobs)} 个进程，{workers} 路并发（列用例 {time.time() - t_plan:.1f}s）")
    env = child_env()
    results = []
    t_run = time.time()
    # 两段：先 CPU 的那些排满所有核；再单独跑起 Chrome 的（SwiftShader 软件 WebGL 很吃 CPU，与别的挤在一起时
    # 一个 3D 游戏冷启动从 4.6s 拖到 85s，还会超时黑屏）
    cpu_jobs = [(i, j) for i, j in enumerate(jobs) if not j[4]]
    heavy_jobs = [(i, j) for i, j in enumerate(jobs) if j[4]]
    for lane, n in ((cpu_jobs, workers), (heavy_jobs, a.chrome)):
        if not lane:
            continue
        t_lane = time.time()
        with cf.ThreadPoolExecutor(n) as pool:
            futs = [pool.submit(run, j, i, a.threads, env, a.timeout) for i, j in lane]
            for fu in cf.as_completed(futs):
                r = fu.result()
                results.append(r)
                if r['fails']:
                    log(f"  ✗ {r['name']}：{', '.join(r['fails'][:5])}{' …' if len(r['fails']) > 5 else ''}（{r['log']}）")
        log(f"  {'起 Chrome 的' if lane is heavy_jobs else 'CPU'} {len(lane)} 个进程，{n} 路并发，{time.time() - t_lane:.0f}s")
    run_s = time.time() - t_run
    passed = sum(r['passed'] for r in results)
    fails = [(r['name'], r['exe'], t) for r in results for t in r['fails']]
    flaky, hard = [], []
    if fails and not a.no_retry:
        # 再跑一次：逐个单独一个进程（同一个二进制里的别的用例不在旁边）
        def again(f):
            name, exe, t = f
            p = subprocess.run([exe, '--exact', '--test-threads', '1', t], cwd=ROOT, env=env, capture_output=True, text=True, timeout=a.timeout)
            return f, p.returncode == 0 and ' 1 passed' in p.stdout
        with cf.ThreadPoolExecutor(min(workers, len(fails))) as pool:
            for f, ok in pool.map(again, fails):
                (flaky if ok else hard).append(f)
    else:
        hard = fails
    # 耗时表：只有一个用例的片记到用例上；整个二进制的总耗时累加各片
    tot = {}
    for r in results:
        e = timings.setdefault(r['name'], {'total': 0.0, 'tests': {}})
        tot[r['name']] = tot.get(r['name'], 0.0) + r['secs']
        share = r['secs'] / max(len(r['tests']), 1)
        for t in r['tests']:
            old = e['tests'].get(t)
            e['tests'][t] = share if old is None else old * 0.5 + share * 0.5
    for n, s in tot.items():
        timings[n]['total'] = s
    with open(TIMINGS, 'w', encoding='utf-8') as f:
        json.dump(timings, f, ensure_ascii=False, indent=0)
    slow = sorted(results, key=lambda r: -r['secs'])[:5]
    log('最慢的几片：' + '，'.join(f"{r['name']}×{len(r['tests'])} {r['secs']:.1f}s" for r in slow))
    log(f"进程累计 {sum(r['secs'] for r in results):.0f}s，并发跑完 {run_s:.0f}s，总用时 {time.time() - t_all:.0f}s")
    for name, _, t in flaky:
        log(f"  ⚠ 偶发（再跑过了）：{name} {t}")
    for name, _, t in hard:
        log(f"  ❌ 失败：{name} {t}")
    log(f"测试：通过 {passed}，失败 {len(hard)}" + (f"，偶发 {len(flaky)}" if flaky else ''))
    return 1 if hard else 0


if __name__ == '__main__':
    sys.exit(main())
