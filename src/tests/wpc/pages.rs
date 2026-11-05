//! C7 / C8 / C9：page-container（侧滑先关、一页一个、下滑跟手）、开放能力组件、snapshot / open-container / share-element
use super::*;

const LOG_FN: &str = "function L(s){ getApp().log.push(s); }";

const PC_PAGE: &str = r#"<page-container show="{{a}}" close-on-slide-down bind:afterenter="ea" bind:afterleave="la"><view style="height:300px;background-color:#ff0000"></view></page-container>
<page-container show="{{b}}" bind:afterenter="eb"><view style="height:300px;background-color:#0000ff"></view></page-container>"#;

#[test]
fn page_container_侧滑先关容器_一页只有一个() {
    let q_js = format!("{LOG_FN} Page({{ data: {{ a: false, b: false }}, ea(){{ L('enter a'); }}, eb(){{ L('enter b'); }}, la(){{ L('leave a'); this.setData({{ a: false }}); }} }});");
    let root = fixture("pc-edge", "<view>p</view>", "Page({});", &[("pages/q/q.wxml", PC_PAGE), ("pages/q/q.js", &q_js)]);
    let mut e = engine(&root, EngineOptions::default());
    js(&e, "wx.navigateTo({ url: '/pages/q/q' })");
    assert!(pump_until(&mut e, 2000, |e| e.page_depth() == 2));
    js(&e, "getCurrentPages()[1].setData({ a: true })");
    assert!(pump_until(&mut e, 2000, |e| log(e).contains(&"enter a".to_string())), "{:?}", log(&e));
    // 第二个容器：已有一个弹着，出不来（官方 tip：当前页面最多只有 1 个容器）
    js(&e, "getCurrentPages()[1].setData({ b: true })");
    pump_until(&mut e, 600, |_| false);
    assert!(!log(&e).contains(&"enter b".to_string()), "{:?}", log(&e));
    // 左边缘往右划：关容器，不离开页面
    drag(&mut e, (4.0, 300.0), (200.0, 300.0), 8);
    assert!(pump_until(&mut e, 2000, |e| log(e).contains(&"leave a".to_string())), "{:?}", log(&e));
    pump_until(&mut e, 300, |_| false);
    assert_eq!(e.page_depth(), 2, "侧滑只关了容器");
}

#[test]
fn page_container_下滑跟手_不够就弹回() {
    let q_js = format!("{LOG_FN} Page({{ data: {{ a: false, b: false }}, ea(){{ L('enter a'); }}, eb(){{}}, la(){{ L('leave a'); }} }});");
    let root = fixture("pc-slide", "<view>p</view>", "Page({});", &[("pages/q/q.wxml", PC_PAGE), ("pages/q/q.js", &q_js)]);
    let mut e = engine(&root, EngineOptions::default());
    js(&e, "wx.navigateTo({ url: '/pages/q/q' })");
    assert!(pump_until(&mut e, 2000, |e| e.page_depth() == 2));
    js(&e, "getCurrentPages()[1].setData({ a: true })");
    assert!(pump_until(&mut e, 2000, |e| log(e).contains(&"enter a".to_string())));
    pump_until(&mut e, 200, |_| false);
    // 按住往下拖 60px（不到 1/5 屏高 = 133）：面板跟着往下走
    let (_, h) = e.pixel_size();
    let probe = h - 290; // 面板顶边往下 10px 处
    e.pointer_down(100.0, 500.0);
    e.pump(5000);
    for (i, y) in [520.0, 540.0, 560.0].iter().enumerate() {
        e.pointer_move(100.0, *y);
        e.pump(5016 + i as u64 * 16);
    }
    pump_until(&mut e, 100, |_| false);
    assert_ne!(rgb_at(&mut e, 100, probe), [255, 0, 0], "跟手：面板顶边下移了 60px");
    e.pointer_up(100.0, 560.0);
    pump_until(&mut e, 600, |_| false);
    assert_eq!(rgb_at(&mut e, 100, probe), [255, 0, 0], "不够距离：弹回原位");
    assert!(!log(&e).contains(&"leave a".to_string()));
}

