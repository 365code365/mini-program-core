//! 样式落地的记忆表：同一张样式表、同一个元素上下文，`build_base_style` 的结果原样给回去。
//!
//! 为什么要：setData 之后整棵重建，页面上没变的几百个节点又要把选择器逐条匹配一遍、声明逐条落地。
//! 一加 PLY110「节点与滚动」页（simpleperf，引擎线程 12s）：建树占重建的一半，其中选择器匹配
//! （`compound_matches_desc` / `cascade_chain`）、声明落地（`build_base_style` 自身、排序）合计约四成。
//!
//! 结果只取决于这些输入（读过 `base_style.rs` / `property.rs` / `units.rs`，`ctx` 里只读这几样）：
//! - 样式表（[`SheetId`](crate::parser::wxss::SheetId)：建出来 / 克隆出来各是新编号，规则不再改）；
//! - 元素：标签、`id`、`class`、内联 `style`、`hover-class`（`[attr]` 选择器要看全部属性 —— 有这类规则的表不记）；
//! - 祖先链（每层的标签 / id / 类名 / 兄弟位置 / 按压态）、自己的兄弟位置；
//! - 继承来的文字样式、有没有定位祖先、屏宽高与缩放。
//! 内联 `style` 的取值在这里不碰全局状态（`env()` 只在样式表解析期求值）。
//!
//! 键是上面这些输入的完整字节串（拼在一块复用的缓冲区里，不分配），哈希只用来分桶；
//! 命中时逐字节比对整个键，所以拿回来的一定是同一组输入算出来的结果。
//!
//! `MINI_NO_STYLE_MEMO=1` 关掉（对照用）。
use super::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// 一张表最多记这么多条（每条一个 `Style` + `NodeStyle`，几百字节）；满了整表清空
const CAP: usize = 8192;
/// 最多同时留几张样式表的记录（每个页面 / tabBar 一张，换页时新表顶掉最旧的）
const SHEETS: usize = 6;

struct Entry {
    key: Box<[u8]>,
    val: (Style, NodeStyle),
}

#[derive(Default)]
struct Memo {
    /// (样式表编号, 键的哈希 → 条目)，最近用的在后面
    sheets: Vec<(u64, HashMap<u64, Entry>)>,
    /// 拼键用的缓冲区（每次清空复用）
    buf: Vec<u8>,
}

thread_local! {
    static MEMO: RefCell<Memo> = RefCell::new(Memo::default());
}

fn disabled() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var("MINI_NO_STYLE_MEMO").is_ok())
}

fn put_str(b: &mut Vec<u8>, s: &str) {
    b.extend_from_slice(&(s.len() as u32).to_le_bytes());
    b.extend_from_slice(s.as_bytes());
}

fn put_opt(b: &mut Vec<u8>, s: Option<&str>) {
    match s {
        Some(s) => {
            b.push(1);
            put_str(b, s);
        }
        None => b.push(0),
    }
}

fn put_f32(b: &mut Vec<u8>, v: f32) {
    b.extend_from_slice(&v.to_bits().to_le_bytes());
}

fn put_usize(b: &mut Vec<u8>, v: usize) {
    b.extend_from_slice(&(v as u64).to_le_bytes());
}

/// 把这次 `build_base_style` 的全部输入拼进 `b`
fn write_key(b: &mut Vec<u8>, node: &WxmlNode, ctx: &ComponentContext) {
    put_f32(b, ctx.scale_factor);
    put_f32(b, ctx.screen_width);
    put_f32(b, ctx.screen_height);
    put_str(b, &node.tag_name);
    put_opt(b, node.get_attr("id"));
    put_opt(b, node.get_attr("class"));
    put_opt(b, node.get_attr("style"));
    put_opt(b, node.get_attr("hover-class"));
    put_usize(b, ctx.sibling_index);
    put_usize(b, ctx.sibling_count);
    b.push(ctx.has_positioned_ancestor as u8);
    let t = &ctx.inherited;
    put_f32(b, t.font_size);
    match t.color {
        Some(c) => b.extend_from_slice(&[1, c.r, c.g, c.b, c.a]),
        None => b.push(0),
    }
    b.push(t.weight as u8);
    b.push(t.align as u8);
    match t.line_height {
        Some(v) => {
            b.push(1);
            put_f32(b, v);
        }
        None => b.push(0),
    }
    put_f32(b, t.letter_spacing);
    put_opt(b, t.font_family.as_deref());
    put_usize(b, ctx.ancestors.len());
    for a in &ctx.ancestors {
        put_str(b, &a.tag);
        put_opt(b, a.id.as_deref());
        put_usize(b, a.classes.len());
        for c in &a.classes {
            put_str(b, c);
        }
        put_usize(b, a.sibling_index);
        put_usize(b, a.sibling_count);
        b.push(a.pressed as u8);
        // 祖先的属性表：没有 `[attr]` 选择器时一律是空表（建树时就不带），有的话这张表根本不进记忆表
        put_usize(b, a.attrs.len());
    }
}

/// 记过就直接给，没记过 `compute` 一次再记下
pub(super) fn cached(
    node: &WxmlNode,
    ctx: &mut ComponentContext,
    compute: impl FnOnce(&WxmlNode, &mut ComponentContext) -> (Style, NodeStyle),
) -> (Style, NodeStyle) {
    let sheet = ctx.stylesheet;
    if disabled() || sheet.has_attr_selectors() {
        return compute(node, ctx);
    }
    let sheet_id = sheet.id.0;
    // 拼键、查表。查表时不拿着可变借用去算（compute 里不会再进来，但别赌）
    let (hash, key) = MEMO.with(|m| {
        let mut m = m.borrow_mut();
        let mut b = std::mem::take(&mut m.buf);
        b.clear();
        write_key(&mut b, node, ctx);
        let mut h = std::collections::hash_map::DefaultHasher::new();
        b.hash(&mut h);
        (h.finish(), b)
    });
    let hit = MEMO.with(|m| {
        let m = m.borrow();
        m.sheets
            .iter()
            .find(|(id, _)| *id == sheet_id)
            .and_then(|(_, t)| t.get(&hash))
            .filter(|e| *e.key == *key)
            .map(|e| e.val.clone())
    });
    if let Some(v) = hit {
        MEMO.with(|m| m.borrow_mut().buf = key);
        return v;
    }
    let val = compute(node, ctx);
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        let pos = m.sheets.iter().position(|(id, _)| *id == sheet_id);
        let i = match pos {
            Some(i) => i,
            None => {
                if m.sheets.len() >= SHEETS {
                    m.sheets.remove(0);
                }
                m.sheets.push((sheet_id, HashMap::new()));
                m.sheets.len() - 1
            }
        };
        let table = &mut m.sheets[i].1;
        if table.len() >= CAP {
            table.clear();
        }
        table.insert(hash, Entry { key: key.as_slice().into(), val: val.clone() });
        m.buf = key;
    });
    val
}
