#!/usr/bin/env python3
"""删掉 target/ 里**已经过期、cargo 不会再用**的构建中间件，不碰还活着的那份。

    python3 tools/prune-target.py            # 剪 target/release 与 target/<triple>/release
    python3 tools/prune-target.py --dry      # 只列会删什么
    python3 tools/prune-target.py --quiet    # 只打一行汇总（给构建脚本收尾用）

为什么要有它：cargo 从不回收旧产物。改一次特性 / profile 参数（`CARGO_PROFILE_RELEASE_LTO=thin`
也算）、升一次依赖，同一个目标就多一份带新哈希的产物，旧的永远留着。2026-10-26 实测
target/release 43GB：deps 17GB（每个集成测试 7 份、`mini_render` 单测二进制 10 份，一份 75MB）、
incremental 19GB（`mini_render` 14 份增量缓存，一份 0.7~1.8GB），真正在用的不到三成。

判据（全部保守，宁可少删）：
- 工程自己的目标（lib / bin / test / example）：每个名字**永远留最新一份**；其余份如果比
  工程源码（src/、Cargo.toml、Cargo.lock、build.rs）的最后修改还旧 —— 它们都链着引擎库，
  下次构建无论如何都要重编，删了零成本 —— 或者超过 7 天没动过，就删。
- 第三方依赖：只删 dep-info 里引用的 `<crate>-<version>` 已不在 Cargo.lock 里的（升级前的旧版本）。
  特性组合不同的同名依赖是合法并存的，不动。
- 任何一份 dep-info 里列的源文件已经不存在（删掉的测试 / 示例），删。
- incremental/：每个名字留最新两份（lib 与它的单测各一份），外加源码最后修改之后动过的；
  名字既不是工程目标、deps/ 里也没有活产物的整份删（vendor/ 下的 path 依赖也走增量，要留）。增量缓存删错了只是下次慢一点，不会编错。

并发：别的会话可能正在编。每个 profile 目录先非阻塞地拿 cargo 自己的 `.cargo-lock`，
拿不到（有人在编）就跳过那个目录，不会删掉正在写的文件。
"""
import fcntl
import os
import re
import shutil
import sys
import time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
TARGET = os.path.join(ROOT, "target")
STALE_DAYS = 7
KEEP_INCREMENTAL = 2
# lib<name>-<16位哈希>.rlib / <name>-<哈希> / <name>-<哈希>.d / <name>-<哈希>.<cgu>.rcgu.o …
ART_RE = re.compile(r"^(?:lib)?(?P<name>[A-Za-z0-9_]+?)-(?P<hash>[0-9a-f]{16})(?:\..*)?$")
REGISTRY_RE = re.compile(r"/registry/src/[^/]+/(?P<pkg>[A-Za-z0-9_.+-]+?-\d+\.\d+\.\d+[^/]*)/")


def workspace_targets():
    """工程里所有目标名（`-` 换成 `_`，与 deps/ 里的文件名一致）。

    不用 `cargo metadata`：它会等 target/ 的构建锁，别的会话正在编时就一直挂着
    （2026-10-26 实测：持锁时调它，整条回归收尾卡死）。这里直接读 Cargo.toml 与目录，
    宁可多认几个名字（多留），不会少认。
    """
    try:
        with open(os.path.join(ROOT, "Cargo.toml"), encoding="utf-8") as f:
            toml = f.read()
    except OSError:
        return None
    names = set()
    pkg = re.search(r'^\[package\][^\[]*?^name\s*=\s*"([^"]+)"', toml, re.M | re.S)
    if pkg:
        names.add(pkg.group(1))
    # [lib] / [[bin]] / [[example]] / [[test]] / [[bench]] 里显式写的 name
    for m in re.finditer(r'^\[\[?(?:lib|bin|example|test|bench)\]?\]\s*\n(?:[^\[]*?\n)?name\s*=\s*"([^"]+)"',
                         toml, re.M):
        names.add(m.group(1))
    # 自动发现的目标：src/bin/*.rs、tests/*.rs、examples/*.rs、benches/*.rs 与带 main.rs 的子目录
    for sub in ("src/bin", "tests", "examples", "benches"):
        d = os.path.join(ROOT, sub)
        if not os.path.isdir(d):
            continue
        for fn in os.listdir(d):
            if fn.endswith(".rs"):
                names.add(fn[:-3])
            elif os.path.isfile(os.path.join(d, fn, "main.rs")):
                names.add(fn)
    return {n.replace("-", "_") for n in names}