#[test]
fn 开放能力组件按官方失败路径走_open_data_显示微信用户() {
    let root = fixture(
        "open-caps",
        r#"<ad unit-id="x" binderror="adErr"></ad>
           <official-account binderror="oaErr"></official-account>
           <open-data type="userNickName"></open-data>
           <open-data type="userCity" default-text="未知" binderror="odErr"></open-data>
           <open-data type="userAvatarUrl" style="width:40px;height:40px;display:block"></open-data>
           <view id="end" style="height:10px"></view>"#,
        &format!("{LOG_FN} Page({{ adErr(e){{ L('ad ' + e.detail.errCode); }}, oaErr(e){{ L('oa ' + e.detail.status); }}, odErr(){{ L('od'); }}, \
                   q(){{ this.createSelectorQuery().select('#end').boundingClientRect(r => L('end ' + Math.round(r.top))).exec(); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    assert!(pump_until(&mut e, 1500, |e| log(e).len() >= 3), "{:?}", log(&e));
    let l = log(&e);
    assert!(l.contains(&"ad 1004".to_string()) && l.contains(&"oa 5".to_string()) && l.contains(&"od".to_string()), "{l:?}");
    assert_eq!(js(&e, "String(wx.canIUse('ad') && wx.canIUse('open-data') && wx.canIUse('snapshot'))"), "true");
    // ad / official-account 不占高度；两行 open-data 文本 + 40px 头像在上面
    clear_log(&e);
    js(&e, "getCurrentPages()[0].q()");
    assert!(pump_until(&mut e, 1000, |e| !log(e).is_empty()));
    let top: f32 = log(&e)[0].trim_start_matches("end ").parse().unwrap();
    assert!(top > 40.0 && top < 120.0, "广告与公众号组件不占高度：{top}");
    // 灰头像（#e6e6e6）
    let y = (top - 20.0) as u32;
    assert_eq!(rgb_at(&mut e, 20, y), [0xe6, 0xe6, 0xe6]);
}

#[test]
fn snapshot_截图_file_png_与_arraybuffer_rgba() {
    let root = fixture(
        "snapshot",
        r##"<snapshot id="s" style="display:block;width:60px;height:30px"><view style="width:60px;height:30px;background-color:#00ff00"></view></snapshot>
           <open-container closed-color="#0000ff" closed-border-radius="8" style="width:100px;height:40px"><view style="height:40px"></view></open-container>"##,
        &format!("{LOG_FN} Page({{ go(type, format){{ this.createSelectorQuery().select('#s').node().exec(r => r[0].node.takeSnapshot({{ type: type, format: format, \
                   success: res => L(res.errMsg + ' ' + res.width + 'x' + res.height + ' ' + (res.tempFilePath || '') + ' ' + \
                     (res.data ? res.data.byteLength + ':' + new Uint8Array(res.data).slice(0, 4).join(',') : '')), fail: res => L(res.errMsg) }})); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 300, |_| false);
    js(&e, "getCurrentPages()[0].go('file', 'png')");
    assert!(pump_until(&mut e, 2000, |e| !log(e).is_empty()), "{:?}", log(&e));
    let l = log(&e);
    let parts: Vec<&str> = l[0].split(' ').collect();
    assert_eq!(parts[..2], ["takeSnapshot:ok", "60x30"], "{l:?}");
    let path = crate::wxfile::to_path(parts[2]).expect("wxfile 路径");
    let img = image::open(&path).expect("是 PNG").to_rgba8();
    assert_eq!((img.width(), img.height()), (60, 30));
    assert_eq!(img.get_pixel(30, 15).0, [0, 255, 0, 255], "画的是子树");
    clear_log(&e);
    js(&e, "getCurrentPages()[0].go('arraybuffer', 'rgba')");
    assert!(pump_until(&mut e, 2000, |e| !log(e).is_empty()), "{:?}", log(&e));
    assert!(log(&e)[0].ends_with(" 7200:0,255,0,255"), "60×30×4 字节的 RGBA：{:?}", log(&e));
    // open-container 的 closed 外观：背景色
    assert_eq!(rgb_at(&mut e, 50, 50), [0, 0, 255]);
}

#[test]
fn share_element_容器弹出时从页面上的同_key_元素飞过去() {
    let wxml = r#"<share-element key="k" style="display:block;width:50px;height:50px"><view style="width:50px;height:50px;background-color:#ff0000"></view></share-element>
      <page-container show="{{show}}" position="center" bind:afterenter="ea" bind:afterleave="la">
        <share-element id="t" key="k" transform style="display:block;width:100px;height:100px"><view style="width:100px;height:100px;background-color:#ff0000"></view></share-element>
      </page-container>"#;
    let root = fixture(
        "share-el",
        wxml,
        &format!("{LOG_FN} Page({{ data: {{ show: false }}, ea(){{ L('enter'); }}, la(){{ L('leave'); this.setData({{ show: false }}); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 300, |_| false);
    js(&e, "getCurrentPages()[0].setData({ show: true })");
    assert!(pump_until(&mut e, 2000, |e| log(e).contains(&"enter".to_string())), "{:?}", log(&e));
    let anim = js(&e, "JSON.stringify(getCurrentPages()[0].selectComponent('#t').data.anim)");
    assert!(anim.contains("translate") && anim.contains("scale"), "目标从源的位置大小飞过来：{anim}");
    let first = js(&e, "var r = getCurrentPages()[0].__shareEls.k; r[0].data.hidden + ',' + (r[0] !== getCurrentPages()[0].selectComponent('#t'))");
    assert_eq!(first, "true,true", "飞的时候页面上那个藏起来");
    // 关容器：飞回去，结束后页面上那个重新显示
    assert!(e.back());
    assert!(pump_until(&mut e, 2000, |e| log(e).contains(&"leave".to_string())));
    pump_until(&mut e, 500, |_| false);
    assert_eq!(js(&e, "String(getCurrentPages()[0].__shareEls.k[0].data.hidden)"), "false");
}
