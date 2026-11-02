//! `wx` 的中立命名空间：`nb` / `gb` / `mb` 与「按函数名认出 `xx.函数名`」。
//!
//! ## 为什么要有
//! 小程序代码只认 `wx.*` 的话，用这套引擎的产品就只能把 `wx` 这个名字原样暴露给开发者。
//! 为了不在命名上与他人产生纠纷，同一套 API 也要能以中立的名字调用：`nb.request(...)`、
//! `gb.showToast(...)`、`mb.navigateTo(...)` 与 `wx.*` 完全等价。
//!
//! 别名与 `wx` 是**同一个对象**（`nb === wx`），不是拷贝：之后 `wx.xxx = ...` 的补丁、
//! api-probe 的「未实现」提示、宿主在 hostApis 里点名换上的真实现，所有名字上同时生效。
//! 显式别名与判定逻辑在 `prelude/api-alias.js`（三处共用：QuickJS、小游戏高性能模式的页面、H5 导出）。
//!
//! ## 按函数名认（模糊匹配）
//! 代码里写的是没人声明过的 `xx.request(...)`、`xx.showToast(...)` 时，把 `xx` 也指向 `wx`。
//! 分两步，各做各擅长的：
//! - **这里（Rust）**：对每份源码做一遍便宜的词法扫描，找出「标识符.成员」里成员名是 wx API 的，
//!   同一个前缀命中 ≥ 2 个不同的 API 名才算候选，生成一行 `__wxAliasGuess({...})`；
//! - **JS（`__wxAliasGuess`）**：再按运行时真有的函数复核一遍，且只占用**还没被定义**的全局名。
//!
//! 只命中 1 个不算：`api.request()` 这类自家封装太常见。命中 2 个的局部变量（`const api = require(...)`）
//! 即便被认成别名也无害：模块里的局部声明遮住全局，只有真正未声明的名字才会落到 `wx` 上。
//!
//! **别的平台的命名空间永远不认**（`my` / `tt` / `qq` / `swan` / `uni` / `Taro` …）：uni-app、Taro 的产物里
//! 有 `typeof my !== 'undefined'` 这类平台探测，把 `my` 指向 wx 会让框架以为自己跑在支付宝上，走错整条分支。
//!
//! ## 开关
//! - `MINI_API_ALIAS=foo,bar`：在默认的 nb / gb / mb 之外再装显式别名（QuickJS 运行时，见 `MiniAppApi::init_profile`）；
//! - `MINI_API_ALIAS_GUESS=0`：关掉按函数名认，只留显式别名。
//!
//! 扫描是近似的：跳过注释与字符串，不认正则字面量、不进模板字符串的 `${}`。漏认只是少一个别名，
//! 误认有 JS 侧复核兜底，所以不值得为它写一个真正的 JS 词法器。
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::OnceLock;

/// 同一个前缀至少命中几个不同的 API 名才算候选（与 api-alias.js 的 `MIN_HITS` 一致）
const MIN_HITS: usize = 2;

/// 绝不当成候选的前缀：关键字、常用全局、别的平台的命名空间（理由见文件头）。
/// JS 侧还会拒绝所有已定义的全局，这张表只是让输出保持为空、别给每页都多一行
const SKIP: &[&str] = &[
    "this", "super", "new", "typeof", "void", "delete", "in", "of", "return", "case", "await", "yield",
    "wx", "console", "Math", "JSON", "Object", "Array", "Promise", "Date", "String", "Number", "Boolean",
    "Reflect", "Symbol", "Error", "window", "document", "navigator", "globalThis", "global", "self",
    "GameGlobal", "module", "exports", "require",
    "my", "tt", "qq", "swan", "ks", "jd", "dd", "xhs", "qh", "has", "uni", "Taro", "plus",
];

/// 显式别名的默认名单（与 api-alias.js 里的一致）
pub const DEFAULT_ALIASES: &[&str] = &["nb", "gb", "mb"];

/// `MINI_API_ALIAS` 里额外点名的别名（逗号分隔，只收合法标识符）
pub fn extra_aliases() -> Vec<String> {
    std::env::var("MINI_API_ALIAS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| is_ident(s))
        .collect()
}

/// 装显式别名的那段 JS（没有额外点名时为空）
pub fn extra_alias_script() -> Option<String> {
    let names = extra_aliases();
    if names.is_empty() {
        return None;
    }
    let list = serde_json::to_string(&names).ok()?;
    Some(format!("typeof __wxAlias === 'function' && {list}.forEach(function (n) {{ __wxAlias(n); }});"))
}

fn guess_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("MINI_API_ALIAS_GUESS").map(|v| v != "0").unwrap_or(true))
}

/// 一组源码里按函数名认出来的候选，拼成一行 JS；没有候选（或开关关了）时为 `None`。
///
/// 结尾**不带换行**：H5 / 网页模式会把它拼在别的代码前面，不该挪动开发者源码的行号
pub fn guess_script<'a>(sources: impl IntoIterator<Item = &'a str>) -> Option<String> {
    if !guess_enabled() {
        return None;
    }
    let mut all: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for src in sources {
        for (k, v) in candidates(src) {
            all.entry(k).or_default().extend(v);
        }
    }
    all.retain(|_, v| v.len() >= MIN_HITS);
    if all.is_empty() {
        return None;
    }
    let json = serde_json::to_string(&all).ok()?;
    Some(format!("typeof __wxAliasGuess === 'function' && __wxAliasGuess({json});"))
}