def locked_packages():
    """Cargo.lock 里的 `<name>-<version>` 集合。"""
    pkgs, name = set(), None
    try:
        with open(os.path.join(ROOT, "Cargo.lock"), encoding="utf-8") as f:
            for line in f:
                if line.startswith("name = "):
                    name = line.split('"')[1]
                elif line.startswith("version = ") and name:
                    pkgs.add(name + "-" + line.split('"')[1])
                    name = None
    except OSError:
        return None
    return pkgs


def source_watermark():
    """工程源码最后一次修改的时刻：比它旧的工程产物下次一定重编。"""
    newest = 0.0
    for fixed in ("Cargo.toml", "Cargo.lock", "build.rs"):
        p = os.path.join(ROOT, fixed)
        if os.path.exists(p):
            newest = max(newest, os.path.getmtime(p))
    for dirpath, _, files in os.walk(os.path.join(ROOT, "src")):
        for fn in files:
            newest = max(newest, os.path.getmtime(os.path.join(dirpath, fn)))
    return newest


def depinfo_sources(path):
    """dep-info（Makefile 语法）里第一条规则的全部依赖文件。"""
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            text = f.read()
    except OSError:
        return []
    first = text.split("\n\n", 1)[0].replace("\\\n", " ")
    if ": " not in first:
        return []
    deps = first.split(": ", 1)[1]
    # 路径里的空格转义成 `\ `
    return [p.replace("\\ ", " ") for p in re.split(r"(?<!\\) +", deps.strip()) if p]


def size_of(path):
    if os.path.islink(path) or not os.path.isdir(path):
        try:
            return os.lstat(path).st_blocks * 512
        except OSError:
            return 0
    total = 0
    for dirpath, _, files in os.walk(path):
        for fn in files:
            try:
                total += os.lstat(os.path.join(dirpath, fn)).st_blocks * 512
            except OSError:
                pass
    return total


