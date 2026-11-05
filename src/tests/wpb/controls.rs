//! B5 / B6 / B7：radio / checkbox / label / button / icon / progress
use super::*;

/// 点 `#id` 的中心（boundingClientRect 是视口坐标，与 pointer 同一坐标系）
fn tap_id(e: &mut MiniEngine, id: &str, t: u64) {
    let [l, top, w, h] = rect_of(e, id);
    assert!(w > 0.0 && h > 0.0, "#{id} 没有盒子：{l} {top} {w} {h}");
    tap(e, (l + w / 2.0) as f32, (top + h / 2.0) as f32, t);
    settle(e);
}

#[test]
fn b5_radio_互斥只在同组_点已选中的不发_change() {
    let wxml = r#"<form bindsubmit="onSubmit">
      <radio-group name="r1" bindchange="g1"><radio id="a" value="a" checked/><radio id="b" value="b"/></radio-group>
      <radio-group name="r2" bindchange="g2"><radio id="x" value="x" checked/><radio id="y" value="y"/></radio-group>
      <button id="go" form-type="submit" style="width:100px;height:40px;margin:0">交</button>
    </form>"#;
    let p_js = format!(
        "{LOG_FN} Page({{ g1(e){{ L('g1 ' + JSON.stringify(e.detail.value)); }}, g2(e){{ L('g2 ' + JSON.stringify(e.detail.value)); }}, \
           onSubmit(e){{ L('submit ' + JSON.stringify(e.detail.value)); }} }});"
    );
    let root = fixture("wpb-radio-groups", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    tap_id(&mut e, "b", 2000);
    tap_id(&mut e, "b", 3000);
    tap_id(&mut e, "go", 4000);
    assert_eq!(
        log(&e),
        vec![r#"g1 "b""#, r#"submit {"r1":"b","r2":"x"}"#],
        "选 b 只取消同组的 a、另一组的 x 不动；再点已选中的 b 不发 change"
    );
}

#[test]
fn b5_长列表_组的值与_label_for_不看视口() {
    let items: String = (0..30).map(|i| format!(r#"<checkbox id="c{i}" value="c{i}" {} style="display:block;height:80px"/>"#, if i == 0 { "checked" } else { "" })).collect();
    let wxml = format!(
        r#"<form bindsubmit="onSubmit">
          <label for="far" style="display:block;height:40px">远处的勾选</label>
          <view style="display:flex;height:60px"><label style="display:flex;height:60px;width:300px"><text style="width:200px">按钮的说明文字</text><button form-type="submit" style="width:80px;height:40px;margin:0">交</button></label></view>
          <checkbox-group name="list" bindchange="onList">{items}</checkbox-group>
          <view style="height:3000px"></view>
          <checkbox-group name="far"><checkbox id="far" value="v"/></checkbox-group>
        </form>"#
    );
    let p_js = format!(
        "{LOG_FN} Page({{ onList(e){{ L('list ' + JSON.stringify(e.detail.value)); }}, onSubmit(e){{ L('submit ' + JSON.stringify(e.detail.value)); }} }});"
    );
    let root = fixture("wpb-long-group", &wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    // 点 label：for 指向 3000px 以外的勾选框（本帧没画、不在交互元素表里），照样勾上
    tap(&mut e, 100.0, 20.0, 2000);
    settle(&mut e);
    // 点 label 里的说明文字：label 包着的 submit 按钮提交表单
    tap(&mut e, 50.0, 70.0, 3000);
    settle(&mut e);
    // 滚到列表底部点最后一项：第一项（已滚出视口）仍在 detail.value 里
    js(&e, "wx.pageScrollTo({ scrollTop: 2000, duration: 0 })");
    settle(&mut e);
    tap_id(&mut e, "c29", 5000);
    let l = log(&e);
    assert_eq!(l.first().map(String::as_str), Some(r#"submit {"far":["v"],"list":["c0"]}"#), "{l:?}");
    assert_eq!(l.last().map(String::as_str), Some(r#"list ["c0","c29"]"#), "{l:?}");
}

#[test]
fn b6_disabled_按钮不派_tap_loading_只重画按钮那一块() {
    let wxml = r#"<view id="row" bindtap="onRow" style="height:60px"><button id="dis" disabled bindtap="onBtn" style="width:200px;height:44px;margin:0">禁用</button></view>
                  <button id="ld" loading style="width:200px;height:44px;margin:0">加载中</button>"#;
    let p_js = format!("{LOG_FN} Page({{ onRow(e){{ L('row ' + e.target.id + ' ' + e.currentTarget.id); }}, onBtn(){{ L('btn'); }} }});");
    let root = fixture("wpb-button", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    tap(&mut e, 100.0, 22.0, 2000);
    settle(&mut e);
    assert_eq!(log(&e), vec!["row row row"], "禁用按钮不接 tap，点击落到外层（target 是外层自己）");
    // loading 转圈：一秒 12 格，每格只重画按钮自己（局部），不整帧
    let mut kinds = Vec::new();
    for i in 0..12u64 {
        std::thread::sleep(std::time::Duration::from_millis(90));
        if e.pump(10_000 + i * 90) {
            kinds.push(e.last_frame_kind());
        }
    }
    assert!(kinds.len() >= 4, "转圈要续帧：{kinds:?}");
    assert!(kinds.iter().all(|k| *k == crate::host::engine_damage::FrameKind::Damage), "只重画按钮那一块：{kinds:?}");
}

#[test]
fn b7_progress_默认直角_rpx_color_active_与_activeend_icon_size_rpx() {
    let wxml = r##"<progress percent="50" stroke-width="40rpx" color="#ff0000" style="width:200px"/>
                  <view style="height:20px"></view>
                  <progress id="pa" percent="30" active duration="5" bindactiveend="onEnd" style="width:200px"/>
                  <view style="height:20px"></view>
                  <icon type="success" size="80rpx" color="#0000ff"/>"##;
    let p_js = format!("{LOG_FN} Page({{ onEnd(e){{ L('end ' + typeof e.detail.curPercent + ' ' + e.detail.curPercent); }} }});");
    let root = fixture("wpb-progress", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    // 40rpx = 20px 高（375 宽），color（activeColor 的旧名）红，默认直角：左上角那个像素就是红的
    assert_eq!(rgb_at(&mut e, 0, 0), [255, 0, 0], "直角：左上角像素是进度色");
    assert_eq!(rgb_at(&mut e, 99, 19), [255, 0, 0]);
    assert_ne!(rgb_at(&mut e, 99, 21), [255, 0, 0], "条高 20px");
    assert_ne!(rgb_at(&mut e, 101, 10), [255, 0, 0], "50% 到 100px");
    // active：30% × 5ms = 150ms 走完，发 activeend（detail.curPercent 是 number）
    assert!(pump_until(&mut e, 1500, |e| log(e).iter().any(|s| s.starts_with("end"))), "{:?}", log(&e));
    assert_eq!(log(&e), vec!["end number 30"]);
    // icon size="80rpx" = 40px：蓝色像素的横向跨度 ≤ 40 且 > 30（从前只认纯数字，落回 23）
    let (w, _) = e.pixel_size();
    let px = e.rgba();
    let xs: Vec<u32> = (0..w).filter(|&x| (40..100u32).any(|y| {
        let i = ((y * w + x) * 4) as usize;
        px[i] < 60 && px[i + 1] < 60 && px[i + 2] > 200
    })).collect();
    let span = xs.last().unwrap_or(&0) - xs.first().unwrap_or(&0) + 1;
    assert!(span > 30 && span <= 40, "icon 40px：{span}");
}
