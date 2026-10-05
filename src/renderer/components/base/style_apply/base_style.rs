//! 由 CSS 声明构建基础样式（盒模型、定位、层叠）
//!
//! `style_apply_new` 的一片。**纯搬迁**：从 792 行按职责切开，一行逻辑没改。
use super::*;

/// 构建基础 Taffy 样式
pub fn build_base_style(
    node: &WxmlNode,
    ctx: &mut ComponentContext,
) -> (Style, NodeStyle) {
    let classes = get_classes(node);
    let id = node.get_attr("id");
    // 构建「祖先链 + 当前元素」，支持 #id、[attr]、*、后代/子选择器
    let mut chain = ctx.ancestors.clone();
    // 属性表只有 `[attr]` 选择器用得到；没有这类规则就别带 —— 它会随祖先链克隆被深拷很多遍
    static EMPTY_ATTRS: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();
    let desc_attrs = if ctx.stylesheet.has_attr_selectors() {
        &node.attributes
    } else {
        EMPTY_ATTRS.get_or_init(HashMap::new)
    };
    chain.push(ElementDesc::new(&node.tag_name, id, &classes, desc_attrs)
        .with_position(ctx.sibling_index, ctx.sibling_count));
    let css = ctx.stylesheet.get_styles_chain(&chain);
    
    // 先用继承的文本样式做默认值，再用 CSS/内联覆盖（CSS 继承语义）
    let mut ns = NodeStyle {
        font_size: ctx.inherited.font_size,
        text_color: ctx.inherited.color,
        font_weight: ctx.inherited.weight,
        text_align: ctx.inherited.align,
        line_height: ctx.inherited.line_height,
        letter_spacing: ctx.inherited.letter_spacing,
        font_family: ctx.inherited.font_family.clone(),
        opacity: 1.0,
        ..Default::default()
    };
    
    // 默认样式：flex 布局，列方向
    let mut ts = Style { 
        display: Display::Flex, 
        flex_direction: FlexDirection::Column,
        ..Default::default() 
    };

    // 应用类样式（顺序见 `declaration_order`：简写必须先落地）
    for (name, value) in sorted_declarations(&css) {
        apply_style_property(name, value, &mut ts, &mut ns, ctx);
    }

    // 应用内联样式
    if let Some(style_str) = node.get_attr("style") {
        for part in style_str.split(';') {
            let part = part.trim();
            if part.is_empty() { continue; }
            
            if let Some(colon_pos) = part.find(':') {
                let name = part[..colon_pos].trim();
                let value_str = part[colon_pos + 1..].trim();
                let value = parse_inline_value(value_str);
                apply_style_property(name, &value, &mut ts, &mut ns, ctx);
            }
        }
    }
    
    // 无单位 line-height 在所有声明落地后统一按最终字号换算
    if let Some(scale) = ns.line_height_scale {
        ns.line_height = Some(ns.font_size * scale);
    }

    // ── 绝对定位的包含块修正（CSS 语义）──
    //
    // `position:absolute` 的包含块是「最近的定位祖先」，一个都没有时是**初始包含块**
    // （视口）。布局引擎只会按父节点解析百分比，于是
    // `.background{position:absolute;left:0;top:0;width:100%;height:100%}` 挂在
    // 一个 auto 高度的父 view 下时，高度会塌成 0 —— 整屏铺底的背景图就此消失。
    //
    // 没有定位祖先时，这里把百分比尺寸按视口折算成确定像素，等价于把包含块换成视口。
    // 只修**高度**：宽度按父节点解析本来就是对的（父节点通常就是整宽），
    // 而且元素的 left/top 偏移仍然是相对父节点的 —— 把宽度也换成视口宽会让
    // 「窄父节点里的绝对定位元素」既变宽又不移位，反而画错（实测 canvas/组件页
    // 与 H5 的差异从 5.4%/3.8% 恶化到 12.5%/10.2%）。
    // 高度不一样：父节点高度是 auto 时百分比没有参照物，会直接塌成 0。
    if ts.position == Position::Absolute && !ctx.has_positioned_ancestor {
        if let Some(p) = dim_percent(ts.size.height) {
            ts.size.height = Dimension::length(ctx.screen_height * ctx.scale_factor * p);
        }
    }

    // ── 按压态样式 ──
    // 同一条祖先链，把目标元素标记为 pressed 并补上 `hover-class` 的类名，
    // 再取一次 CSS 声明 —— `:active` 与小程序的 `hover-class` 因此走同一条路径。
    // 只覆盖绘制类属性（taffy 布局结果丢弃）：按压不触发重新布局，
    // 这是刻意的取舍，按一下就重排整页在纯软件光栅上代价太高。
    // `<button>` 没写 hover-class 时默认就是 `button-hover`（微信文档）；
    // `hover-class="none"` 显式关掉点击态。普通 view 默认没有点击态。
    let hover_class = match node.get_attr("hover-class").map(str::trim) {
        Some("none") => None,
        Some(s) if !s.is_empty() => Some(s.to_string()),
        _ if node.tag_name == "button" => Some("button-hover".to_string()),
        _ => None,
    };
    if ctx.stylesheet.has_active_rules() || hover_class.is_some() {
        let mut pressed_chain = chain;
        if let Some(last) = pressed_chain.last_mut() {
            last.pressed = true;
            if let Some(hc) = &hover_class {
                for c in hc.split_whitespace() {
                    last.classes.push(c.to_string());
                }
            }
        }
        let pressed_css = ctx.stylesheet.get_styles_chain(&pressed_chain);
        let mut pressed_ns = ns.clone();
        let mut throwaway_ts = ts.clone();
        for (name, value) in sorted_declarations(&pressed_css) {
            apply_style_property(name, value, &mut throwaway_ts, &mut pressed_ns, ctx);
        }
        // 内联 style 优先级高于类样式，按压态同样要重放一遍
        if let Some(style_str) = node.get_attr("style") {
            for part in style_str.split(';') {
                let part = part.trim();
                if part.is_empty() { continue; }
                if let Some(colon_pos) = part.find(':') {
                    let name = part[..colon_pos].trim();
                    let value = parse_inline_value(part[colon_pos + 1..].trim());
                    apply_style_property(name, &value, &mut throwaway_ts, &mut pressed_ns, ctx);
                }
            }
        }
        if let Some(scale) = pressed_ns.line_height_scale {
            pressed_ns.line_height = Some(pressed_ns.font_size * scale);
        }
        pressed_ns.pressed_style = None; // 不递归
        ns.pressed_style = Some(Box::new(pressed_ns));
    }
    
    (ts, ns)
}
