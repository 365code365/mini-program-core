//! wx 的中立别名：nb / gb / mb 与按函数名认出 `xx.函数名`（js/api_alias.rs、prelude/api-alias.js）
use crate::js::PreludeProfile;
use crate::runtime::MiniApp;

fn app(profile: PreludeProfile) -> MiniApp {
    let mut app = MiniApp::new(375, 667).expect("MiniApp");
    app.init_profile(profile).expect("前置代码");
    app
}

#[test]
fn nb_gb_mb_与_wx_是同一个对象() {
    for p in [PreludeProfile::MiniProgram, PreludeProfile::MiniGame] {
        let a = app(p);
        let r = a.eval("[nb === wx, gb === wx, mb === wx, typeof nb.request, nb.canIUse('nb.request'), gb.canIUse('gb.showToast'), wx.canIUse('mb.request')].join(',')").unwrap();
        assert_eq!(r, "true,true,true,function,true,true,true", "{p:?}");
        // 之后往 wx 上补的 API，别名上同样看得见
        assert_eq!(a.eval("wx.__later = 1; nb.__later").unwrap(), "1");
    }
}

#[test]
fn 未声明的_xx_用了两个_api_就认成别名() {
    let a = app(PreludeProfile::MiniProgram);
    a.load_module_script("pages/a/a", "xx.setStorageSync('k', 'v'); globalThis.__got = xx.getStorageSync('k');").unwrap();
    assert_eq!(a.eval("[xx === wx, __got].join(',')").unwrap(), "true,v");
    assert_eq!(a.eval("wx.canIUse('xx.request')").unwrap(), "true");
}

#[test]
fn 只用一个_api_平台名与已有全局都不认() {
    let a = app(PreludeProfile::MiniProgram);
    a.eval("globalThis.yy = { request: function () { return 'mine'; }, showToast: function () {} };").unwrap();
    a.define_module("utils/a", "function f() { one.request(); my.request(); my.showToast(); tt.request(); tt.showToast(); yy.request(); yy.showToast(); }").unwrap();
    let r = a.eval("[typeof one, typeof my, typeof tt, yy.request()].join(',')").unwrap();
    assert_eq!(r, "undefined,undefined,undefined,mine");
}

#[test]
fn 按函数名认在模块登记时就生效() {
    // define_module 只登记工厂，require 时才执行：别名要在登记那一刻就装好
    let a = app(PreludeProfile::MiniGame);
    a.define_module("js/m", "module.exports = function () { return [zz.getSystemInfoSync().windowWidth > 0, typeof zz['request']].join(','); };").unwrap();
    assert_eq!(a.eval("typeof zz").unwrap(), "undefined", "只用到一个 API 名，不该认");
    a.define_module("js/n", "zz.request; zz.showToast;").unwrap();
    assert_eq!(a.eval("require('js/m')()").unwrap(), "true,function");
}
