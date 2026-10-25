//! 模板展开时记下的绑定信息：`refresher-triggered` 绑定了哪些数据字段。

/// 模板展开时给 scroll-view 记下的「`refresher-triggered` 绑定了哪些字段」（逗号分隔）。
/// 下拉触发刷新后，页面 setData 了这些字段（哪怕值没变，一直是 false）才算把刷新状态设回去 ——
/// 微信的属性设置就是这个语义，只比较值的话「一直是 false」的写法永远收不起刷新指示器
pub const REFRESHER_KEYS_ATTR: &str = "__rt_keys";

/// 模板展开时给 scroll-view 记下的「`scroll-into-view` 绑定了哪些字段」（逗号分隔）。
/// `setData({intoView: ''}); setData({intoView: id})` 是「再滚一次到同一处」的常见写法：两次 setData
/// 落在同一帧，渲染时只看得到最终值（与上次相同），靠这些字段「中途变过」才知道要再滚（host/scroll_view.rs）
pub const INTO_VIEW_KEYS_ATTR: &str = "__siv_keys";

/// `refresher-triggered="{{expr}}"` 里的根字段名（`{{a.b || c}}` → `a`, `c`）。
/// 模板展开时调，结果写进 [`REFRESHER_KEYS_ATTR`]
pub fn binding_roots(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = raw;
    while let Some(i) = rest.find("{{") {
        rest = &rest[i + 2..];
        let end = rest.find("}}").unwrap_or(rest.len());
        let expr = &rest[..end];
        let bytes = expr.as_bytes();
        let mut j = 0;
        while j < bytes.len() {
            let c = bytes[j] as char;
            if c == '\'' || c == '"' {
                // 跳过字符串字面量
                j += 1;
                while j < bytes.len() && bytes[j] as char != c {
                    j += 1;
                }
                j += 1;
                continue;
            }
            if c.is_ascii_alphabetic() || c == '_' || c == '$' {
                let s = j;
                while j < bytes.len() && ((bytes[j] as char).is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'$') {
                    j += 1;
                }
                let word = &expr[s..j];
                let after_dot = expr[..s].trim_end().ends_with('.');
                if !after_dot && !matches!(word, "true" | "false" | "null" | "undefined") && !out.iter().any(|w| w == word) {
                    out.push(word.to_string());
                }
                continue;
            }
            j += 1;
        }
        rest = &rest[end.min(rest.len())..];
    }
    out
}
