//! 临时诊断：登录页按钮/输入框的最终样式
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::WxmlRenderer;
use crate::Canvas;
use serde_json::json;

#[test]
fn probe_login_button_style() {
    let wxml = r##"<view class="card">
        <input class="f-input" type="number" placeholder="请输入手机号" placeholder-class="ph"/>
        <button class="wx-btn" open-type="getPhoneNumber"><text class="wx-btn-t">微信一键登录</text></button>
      </view>"##;
    let wxss = std::fs::read_to_string("sample/tea-app/pages/login/login.wxss").unwrap();
    let app = std::fs::read_to_string("sample/tea-app/app.wxss").unwrap();
    let merged = format!("{}\n{}", app, wxss);
    let ss = WxssParser::new(&merged).parse().unwrap();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 2.0);
    let mut c = Canvas::new(750, 1334);
    let nodes = WxmlParser::new(wxml).parse().unwrap();
    let mut im = crate::ui::interaction::InteractionManager::new();
    r.render_with_interaction(&mut c, &nodes, &json!({}), &mut im);
    println!("wx-btn   => {:?}", r.node_style_by_class("wx-btn"));
    println!("wx-btn-t => {:?}", r.node_style_by_class("wx-btn-t"));
    println!("f-input  => {:?}", r.node_style_by_class("f-input"));
    println!("wx-btn 尺寸 {:?}", r.node_size_by_class("wx-btn"));
    println!("f-input 尺寸 {:?}", r.node_size_by_class("f-input"));
}
