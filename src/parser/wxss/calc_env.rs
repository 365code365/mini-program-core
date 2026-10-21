//! `calc()` 的静态求值，以及 `env(safe-area-inset-*)` / `constant(...)` 的取值。
//!
//! `eval_calc` 从 `parser.rs` 原样搬来（那个文件已经 570 行）；新增的只有安全区：
//! 宿主（SDK / Flutter 插件）告诉引擎刘海、底部横条占了多少，样式里的
//! `env(safe-area-inset-*)` 才能算出真值。
//!
//! ## 为什么是 thread_local，而不是解析器参数
//! `WxssParser` 的调用点散在页面加载、自定义组件、tabBar 组件好几处，而安全区
//! 只有宿主知道。引擎实例只在一条线程上跑（QuickJS 的约定），所以「解析前把当前
//! 实例的安全区放进本线程」既不用改每个调用点，也不会让并行跑的测试互相串值
//! （进程级 static 会）。**没设置时全是 0**：桌面窗体、无头快照、所有基线都走这条，
//! 与从前逐字节一致。
//!
//! ## 取值时机
//! 在**解析期**求值（与 rpx 的换算一样是常量折叠）。安全区变了（转屏）要重新解析
//! 样式表 —— `MiniEngine::set_safe_area` 会把页面栈上的样式表重解析一遍。
use super::{LengthUnit, StyleValue, WxssParser};
use std::cell::Cell;

/// 解析期可见的环境量（逻辑像素）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvContext {
    /// 上、右、下、左
    pub insets: [f32; 4],
    /// 屏幕逻辑宽：`calc(164rpx + env(...))` 要把安全区折算成 rpx 才能合成一个值
    pub screen_width: f32,
    /// 屏幕逻辑高（`@media (min-height)` / `orientation`，见 media.rs）
    pub screen_height: f32,
    /// 深色模式（`@media (prefers-color-scheme: dark)`）。SDK 取创建选项 `systemInfo.theme`，
    /// 桌面窗体与无头取环境变量 `MINI_THEME=dark`；都没给就是浅色
    pub dark: bool,
}

impl Default for EnvContext {
    fn default() -> Self {
        Self {
            insets: [0.0; 4],
            screen_width: 375.0,
            screen_height: 667.0,
            dark: std::env::var("MINI_THEME").map(|v| v.trim().eq_ignore_ascii_case("dark")).unwrap_or(false),
        }
    }
}

thread_local! {
    static ENV: Cell<EnvContext> = Cell::new(EnvContext::default());
}

/// 设定本线程之后解析的样式表看到的环境量
pub fn set_env_context(ctx: EnvContext) {
    ENV.with(|c| c.set(ctx));
}

/// 本线程当前的环境量
pub fn env_context() -> EnvContext {
    ENV.with(|c| c.get())
}

/// `env(safe-area-inset-bottom)` / `constant(safe-area-inset-bottom)` → 逻辑像素。
/// 不是这两种写法返回 None；认不出的变量名按 0（与浏览器对未知 env 名的处理一致）。
pub(super) fn env_inset_px(tok: &str) -> Option<f32> {
    let tok = tok.trim();
    let inner = tok
        .strip_prefix("env(")
        .or_else(|| tok.strip_prefix("constant("))?
        .strip_suffix(')')?;
    let name = inner.split(',').next().unwrap_or("").trim();
    let i = env_context().insets;
    Some(match name {
        "safe-area-inset-top" => i[0],
        "safe-area-inset-right" => i[1],
        "safe-area-inset-bottom" => i[2],
        "safe-area-inset-left" => i[3],
        _ => 0.0,
    })
}

impl WxssParser {
    /// 静态求值 calc()：仅支持同单位（或无单位）的加减法。
    /// 混合单位（如 100% - 20px）无法静态解析，返回 None（由调用方回退为字符串）。
    ///
    /// `env(safe-area-inset-*)` 单独累加，最后折算进结果的单位：rpx 按屏宽换算、
    /// px 直接加；其它单位（% / vw）没法在解析期合并，安全区非 0 时放弃（回退字符串）。
    /// 安全区为 0 时与从前完全一样（env 记 0、不参与单位判定）。
    pub(super) fn eval_calc(value: &str) -> Option<StyleValue> {
        let inner = value.trim().strip_prefix("calc(")?.strip_suffix(')')?.trim();
        let tokens: Vec<&str> = inner.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }

        let mut acc: Option<f32> = None;
        let mut unit: Option<LengthUnit> = None;
        let mut op = '+';
        let mut env_px = 0.0f32;

