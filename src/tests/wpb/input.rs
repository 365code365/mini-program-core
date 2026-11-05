//! B1 / B2：input / textarea 的聚焦、事件 detail、占位符样式、光标颜色、auto-height 与 linechange
use super::*;

#[test]
fn b1_focus_属性_挂载聚焦_变假失焦_change_与_detail_类型() {
    let wxml = r#"<input id="a" focus="{{f}}" value="hello" cursor="2" bindfocus="onF" bindinput="onI" bindchange="onC" bindblur="onB" style="height:40px"/>"#;
    let p_js = format!(
        "{LOG_FN} Page({{ data: {{ f: true }}, \
           onF(e){{ L('focus ' + typeof e.detail.height + ' ' + e.detail.height + ' ' + e.detail.value); }}, \
           onI(e){{ L('input ' + e.detail.value + ' ' + e.detail.cursor + ' ' + typeof e.detail.keyCode + ' ' + e.detail.keyCode); }}, \
           onC(e){{ L('change ' + e.detail.value); }}, onB(e){{ L('blur ' + e.detail.value + ' ' + e.detail.cursor); }} }});"
    );
    let root = fixture("wpb-focus-attr", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    assert!(pump_until(&mut e, 800, |e| log(e).iter().any(|s| s.starts_with("focus"))), "挂载时 focus 为真就聚焦：{:?}", log(&e));
    assert_eq!(log(&e)[0], "focus number 0 hello", "detail.height 是 number，宿主没报键盘高度前为 0");
    let evs = host_events(&mut e);
    let f = evs.iter().find(|v| v["type"] == "inputFocus").expect("要宿主拉起键盘（inputFocus）");
    assert_eq!(f["cursor"], 2, "按属性聚焦时光标在 cursor=2：{f}");
    // 光标在 2：打一个字落在中间，bindinput 的 cursor / keyCode 是真值（number）
    clear_log(&e);
    e.text_input("X");
    settle(&mut e);
    assert_eq!(log(&e), vec!["input heXllo 3 number 88"]);
    // 页面把 focus 设回 false：失焦，内容变了先 change 后 blur（input 的 blur 不带 cursor）
    clear_log(&e);
    js(&e, "getCurrentPages()[0].setData({ f: false })");
    assert!(pump_until(&mut e, 800, |e| log(e).iter().any(|s| s.starts_with("blur"))), "{:?}", log(&e));
    assert_eq!(log(&e), vec!["change heXllo", "blur heXllo undefined"]);
    assert!(host_events(&mut e).iter().any(|v| v["type"] == "inputBlur"), "失焦要收键盘");
    // 再设回 true：重新聚焦
    clear_log(&e);
    js(&e, "getCurrentPages()[0].setData({ f: true })");
    assert!(pump_until(&mut e, 800, |e| log(e).iter().any(|s| s.starts_with("focus"))), "{:?}", log(&e));
}

#[test]
fn b1_hold_keyboard_点页面不失焦_完成键发_confirm_再_blur_没改不发_change() {
    let wxml = r#"<input id="a" hold-keyboard bindfocus="onF" bindconfirm="onOk" bindchange="onC" bindblur="onB" style="height:40px"/>
                  <view bindtap="onTap" style="height:200px">空白</view>"#;
    let p_js = format!(
        "{LOG_FN} Page({{ onF(){{ L('focus'); }}, onOk(e){{ L('confirm ' + e.detail.value); }}, onC(){{ L('change'); }}, \
           onB(e){{ L('blur ' + e.detail.value); }}, onTap(){{ L('tap'); }} }});"
    );
    let root = fixture("wpb-hold-keyboard", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    tap(&mut e, 100.0, 20.0, 2000);
    settle(&mut e);
    tap(&mut e, 100.0, 120.0, 3000);
    settle(&mut e);
    assert_eq!(log(&e), vec!["focus", "tap"], "hold-keyboard：点页面照常派 tap，不失焦");
    clear_log(&e);
    e.key("enter");
    settle(&mut e);
    assert_eq!(log(&e), vec!["confirm ", "blur "], "按完成：confirm 之后失焦，内容没变不发 change");
}

#[test]
fn b1_textarea_返回值不替换_换输入框先_blur_旧的_focus_height_是键盘高() {
    // 两个框各自写全 focus / blur：派发找不到绑定时会退到「全页唯一一条同类绑定」（tap.rs），别让它串台
    let wxml = r#"<textarea id="t" bindinput="onI" bindfocus="onTF" bindblur="onB" style="height:80px"/>
                  <input id="b" bindfocus="onF" bindblur="onBB" style="height:40px"/>"#;
    let p_js = format!(
        "{LOG_FN} Page({{ onI(e){{ return 'ZZZ'; }}, onB(e){{ L('blur ' + e.detail.value + ' ' + typeof e.detail.cursor + ' ' + e.detail.cursor); }}, \
           onF(e){{ L('focus ' + e.detail.height); }}, onTF(){{ L('tfocus'); }}, onBB(e){{ L('bblur ' + e.detail.cursor); }} }});"
    );
    let root = fixture("wpb-textarea-input", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    tap(&mut e, 100.0, 40.0, 2000);
    settle(&mut e);
    e.text_input("ab");
    settle(&mut e);
    assert!(!host_events(&mut e).iter().any(|v| v["type"] == "inputValue"), "textarea 的 bindinput 返回值不替换内容");
    // 宿主报了键盘高度，再点另一个输入框：focus 的 height 是这个高度
    e.dispatch_event("keyboardHeightChange", r#"{"height":300,"duration":250}"#);
    tap(&mut e, 100.0, 100.0, 3000);
    settle(&mut e);
    js(&e, "wx.hideKeyboard()");
    settle(&mut e);
    assert_eq!(
        log(&e),
        vec!["tfocus", "blur ab number 2", "focus 300", "bblur undefined"],
        "先 textarea 失焦（带 cursor），再 input 聚焦（height = 键盘高）；input 的 blur 不带 cursor"
    );
}

#[test]
fn b1_textarea_失焦_detail_cursor_是_number() {
    let wxml = r#"<textarea id="t" bindblur="onB" style="height:80px"/>"#;
    let p_js = format!("{LOG_FN} Page({{ onB(e){{ L('blur ' + e.detail.value + ' ' + typeof e.detail.cursor + ' ' + e.detail.cursor); }} }});");
    let root = fixture("wpb-textarea-blur", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    tap(&mut e, 100.0, 40.0, 2000);
    settle(&mut e);
    e.text_input("abc");
    e.key("left");
    settle(&mut e);
    js(&e, "wx.hideKeyboard()");
    settle(&mut e);
    assert_eq!(log(&e), vec!["blur abc number 2"]);
}

#[test]
fn b1_selection_start_end_按属性聚焦时选中那一段() {
    let wxml = r#"<input id="a" focus value="abcdef" selection-start="1" selection-end="4" bindinput="onI" style="height:40px"/>"#;
    let p_js = format!("{LOG_FN} Page({{ onI(e){{ L(e.detail.value + ' ' + e.detail.cursor); }} }});");
    let root = fixture("wpb-selection", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    // 选中 bcd：打一个字替换掉选区
    e.text_input("Z");
    settle(&mut e);
    assert_eq!(log(&e), vec!["aZef 2"]);
}

#[test]
fn b1_占位符字号_光标颜色_textarea_默认占位符类() {
    let wxml = r##"<input placeholder="WWWW" placeholder-style="color:#ff0000;font-size:30px" style="height:50px"/>
                  <input placeholder="WWWW" placeholder-style="color:#ff0000;font-size:10px" style="height:50px"/>
                  <textarea placeholder="WWWW" style="height:60px"/>
                  <input id="c" focus cursor-color="#0000ff" style="height:40px"/>"##;
    let wxss = ".textarea-placeholder{color:#00c000}";
    let root = fixture("wpb-placeholder", wxml, "Page({});", &[("pages/p/p.wxss", wxss)]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    e.pump(5000);
    let big = count_color(&mut e, (0, 0, 375, 50), [255, 0, 0]);
    let small = count_color(&mut e, (0, 50, 375, 100), [255, 0, 0]);
    assert!(big > small * 4 && small > 0, "placeholder-style 的 font-size 生效：30px {big} vs 10px {small}");
    assert!(count_color(&mut e, (0, 100, 375, 160), [0, 192, 0]) > 20, "textarea 默认 placeholder-class 是 textarea-placeholder");
    let blue = count_color(&mut e, (0, 160, 375, 200), [0, 0, 255]);
    assert!(blue > 10, "cursor-color 画出蓝色光标：{blue}");
}

#[test]
fn b2_auto_height_随行数长高_linechange_detail_与_max_height() {
    let wxml = r#"<textarea id="t" auto-height bindlinechange="onL" style="width:120px;height:300px;font-size:16px;line-height:20px;max-height:100px"/>
                  <view id="below" style="height:20px">下面</view>"#;
    let p_js = format!(
        "{LOG_FN} Page({{ onL(e){{ var d = e.detail; L([typeof d.height, d.height, typeof d.heightRpx, d.lineCount].join(' ')); }} }});"
    );
    let root = fixture("wpb-auto-height", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    let h1 = rect_of(&mut e, "t")[3];
    // 默认 textarea 内边距上下各 8：一行 20 + 16
    assert!((h1 - 36.0).abs() < 1.0, "auto-height 时 style.height 不生效，一行高：{h1}");
    tap(&mut e, 60.0, 15.0, 2000);
    settle(&mut e);
    e.text_input("\n");
    e.text_input("\n");
    settle(&mut e);
    pump_until(&mut e, 300, |_| false);
    let h3 = rect_of(&mut e, "t")[3];
    assert!((h3 - 76.0).abs() < 1.0, "三行：{h3}");
    assert!(log(&e).iter().any(|s| s == "number 60 number 3"), "linechange {{height, heightRpx, lineCount}}：{:?}", log(&e));
    let below = rect_of(&mut e, "below")[1];
    assert!((below - h3).abs() < 1.0, "下面的元素跟着下移：{below}");
    for _ in 0..6 {
        e.text_input("\n");
    }
    settle(&mut e);
    pump_until(&mut e, 300, |_| false);
    let hmax = rect_of(&mut e, "t")[3];
    assert!((hmax - 100.0).abs() < 1.0, "max-height 夹住：{hmax}");
}

#[test]
fn b2_disable_default_padding_去掉默认内边距() {
    let wxml = r#"<textarea placeholder="WWWW" placeholder-style="color:#ff0000" style="height:60px"/>
                  <textarea placeholder="WWWW" placeholder-style="color:#ff0000" disable-default-padding style="height:60px"/>"#;
    let root = fixture("wpb-no-padding", wxml, "Page({});", &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    let left = |e: &mut MiniEngine, y0: u32| (0..375u32).find(|&x| (y0..y0 + 60).any(|y| rgb_at(e, x, y)[0] > 200 && rgb_at(e, x, y)[1] < 80));
    let a = left(&mut e, 0).expect("占位符");
    let b = left(&mut e, 60).expect("占位符");
    assert!(a >= 11 && b <= 2, "默认左内边距 12 / 去掉后 0：{a} / {b}");
}
