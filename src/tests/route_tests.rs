//! 路由 / 页面栈 / 组件模型 / 模块系统 / 异步 测试
//! 通过真实的 MiniApp（含 QuickJS 运行时）验证逻辑层行为

use crate::runtime::MiniApp;

/// 创建并初始化一个 app
fn new_app() -> MiniApp {
    let mut app = MiniApp::new(375, 667).expect("create MiniApp");
    app.init().expect("init MiniApp");
    app
}

// ============ 页面注册与数据 ============

#[test]
fn test_page_register_and_data() {
    let app = new_app();
    app.load_script(r#"Page({ data: { title: "首页", count: 0 } });"#).unwrap();
    assert_eq!(app.eval("getCurrentPages().length").unwrap(), "1");
    assert_eq!(app.eval("__currentPage.data.title").unwrap(), "首页");
    assert_eq!(app.eval("__currentPage.data.count").unwrap(), "0");
}

#[test]
fn test_page_method_setdata() {
    let app = new_app();
    app.load_script(r#"
        Page({
            data: { count: 0 },
            inc: function() { this.setData({ count: this.data.count + 1 }); }
        });
    "#).unwrap();
    app.eval("__currentPage.inc(); __currentPage.inc(); __currentPage.inc();").unwrap();
    assert_eq!(app.eval("__currentPage.data.count").unwrap(), "3");
    // __getPageData 返回 JSON
    let data = app.eval("__getPageData()").unwrap();
    assert!(data.contains("\"count\""));
    assert!(data.contains("3"));
}

// ============ 页面栈 / 路由切换 ============

#[test]
fn test_page_stack_navigate_and_back() {
    let app = new_app();
    // 页面 A 入栈
    app.load_script(r#"Page({ data: { name: "A" } });"#).unwrap();
    assert_eq!(app.eval("getCurrentPages().length").unwrap(), "1");

    // 模拟 navigateTo -> 加载页面 B（再次 Page() 入栈）
    app.load_script(r#"Page({ data: { name: "B" } });"#).unwrap();
    assert_eq!(app.eval("getCurrentPages().length").unwrap(), "2");
    assert_eq!(app.eval("__currentPage.data.name").unwrap(), "B");

    // 模拟 navigateTo -> 页面 C
    app.load_script(r#"Page({ data: { name: "C" } });"#).unwrap();
    assert_eq!(app.eval("getCurrentPages().length").unwrap(), "3");
    assert_eq!(app.eval("__currentPage.data.name").unwrap(), "C");

    // navigateBack -> 回到 B
    app.eval("wx.navigateBack()").unwrap();
    assert_eq!(app.eval("getCurrentPages().length").unwrap(), "2");
    assert_eq!(app.eval("__currentPage.data.name").unwrap(), "B");

    // navigateBack(delta=1) -> 回到 A
    app.eval("wx.navigateBack({ delta: 1 })").unwrap();
    assert_eq!(app.eval("getCurrentPages().length").unwrap(), "1");
    assert_eq!(app.eval("__currentPage.data.name").unwrap(), "A");
}

#[test]
fn test_page_instance_isolation() {
    let app = new_app();
    // A：修改自身数据
    app.load_script(r#"Page({ data: { v: "A0" } });"#).unwrap();
    app.eval(r#"__currentPage.setData({ v: "A1" });"#).unwrap();
    // 导航到 B
    app.load_script(r#"Page({ data: { v: "B0" } });"#).unwrap();
    assert_eq!(app.eval("__currentPage.data.v").unwrap(), "B0");
    // 返回 A：A 的修改应被保留（实例隔离）
    app.eval("wx.navigateBack()").unwrap();
    assert_eq!(app.eval("__currentPage.data.v").unwrap(), "A1");
}

#[test]
fn test_navigate_back_delta_multiple() {
    let app = new_app();
    for name in ["A", "B", "C", "D"] {
        app.load_script(&format!(r#"Page({{ data: {{ name: "{}" }} }});"#, name)).unwrap();
    }
    assert_eq!(app.eval("getCurrentPages().length").unwrap(), "4");
    // 一次回退 2 层 -> 回到 B
    app.eval("wx.navigateBack({ delta: 2 })").unwrap();
    assert_eq!(app.eval("getCurrentPages().length").unwrap(), "2");
    assert_eq!(app.eval("__currentPage.data.name").unwrap(), "B");
}

// ============ setData 数据路径 ============

#[test]
fn test_setdata_data_paths() {
    let app = new_app();
    app.load_script(r#"Page({ data: { obj: { a: 1 }, list: [ { x: 1 }, { x: 2 } ] } });"#).unwrap();
    app.eval(r#"__currentPage.setData({ "obj.a": 5, "obj.b": 7, "list[1].x": 99 });"#).unwrap();
    assert_eq!(app.eval("__currentPage.data.obj.a").unwrap(), "5");
    assert_eq!(app.eval("__currentPage.data.obj.b").unwrap(), "7");
    assert_eq!(app.eval("__currentPage.data.list[1].x").unwrap(), "99");
    // 未涉及的保持不变
    assert_eq!(app.eval("__currentPage.data.list[0].x").unwrap(), "1");
}

#[test]
fn test_setdata_callback_runs() {
    let app = new_app();
    app.load_script(r#"Page({ data: { x: 0 } });"#).unwrap();
    app.eval(r#"var __cb = false; __currentPage.setData({ x: 1 }, function(){ __cb = true; });"#).unwrap();
    assert_eq!(app.eval("__cb").unwrap(), "true");
    assert_eq!(app.eval("__currentPage.data.x").unwrap(), "1");
}

// ============ App 全局 ============

#[test]
fn test_app_global_data() {
    let app = new_app();
    app.load_script(r#"App({ globalData: { version: "1.2.3", user: null } });"#).unwrap();
    assert_eq!(app.eval("getApp().globalData.version").unwrap(), "1.2.3");
    app.eval(r#"getApp().globalData.user = "tom";"#).unwrap();
    assert_eq!(app.eval("getApp().globalData.user").unwrap(), "tom");
}

// ============ 自定义组件 / Behavior ============

#[test]
fn test_component_instance_and_methods() {
    let app = new_app();
    app.eval(r#"
        __setPendingComponentPath('components/counter');
        Component({
            data: { n: 5 },
            properties: { label: { type: String, value: 'default' } },
            methods: { inc: function() { this.setData({ n: this.data.n + 1 }); } }
        });
    "#).unwrap();
    // 创建实例并注入 property
    app.eval(r#"globalThis.inst = __createComponentInstance('components/counter', { label: 'Hello' });"#).unwrap();
    assert_eq!(app.eval("inst.data.label").unwrap(), "Hello");
    assert_eq!(app.eval("inst.data.n").unwrap(), "5");
    // 调用方法
    app.eval("inst.inc();").unwrap();
    assert_eq!(app.eval("inst.data.n").unwrap(), "6");
}

#[test]
fn test_component_property_defaults() {
    let app = new_app();
    app.eval(r#"
        __setPendingComponentPath('components/box');
        Component({
            properties: {
                title: { type: String, value: 'T' },
                count: { type: Number, value: 10 },
                active: { type: Boolean, value: true }
            },
            data: {}
        });
        globalThis.b = __createComponentInstance('components/box', {});
    "#).unwrap();
    assert_eq!(app.eval("b.data.title").unwrap(), "T");
    assert_eq!(app.eval("b.data.count").unwrap(), "10");
    assert_eq!(app.eval("b.data.active").unwrap(), "true");
}

#[test]
fn test_component_observer() {
    let app = new_app();
    app.eval(r#"
        __setPendingComponentPath('components/obs');
        Component({
            data: { n: 0, observed: 'no' },
            observers: { 'n': function(val) { this.data.observed = 'changed:' + val; } },
            methods: {}
        });
        globalThis.o = __createComponentInstance('components/obs', {});
        o.setData({ n: 42 });
    "#).unwrap();
    assert_eq!(app.eval("o.data.n").unwrap(), "42");
    assert_eq!(app.eval("o.data.observed").unwrap(), "changed:42");
}

#[test]
fn test_component_lifetime_created_attached() {
    let app = new_app();
    app.eval(r#"
        globalThis.__lc = [];
        __setPendingComponentPath('components/lc');
        Component({
            data: {},
            lifetimes: {
                created: function() { __lc.push('created'); },
                attached: function() { __lc.push('attached'); },
                ready: function() { __lc.push('ready'); }
            }
        });
        __createComponentInstance('components/lc', {});
    "#).unwrap();
    assert_eq!(app.eval("__lc.join(',')").unwrap(), "created,attached,ready");
}

#[test]
fn test_component_detached_and_moved() {
    let app = new_app();
    app.eval(r#"
        globalThis.__lc = [];
        __setPendingComponentPath('components/dm');
        Component({
            data: {},
            lifetimes: {
                detached: function() { __lc.push('detached'); },
                moved: function() { __lc.push('moved'); }
            }
        });
        var inst = __createComponentInstance('components/dm', {});
        __moveComponentInstance(inst.id);
        __detachComponentInstance(inst.id);
    "#).unwrap();
    assert_eq!(app.eval("__lc.join(',')").unwrap(), "moved,detached");
    // 实例已被回收
    assert_eq!(app.eval("typeof __componentInstances[inst.id]").unwrap(), "undefined");
}

#[test]
fn test_component_page_lifetimes() {
    let app = new_app();
    app.eval(r#"
        globalThis.__pl = [];
        __setPendingComponentPath('components/pl');
        Component({
            data: {},
            pageLifetimes: {
                show: function() { __pl.push('show'); },
                hide: function() { __pl.push('hide'); },
                resize: function() { __pl.push('resize'); }
            }
        });
        __createComponentInstance('components/pl', {});
        // 页面级生命周期应联动组件 pageLifetimes
        Page({ data: {}, onShow: function(){}, onHide: function(){}, onResize: function(){} });
        __dispatchPage('onShow');
        __dispatchPage('onResize');
        __dispatchPage('onHide');
    "#).unwrap();
    assert_eq!(app.eval("__pl.join(',')").unwrap(), "show,resize,hide");
}

#[test]
fn test_page_full_lifecycle_dispatch() {
    let app = new_app();
    app.eval(r#"
        globalThis.__seq = [];
        Page({
            data: {},
            onLoad: function(){ __seq.push('load'); },
            onShow: function(){ __seq.push('show'); },
            onReady: function(){ __seq.push('ready'); },
            onPullDownRefresh: function(){ __seq.push('pull'); },
            onReachBottom: function(){ __seq.push('bottom'); },
            onPageScroll: function(){ __seq.push('scroll'); },
            onShareAppMessage: function(){ __seq.push('share'); return {title:'t'}; },
            onHide: function(){ __seq.push('hide'); },
            onUnload: function(){ __seq.push('unload'); }
        });
        __dispatchPage('onLoad');
        __dispatchPage('onShow');
        __dispatchPage('onReady');
        __dispatchPage('onPullDownRefresh');
        __dispatchPage('onReachBottom');
        __dispatchPage('onPageScroll');
        __dispatchPage('onShareAppMessage');
        __dispatchPage('onHide');
        __dispatchPage('onUnload');
    "#).unwrap();
    assert_eq!(
        app.eval("__seq.join(',')").unwrap(),
        "load,show,ready,pull,bottom,scroll,share,hide,unload"
    );
}

#[test]
fn test_app_lifecycle_dispatch() {
    let app = new_app();
    app.eval(r#"
        globalThis.__ap = [];
        App({
            onLaunch: function(){ __ap.push('launch'); },
            onShow: function(){ __ap.push('show'); },
            onHide: function(){ __ap.push('hide'); },
            onError: function(e){ __ap.push('error:' + e); },
            onPageNotFound: function(){ __ap.push('404'); },
            onThemeChange: function(t){ __ap.push('theme:' + t.theme); }
        });
        __dispatchApp('onShow');
        __dispatchApp('onHide');
        __dispatchApp('onError', 'boom');
        __dispatchApp('onPageNotFound');
        __dispatchApp('onThemeChange', { theme: 'dark' });
    "#).unwrap();
    assert_eq!(
        app.eval("__ap.join(',')").unwrap(),
        "launch,show,hide,error:boom,404,theme:dark"
    );
}

#[test]
fn test_behavior_mixin() {
    let app = new_app();
    app.eval(r#"
        var shareBehavior = Behavior({
            data: { shared: 'S' },
            methods: { hello: function() { return 'hi'; } }
        });
        __setPendingComponentPath('components/withb');
        Component({
            behaviors: [shareBehavior],
            data: { own: 'O' },
            methods: {}
        });
        globalThis.wb = __createComponentInstance('components/withb', {});
    "#).unwrap();
    assert_eq!(app.eval("wb.data.shared").unwrap(), "S");
    assert_eq!(app.eval("wb.data.own").unwrap(), "O");
    assert_eq!(app.eval("typeof wb.hello").unwrap(), "function");
    assert_eq!(app.eval("wb.hello()").unwrap(), "hi");
}

#[test]
fn test_component_trigger_event_to_page() {
    let app = new_app();
    // 页面提供处理器
    app.load_script(r#"
        Page({ data: { received: 'none' }, onCustom: function(e) { this.setData({ received: e.detail.msg }); } });
    "#).unwrap();
    // 组件实例触发事件，登记映射到页面方法
    app.eval(r#"
        __setPendingComponentPath('components/emitter');
        Component({ methods: { fire: function() { this.triggerEvent('custom', { msg: 'from-child' }); } } });
        globalThis.em = __createComponentInstance('components/emitter', {});
        __bindComponentEvent(em.id, 'custom', 'onCustom');
        em.fire();
    "#).unwrap();
    assert_eq!(app.eval("__currentPage.data.received").unwrap(), "from-child");
}

// ============ CommonJS 模块系统 ============

#[test]
fn test_commonjs_require_exports() {
    let app = new_app();
    app.define_module("utils/math", r#"
        exports.add = function(a, b) { return a + b; };
        exports.mul = function(a, b) { return a * b; };
    "#).unwrap();
    app.define_module("pages/calc/calc", r#"
        var math = require('../../utils/math');
        Page({ data: { sum: math.add(2, 3), product: math.mul(4, 5) } });
    "#).unwrap();
    app.require_module("pages/calc/calc").unwrap();
    assert_eq!(app.eval("__currentPage.data.sum").unwrap(), "5");
    assert_eq!(app.eval("__currentPage.data.product").unwrap(), "20");
}

#[test]
fn test_commonjs_module_exports_object() {
    let app = new_app();
    app.define_module("lib/config", r#"
        module.exports = { appName: 'Mini', maxItems: 100 };
    "#).unwrap();
    app.define_module("app", r#"
        var cfg = require('lib/config');
        App({ globalData: { name: cfg.appName, max: cfg.maxItems } });
    "#).unwrap();
    app.require_module("app").unwrap();
    assert_eq!(app.eval("getApp().globalData.name").unwrap(), "Mini");
    assert_eq!(app.eval("getApp().globalData.max").unwrap(), "100");
}

#[test]
fn test_commonjs_module_cached() {
    let app = new_app();
    // 模块只执行一次（缓存）
    app.define_module("lib/counter", r#"
        globalThis.__modInitCount = (globalThis.__modInitCount || 0) + 1;
        module.exports = { n: globalThis.__modInitCount };
    "#).unwrap();
    app.define_module("m/a", r#"var c = require('../lib/counter'); Page({ data: { first: c.n } });"#).unwrap();
    app.require_module("m/a").unwrap();
    // 再次 require 同模块
    app.eval("var c2 = require('lib/counter'); globalThis.__second = c2.n;").unwrap();
    assert_eq!(app.eval("__modInitCount").unwrap(), "1", "模块应只初始化一次");
    assert_eq!(app.eval("__second").unwrap(), "1");
}

// ============ 异步 / Promise / 微任务 ============

#[test]
fn test_promise_microtask_pump() {
    let app = new_app();
    // eval 内部会 pump_jobs，Promise.then 回调应已执行
    app.eval(r#"var __pr = 'pending'; Promise.resolve(42).then(function(v){ __pr = 'got' + v; });"#).unwrap();
    assert_eq!(app.eval("__pr").unwrap(), "got42");
}

#[test]
fn test_async_await() {
    let app = new_app();
    app.eval(r#"
        var __ar = 'none';
        async function run() { var v = await Promise.resolve(7); __ar = 'v=' + v; }
        run();
    "#).unwrap();
    // 再触发一次 pump
    app.eval("1;").unwrap();
    assert_eq!(app.eval("__ar").unwrap(), "v=7");
}

#[test]
fn test_promise_chain() {
    let app = new_app();
    app.eval(r#"
        var __chain = 0;
        Promise.resolve(1)
            .then(function(v){ return v + 1; })
            .then(function(v){ return v * 10; })
            .then(function(v){ __chain = v; });
    "#).unwrap();
    app.eval("1;").unwrap();
    assert_eq!(app.eval("__chain").unwrap(), "20");
}

// ============ 表达式引擎在页面数据中的行为（集成） ============

#[test]
fn test_json_roundtrip_object() {
    let app = new_app();
    app.load_script(r#"Page({ data: { user: { name: "Tom", tags: ["a", "b"] } } });"#).unwrap();
    let json = app.eval("JSON.stringify(__currentPage.data.user)").unwrap();
    assert!(json.contains("Tom"));
    assert!(json.contains("\"tags\""));
}