class Pruner:
    def __init__(self, dry, ws, lock, watermark):
        self.dry, self.ws, self.lock, self.watermark = dry, ws, lock, watermark
        self.now = time.time()
        self.freed = 0
        self.removed = 0
        self.lines = []
        # 剪完 deps/ 之后还留着产物的名字：incremental/ 里这些名字的缓存不能整份删
        # （path 依赖如 vendor/fontdue 也走增量编译，但它不是工程目标）
        self.live_names = set()

    def remove(self, path, why):
        sz = size_of(path)
        self.freed += sz
        self.removed += 1
        self.lines.append(f"  {sz / 1048576:8.1f}MB  {os.path.relpath(path, ROOT)}  ({why})")
        if self.dry:
            return
        if os.path.isdir(path) and not os.path.islink(path):
            shutil.rmtree(path, ignore_errors=True)
        else:
            try:
                os.remove(path)
            except OSError:
                pass

    def group_dir(self, d):
        """按哈希分组：一个哈希就是一个编译单元的全部产物。"""
        groups = {}
        for fn in os.listdir(d):
            m = ART_RE.match(fn)
            if not m:
                continue
            g = groups.setdefault(m.group("hash"), {"files": [], "name": None, "mtime": 0.0, "dep": None})
            p = os.path.join(d, fn)
            g["files"].append(p)
            try:
                g["mtime"] = max(g["mtime"], os.lstat(p).st_mtime)
            except OSError:
                pass
            if fn.endswith(".d") and fn.count(".") == 1:
                # dep-info 的文件名不带 `lib` 前缀，名字以它为准（`liblibc-…rlib` 对应 `libc-….d`）
                g["name"] = fn[: -len(m.group("hash")) - 3]
                g["dep"] = p
            elif g["name"] is None:
                g["name"] = m.group("name")
        return groups

    def verdict(self, g, newest_of_name):
        """返回删除理由；None 表示留着。"""
        is_ws = self.ws is not None and g["name"] in self.ws
        srcs = depinfo_sources(g["dep"]) if g["dep"] else []
        # 源文件没了（删掉的测试 / 示例 / 搬走的工程）：这份产物再也不会被用到
        # 工程自己的源文件在 dep-info 里是相对工作区根目录的路径
        for s in srcs:
            if not os.path.exists(s if os.path.isabs(s) else os.path.join(ROOT, s)):
                return "源文件已不存在"
        if is_ws:
            if g is newest_of_name:
                return None
            if g["mtime"] < self.watermark:
                return "比源码旧，下次必重编"
            if self.now - g["mtime"] > STALE_DAYS * 86400:
                return f"{STALE_DAYS} 天未用的旧变体"
            return None
        if self.lock:
            for s in srcs:
                m = REGISTRY_RE.search(s)
                if m:
                    return None if m.group("pkg") in self.lock else "依赖版本已不在 Cargo.lock"
        return None

    def prune_artifacts(self, d):
        if not os.path.isdir(d):
            return
        groups = self.group_dir(d)
        newest = {}
        for g in groups.values():
            cur = newest.get(g["name"])
            if cur is None or g["mtime"] > cur["mtime"]:
                newest[g["name"]] = g
        for h, g in sorted(groups.items(), key=lambda kv: kv[1]["mtime"]):
            why = self.verdict(g, newest[g["name"]])
            if not why:
                self.live_names.add(g["name"])
                continue
            # 一组只列一行（列最大的那个文件），其余文件计入释放量
            files = sorted(g["files"], key=size_of, reverse=True)
            self.remove(files[0], why)
            for p in files[1:]:
                self.remove_quiet(p)

    def remove_quiet(self, path):
        """同一组的其余文件：计入释放量，不单独列一行。"""
        self.freed += size_of(path)
        if not self.dry:
            if os.path.isdir(path) and not os.path.islink(path):
                shutil.rmtree(path, ignore_errors=True)
            else:
                try:
                    os.remove(path)
                except OSError:
                    pass

    def prune_incremental(self, d):
        if not os.path.isdir(d):
            return
        by_name = {}
        for fn in os.listdir(d):
            name, _, _ = fn.rpartition("-")
            if not name:
                continue
            p = os.path.join(d, fn)
            by_name.setdefault(name, []).append((os.path.getmtime(p), p))
        for name, dirs in by_name.items():
            dirs.sort(reverse=True)
            if self.ws is not None and name not in self.ws and name not in self.live_names:
                for _, p in dirs:
                    self.remove(p, "已不是工程目标")
                continue
            for i, (mt, p) in enumerate(dirs):
                if i < KEEP_INCREMENTAL or mt >= self.watermark:
                    continue
                self.remove(p, f"第 {i + 1} 新的增量缓存")

    def prune_profile(self, prof):
        lock_path = os.path.join(prof, ".cargo-lock")
        fd = None
        if os.path.exists(lock_path):
            fd = os.open(lock_path, os.O_RDWR)
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                os.close(fd)
                self.lines.append(f"  跳过 {os.path.relpath(prof, ROOT)}：别的 cargo 正在用它")
                return
        self.live_names = set()
        try:
            self.prune_artifacts(os.path.join(prof, "deps"))
            self.prune_artifacts(os.path.join(prof, "examples"))
            self.prune_incremental(os.path.join(prof, "incremental"))
        finally:
            if fd is not None:
                fcntl.flock(fd, fcntl.LOCK_UN)
                os.close(fd)


def profile_dirs():
    out = []
    if not os.path.isdir(TARGET):
        return out
    for top in sorted(os.listdir(TARGET)):
        base = os.path.join(TARGET, top)
        if top == "release":
            out.append(base)
        elif "-" in top and os.path.isdir(os.path.join(base, "release")):
            # 交叉编译目标 target/<triple>/release
            out.append(os.path.join(base, "release"))
    return out


def main():
    args = set(sys.argv[1:])
    unknown = args - {"--dry", "--quiet"}
    if unknown:
        print(f"未知参数 {' '.join(sorted(unknown))}（可用：--dry --quiet）")
        return 2
    dry, quiet = "--dry" in args, "--quiet" in args
    ws = workspace_targets()
    if ws is None:
        print("⚠  读不到 Cargo.toml：只按「源文件已不存在」剪，不动工程目标的旧变体")
    pr = Pruner(dry, ws, locked_packages(), source_watermark())
    for prof in profile_dirs():
        pr.prune_profile(prof)
    if not quiet:
        for line in pr.lines:
            print(line)
    verb = "可释放" if dry else "释放"
    print(f"✂  过期构建产物 {pr.removed} 组，{verb} {pr.freed / 1073741824:.1f}GB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