        for (i, tok) in tokens.iter().enumerate() {
            if i % 2 == 1 {
                // 运算符位置：仅支持 + / -
                if *tok == "+" || *tok == "-" {
                    op = tok.chars().next().unwrap();
                } else {
                    return None; // 乘除或混合暂不支持
                }
                continue;
            }
            // 操作数
            let (n, u) = if let Some((n, u)) = Self::parse_length(tok) {
                (n, Some(u))
            } else if let Ok(n) = (*tok).parse::<f32>() {
                (n, None)
            } else if tok.starts_with("env(") {
                // `env(safe-area-inset-*)`：宿主没给安全区时记 0（渲染视口按无刘海机型算，
                // 对比用的 Chrome 视口同样返回 0）。
                //
                // 从前整条 calc 因为这个 token 解析失败而被**丢弃**，属性等于没写 ——
                // `height: calc(164rpx + env(safe-area-inset-bottom))` 这种写法
                // （uni-app 产物里自定义 tabBar 的标准高度）在浏览器里算出 82px，
                // 在我们这里退成 auto，高度全靠内容撑。
                let px = env_inset_px(tok).unwrap_or(0.0);
                env_px += if op == '-' { -px } else { px };
                (0.0, None)
            } else {
                return None;
            };
            if let Some(u) = u {
                match unit {
                    None => unit = Some(u),
                    Some(existing) if existing != u => return None, // 单位不一致
                    _ => {}
                }
            }
            acc = Some(match acc {
                None => n,
                Some(a) => {
                    if op == '+' {
                        a + n
                    } else {
                        a - n
                    }
                }
            });
        }

        let a = acc?;
        if env_px != 0.0 {
            let sw = env_context().screen_width.max(1.0);
            return match unit {
                Some(LengthUnit::Rpx) => Some(StyleValue::Length(a + env_px * 750.0 / sw, LengthUnit::Rpx)),
                Some(LengthUnit::Px) | None => Some(StyleValue::Length(a + env_px, LengthUnit::Px)),
                _ => None,
            };
        }
        Some(match unit {
            Some(u) => StyleValue::Length(a, u),
            None => StyleValue::Number(a),
        })
    }

    /// 单独写的 `env(safe-area-inset-*)`（如 `padding-top: env(safe-area-inset-top)`）。
    ///
    /// 只在安全区**非 0** 时给出长度；为 0 时返回 None，让调用方照旧把它当字符串 ——
    /// 这条路径从前就是字符串，没设安全区的宿主（含全部基线）因此一个像素都不变。
    pub(super) fn eval_env(value: &str) -> Option<StyleValue> {
        let px = env_inset_px(value)?;
        (px != 0.0).then_some(StyleValue::Length(px, LengthUnit::Px))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_env<T>(insets: [f32; 4], f: impl FnOnce() -> T) -> T {
        set_env_context(EnvContext { insets, screen_width: 375.0, ..Default::default() });
        let r = f();
        set_env_context(EnvContext::default());
        r
    }

    #[test]
    fn 没设安全区时_env_记_0_且不改单位() {
        match WxssParser::eval_calc("calc(164rpx + env(safe-area-inset-bottom))") {
            Some(StyleValue::Length(v, LengthUnit::Rpx)) => assert_eq!(v, 164.0),
            other => panic!("{other:?}"),
        }
        assert!(WxssParser::eval_env("env(safe-area-inset-top)").is_none(), "0 时保持旧的字符串路径");
    }

    #[test]
    fn 安全区折算进_rpx() {
        // 屏宽 375：34px = 68rpx
        let v = with_env([0.0, 0.0, 34.0, 0.0], || {
            WxssParser::eval_calc("calc(164rpx + env(safe-area-inset-bottom))")
        });
        match v {
            Some(StyleValue::Length(v, LengthUnit::Rpx)) => assert!((v - 232.0).abs() < 1e-3, "{v}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn 安全区加进_px_与无单位() {
        let (a, b, c) = with_env([47.0, 0.0, 0.0, 0.0], || {
            (
                WxssParser::eval_calc("calc(44px + env(safe-area-inset-top))"),
                WxssParser::eval_calc("calc(0 + env(safe-area-inset-top))"),
                WxssParser::eval_env("constant(safe-area-inset-top)"),
            )
        });
        assert!(matches!(a, Some(StyleValue::Length(v, LengthUnit::Px)) if (v - 91.0).abs() < 1e-3));
        assert!(matches!(b, Some(StyleValue::Length(v, LengthUnit::Px)) if (v - 47.0).abs() < 1e-3));
        assert!(matches!(c, Some(StyleValue::Length(v, LengthUnit::Px)) if (v - 47.0).abs() < 1e-3));
    }

    #[test]
    fn 百分比不能与非零安全区合并() {
        let v = with_env([0.0, 0.0, 34.0, 0.0], || WxssParser::eval_calc("calc(10% + env(safe-area-inset-bottom))"));
        assert!(v.is_none(), "解析期没法把 % 与 px 合成一个值");
    }

    #[test]
    fn 减号作用在安全区上() {
        let v = with_env([0.0, 0.0, 34.0, 0.0], || WxssParser::eval_calc("calc(100px - env(safe-area-inset-bottom))"));
        assert!(matches!(v, Some(StyleValue::Length(v, LengthUnit::Px)) if (v - 66.0).abs() < 1e-3));
    }
}