/// 前置代码里 `wx.名字 = ...` 赋值过的全部名字：就是「wx 上有的 API」（只用来筛候选，JS 侧再复核）
fn api_names() -> &'static HashSet<&'static str> {
    static NAMES: OnceLock<HashSet<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut out = HashSet::new();
        for src in super::api::all_sources() {
            collect_assigned(src, &mut out);
        }
        out
    })
}

fn collect_assigned(src: &'static str, out: &mut HashSet<&'static str>) {
    let b = src.as_bytes();
    let mut from = 0;
    while let Some(p) = src[from..].find("wx.") {
        let s = from + p;
        from = s + 3;
        if s > 0 && is_ident_byte(b[s - 1]) {
            continue;
        }
        let e = ident_end(b, from);
        if e == from {
            continue;
        }
        let k = skip_blank(b, e);
        if b.get(k) == Some(&b'=') && b.get(k + 1) != Some(&b'=') {
            out.insert(&src[from..e]);
        }
        from = e;
    }
}

/// 一份源码里「前缀 → 命中的 API 名」（未按 MIN_HITS 过滤：多份源码要先合并再数）
fn candidates(src: &str) -> BTreeMap<&str, BTreeSet<&str>> {
    let api = api_names();
    let b = src.as_bytes();
    let n = b.len();
    let mut out: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut i = 0;
    // 上一个有意义的字节（不含空白、注释）：是 `.` 说明当前标识符是成员，不是前缀
    let mut prev = 0u8;
    while i < n {
        let c = b[i];
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            i = src[i..].find('\n').map_or(n, |p| i + p);
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i = src[i + 2..].find("*/").map_or(n, |p| i + 2 + p + 2);
        } else if c == b'\'' || c == b'"' || c == b'`' {
            i = skip_string(b, i);
            prev = c;
        } else if c.is_ascii_digit() {
            while i < n && (is_ident_byte(b[i]) || b[i] == b'.') {
                i += 1;
            }
            prev = b'0';
        } else if is_ident_start(c) {
            let s = i;
            i = ident_end(b, i);
            let member_of_something = prev == b'.';
            prev = b'a';
            if member_of_something {
                continue;
            }
            if let Some((ms, me)) = member_after(b, i) {
                let (name, member) = (&src[s..i], &src[ms..me]);
                if api.contains(member) && !SKIP.contains(&name) {
                    out.entry(name).or_default().insert(member);
                }
            }
        } else {
            if !c.is_ascii_whitespace() {
                prev = c;
            }
            i += 1;
        }
    }
    out
}

/// `i` 处起 `空白 [?] . 空白 标识符`：返回成员名的范围（`..` 展开、`.5` 这类不算）
fn member_after(b: &[u8], i: usize) -> Option<(usize, usize)> {
    let mut j = skip_blank(b, i);
    if b.get(j) == Some(&b'?') && b.get(j + 1) == Some(&b'.') {
        j += 1;
    }
    if b.get(j) != Some(&b'.') || b.get(j + 1) == Some(&b'.') {
        return None;
    }
    let ms = skip_blank(b, j + 1);
    if ms >= b.len() || !is_ident_start(b[ms]) {
        return None;
    }
    Some((ms, ident_end(b, ms)))
}

/// 跳过一个字符串字面量（含转义）；单 / 双引号遇到换行就停（未闭合的不拖着吃掉后面整份代码）
fn skip_string(b: &[u8], start: usize) -> usize {
    let q = b[start];
    let mut i = start + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            c if c == q => return i + 1,
            b'\n' if q != b'`' => return i,
            _ => i += 1,
        }
    }
    b.len()
}

fn skip_blank(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn ident_end(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && is_ident_byte(b[i]) {
        i += 1;
    }
    i
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'$'
}

fn is_ident_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

fn is_ident(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty() && is_ident_start(b[0]) && b.iter().all(|&c| is_ident_byte(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(src: &str) -> Vec<String> {
        let mut m = candidates(src);
        m.retain(|_, v| v.len() >= MIN_HITS);
        m.into_keys().map(String::from).collect()
    }

    #[test]
    fn 前置代码里赋值过的名字就是_api() {
        let n = api_names();
        for k in ["request", "showToast", "navigateTo", "setStorageSync", "canIUse"] {
            assert!(n.contains(k), "缺 {k}");
        }
        assert!(!n.contains("setData"), "setData 不是 wx 上的函数");
    }

    #[test]
    fn 命中两个不同_api_才算候选() {
        assert_eq!(keys("xx.request({}); xx.showToast({title:'a'})"), vec!["xx"]);
        assert!(keys("xx.request({}); xx.request({})").is_empty(), "同一个名字只算一次");
        assert!(keys("that.setData({}); that.data.a").is_empty());
    }

    #[test]
    fn 成员链_注释_字符串与平台名都不算() {
        assert!(keys("a.xx.request(); a.xx.showToast()").is_empty(), "xx 是成员，不是前缀");
        assert!(keys("// xx.request(); xx.showToast()\n/* yy.request(); yy.showToast() */").is_empty());
        assert!(keys("s = 'xx.request(); xx.showToast()'; t = `zz.request() zz.showToast()`").is_empty());
        assert!(keys("my.request(); my.showToast(); tt.request(); tt.showToast(); wx.request(); wx.showToast()").is_empty());
        assert_eq!(keys("nb ?. request(); nb\n  .showToast()"), vec!["nb"], "可选链与换行照认");
    }

    #[test]
    fn 多份源码合并后再数() {
        let s = guess_script(["xx.request()", "xx.showToast()"]).unwrap();
        assert!(s.contains("\"xx\":[\"request\",\"showToast\"]"), "{s}");
        assert!(!s.ends_with('\n'));
        assert!(guess_script(["var a = 1;"]).is_none());
    }
}
