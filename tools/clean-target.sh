#!/usr/bin/env bash
# 清理 target/：测试产物会无限堆积，实测能涨到 26GB。
#
#   bash tools/clean-target.sh          # 清测试产物 + debug 构建（保留 release，不用重编）
#   bash tools/clean-target.sh --targets # 再清交叉编译目标目录（iOS/Android，~6GB）
#   bash tools/clean-target.sh --all    # 连 cargo 构建缓存一起清（下次要全量重编）
#   bash tools/clean-target.sh --dry    # 只看会删什么
#
# 为什么要专门写个脚本：截图/对比工具都往 `target/<随便起的名字>` 里写
# （`target/_ns5`、`target/wc25`、`target/tf19_cmp`…），跑一轮回归就多出几十个目录，
# 每个十几 MB。基线目录（`fin2_sample` 等）也在里面，但它们**一条命令就能重新生成**
# （见 README 的「测试与回归」），所以一并清掉，不必长期占着。
set -eu
cd "$(dirname "$0")/.."
[ -d target ] || { echo "没有 target/，无需清理"; exit 0; }

MODE="${1:-}"
before=$(du -sk target | awk '{print $1}')

# cargo 自己管的东西：保留（删了要重编）
keep_dir() {
    case "$1" in
        debug|release|package|tmp|doc|CACHEDIR.TAG|.rustc_info.json|.cargo-lock) return 0 ;;
        # 交叉编译目标：aarch64-apple-ios / aarch64-linux-android / x86_64-pc-windows-msvc …
        *-apple-*|*-linux-*|*-pc-windows-*|*-unknown-*) return 0 ;;
        *) return 1 ;;
    esac
}

victims=()
for p in target/* target/.[!.]*; do
    [ -e "$p" ] || continue
    name="$(basename "$p")"
    keep_dir "$name" && continue
    victims+=("$p")
done

if [ "$MODE" = "--dry" ]; then
    printf '%s\n' "${victims[@]:-（无）}"
    echo "共 ${#victims[@]} 项测试产物"
    exit 0
fi

[ ${#victims[@]} -gt 0 ] && rm -rf "${victims[@]}"
echo "🧹 清掉 ${#victims[@]} 项测试产物"

# debug 目录没人用（所有脚本都跑 --release），直接删
if [ -d target/debug ]; then
    rm -rf target/debug
    echo "🧹 清掉 target/debug（所有脚本都用 --release）"
fi

# 交叉编译目标目录：移动端库不常编，产物已经拷进 sdk/，这里的中间件可以放心删
if [ "$MODE" = "--targets" ] || [ "$MODE" = "--all" ]; then
    # 含 *-apple-darwin：scripts/build-macos.sh 出通用库时按 triple 分目录编，
    # 各约 800MB，而宿主构建走的是 target/release，删掉不影响日常开发
    for d in target/*-apple-ios* target/*-apple-darwin* target/*-linux-android* \
             target/*-linux-androideabi target/*-pc-windows-* target/*-unknown-*; do
        [ -d "$d" ] && rm -rf "$d" && echo "🧹 清掉 $d"
    done
fi

if [ "$MODE" = "--all" ]; then
    cargo clean
    echo "🧹 cargo clean 完成（下次构建是全量）"
fi

after=$(du -sk target 2>/dev/null | awk '{print $1}' || echo 0)
awk -v b="$before" -v a="$after" 'BEGIN{printf "target/ %.1fGB -> %.1fGB（释放 %.1fGB）\n", b/1048576, a/1048576, (b-a)/1048576}'
