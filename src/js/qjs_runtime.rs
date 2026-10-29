//! 建 QuickJS 运行时（third_party/rquickjs-sys 里那份改过的 QuickJS，改了什么见 PATCHES.md）。
//!
//! 两样东西在 C 里、默认开着，各有一个对照开关（与 `MINI_FPS` 这些同等对待，写在 README）：
//! - **小块池**（quickjs.c 的 `js_pool_alloc`）：≤ 512 字节的分配进运行时自带的空闲链表，不经宿主分配器、也不再每次问
//!   `malloc_usable_size`。它替掉了原来 Rust 侧的分档池（rquickjs 的 `Allocator`）：那一层每次分配还要穿过 rquickjs 的包装与
//!   dyn 分发，真机上分配相关合计仍占游戏线程约 3.6%。`MINI_JS_SYSTEM_ALLOC=1` 关掉，退回 QuickJS 默认分配器（系统 malloc）。
//! - **属性读写的内联缓存**（`OP_get_field_ic` 等）：`MINI_JS_NO_IC=1` 关掉（之后第一次被调用的函数都不改写），对照用。
use rquickjs::{qjs, Context, Runtime};
use std::sync::Once;

extern "C" {
    fn JS_SetSmallBlockPoolDefault(enabled: bool);
    fn JS_SetInlineCacheEnabled(rt: *mut qjs::JSRuntime, enabled: bool);
}

fn flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "1")
}

/// 建一个运行时。池开不开是进程级的默认值，必须在第一个运行时之前定下（池里每一块都带头，中途换不了）
pub(crate) fn new_runtime() -> rquickjs::Result<Runtime> {
    static POOL: Once = Once::new();
    // SAFETY：只改一个进程级布尔；Once 保证在任何运行时建好之前、只设一次
    POOL.call_once(|| unsafe { JS_SetSmallBlockPoolDefault(!flag("MINI_JS_SYSTEM_ALLOC")) });
    Runtime::new()
}

/// 运行时建好之后按开关关掉内联缓存（默认开着，什么都不做）
pub(crate) fn apply_flags(context: &Context) {
    if flag("MINI_JS_NO_IC") {
        // SAFETY：ctx 有效；JS_GetRuntime 取的是它所属的运行时，只设一个布尔
        context.with(|ctx| unsafe { JS_SetInlineCacheEnabled(qjs::JS_GetRuntime(ctx.as_raw().as_ptr()), false) });
    }
}

#[cfg(test)]
mod tests {
    /// 小块池 + 内联缓存下跑一段分配与属性读写都很密的脚本：结果对，GC 后运行时能干净地销毁
    #[test]
    fn 池与内联缓存下跑得对() {
        let rt = super::new_runtime().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();
        super::apply_flags(&ctx);
        let n: i32 = ctx.with(|c| {
            c.eval(
                "function P(){} P.prototype.f = function(){ return this.a; };
                 let s = 0;
                 for (let i = 0; i < 20000; i++) { const o = new P(); o.a = i; const t = {a: i, b: [i, i + 1], c: 'x' + i}; s += o.f() - t.a + t.b[1] - i; }
                 P.prototype.f = function(){ return 0; };
                 s + new P().f()",
            )
            .unwrap()
        });
        assert_eq!(n, 20000);
        rt.run_gc();
    }

    /// 内联缓存 / 数组快路径的对抗用例：开着缓存与关掉缓存都要与原版 quickjs-ng 的输出逐行相同
    #[test]
    fn 对抗用例与原版输出相同() {
        let want = include_str!("qjs_ic_cases.out").trim_end();
        for ic in [true, false] {
            let rt = super::new_runtime().unwrap();
            let ctx = rquickjs::Context::full(&rt).unwrap();
            if !ic {
                // SAFETY：ctx 有效，只设一个布尔
                ctx.with(|c| unsafe { super::JS_SetInlineCacheEnabled(rquickjs::qjs::JS_GetRuntime(c.as_raw().as_ptr()), false) });
            }
            // 非严格（用例里有 with、给未声明的变量赋值）
            let got: String = ctx.with(|c| {
                let mut o = rquickjs::context::EvalOptions::default();
                o.strict = false;
                c.eval_with_options(include_str!("qjs_ic_cases.js"), o).unwrap()
            });
            for (i, (g, w)) in got.lines().zip(want.lines()).enumerate() {
                assert_eq!(g, w, "第 {} 行（内联缓存 {}）", i + 1, if ic { "开" } else { "关" });
            }
            assert_eq!(got.lines().count(), want.lines().count(), "行数（内联缓存 {}）", if ic { "开" } else { "关" });
            rt.run_gc();
        }
    }
}
