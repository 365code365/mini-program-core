//! 自定义组件实例的逻辑层用例：对账（建 / 改属性 / detached）、属性类型转换、
//! observer 与 observers、triggerEvent、按实例派发事件、selectComponent、pageLifetimes。
//!
//! 模板层（实例键、slot 分配）的用例在 `component_instance_tests.rs`；用临时小程序跑
//! `MiniEngine` 的端到端点击在集成测试 `tests/component_e2e.rs` —— 小程序根目录是进程级的，
//! 放在这里会和用 sample-app 的引擎用例并行时互相改掉根目录（实测全量跑必挂）。

use crate::runtime::MiniApp;

/// 一个页面 + 一个登记在 `c/x` 的组件（标签 `x-c`）
fn app_with(component_js: &str) -> MiniApp {
    let mut app = MiniApp::new(375, 667).expect("create MiniApp");
    app.init().expect("init");
    app.load_script(
        r#"var __log = [];
        Page({ data: { v: 1 }, onPick: function (e) { __log.push('page:' + e.type + ':' + JSON.stringify(e.detail) + ':' + typeof e.detail.n + ':' + e.target.id + ':' + e.target.dataset.i); } });"#,
    )
    .unwrap();
    app.eval("__setPendingComponentPath('c/x')").unwrap();
    app.load_script(component_js).unwrap();
    app.eval("__setPendingComponentPath('')").unwrap();
    app.eval("__registerPageComponent('x-c', 'c/x')").unwrap();
    app
}

fn reconcile(app: &MiniApp, list: &str) -> String {
    app.eval(&format!("__reconcileComponents({{\"list\":{list}}})")).unwrap()
}

fn js(app: &MiniApp, code: &str) -> String {
    app.eval(code).unwrap().trim_matches('"').to_string()
}

#[test]
fn 属性按声明转换类型() {
    let app = app_with(
        r#"Component({ properties: {
            n: Number, s: String, b: Boolean, o: Object, a: Array, any: null,
            opt: { type: String, optionalTypes: [Number] }, dv: { type: Number, value: 7 }, bad: Number
        } });"#,
    );
    let r = reconcile(
        &app,
        r#"[{"key":"/x-c@0","tag":"x-c","owner":"","props":{"n":"5","s":12,"b":0,"o":3,"a":"x","any":[1],"opt":9,"bad":"abc","undeclared":1}}]"#,
    );
    assert_eq!(r, "1", "建了实例应报告有变化");
    let d = "__compState().map['/x-c@0'].data";
    assert_eq!(js(&app, &format!("typeof {d}.n + ':' + {d}.n")), "number:5");
    assert_eq!(js(&app, &format!("typeof {d}.s + ':' + {d}.s")), "string:12");
    assert_eq!(js(&app, &format!("{d}.b")), "false");
    assert_eq!(js(&app, &format!("{d}.o === null")), "true", "Object 类型收到数字给默认值 null");
    assert_eq!(js(&app, &format!("JSON.stringify({d}.a)")), "[]", "Array 类型收到字符串给默认值");
    assert_eq!(js(&app, &format!("JSON.stringify({d}.any)")), "[1]");
    assert_eq!(js(&app, &format!("typeof {d}.opt")), "number", "optionalTypes 里有 Number 就原样保留");
    assert_eq!(js(&app, &format!("{d}.dv")), "7", "没传的用 value 默认值");
    assert_eq!(js(&app, &format!("{d}.bad")), "0", "Number 收到非数字给默认值");
    assert_eq!(js(&app, &format!("'undeclared' in {d}")), "false", "没声明的不是属性，不进 data");
}

#[test]
fn 生命周期顺序与属性observer只在真变化时触发() {
    let app = app_with(
        r#"Component({
            properties: {
                count: { type: Number, value: 0, observer: function (n, o) { __log.push('obs:' + n + '<-' + o); } },
                label: { type: String, value: '' }
            },
            observers: {
                'count': function (c) { __log.push('watch:' + c); },
                'count, label': function (c, l) { __log.push('both:' + c + ',' + l); }
            },
            lifetimes: {
                created: function () { __log.push('created:' + this.data.count); },
                attached: function () { __log.push('attached:' + this.data.count); },
                ready: function () { __log.push('ready'); },
                detached: function () { __log.push('detached:' + this.data.count); }
            }
        });"#,
    );
    let occ = |count: u32, label: &str| {
        format!(r#"[{{"key":"/x-c@0","tag":"x-c","owner":"","props":{{"count":{count},"label":"{label}"}}}}]"#)
    };
    reconcile(&app, &occ(1, "a"));
    // created 看到默认值；属性写入在 created 之后、attached 之前（glass-easel 的顺序）
    assert_eq!(
        js(&app, "__log.join('|')"),
        "created:0|watch:1|both:1,a|obs:1<-0|attached:1|ready"
    );
    app.eval("__log = []").unwrap();
    assert_eq!(reconcile(&app, &occ(1, "a")), "0", "属性没变：不算变化");
    assert_eq!(js(&app, "__log.length"), "0", "属性没变不该触发任何 observer");
    reconcile(&app, &occ(2, "a"));
    assert_eq!(js(&app, "__log.join('|')"), "watch:2|both:2,a|obs:2<-1", "真变化各触发一次");
    app.eval("__log = []").unwrap();
    reconcile(&app, &occ(2, "b"));
    assert_eq!(js(&app, "__log.join('|')"), "both:2,b", "只变 label：count 的监听器不触发");
    app.eval("__log = []").unwrap();
    // 组件自己 setData 无关字段：都不触发；setData 属性：值变了才触发属性 observer
    app.eval("__compState().map['/x-c@0'].setData({ other: 1 })").unwrap();
    assert_eq!(js(&app, "__log.length"), "0");
    app.eval("__compState().map['/x-c@0'].setData({ count: '3' })").unwrap();
    assert_eq!(js(&app, "__log.join('|')"), "watch:3|both:3,b|obs:3<-2");
    assert_eq!(js(&app, "typeof __compState().map['/x-c@0'].data.count"), "number", "setData 属性也做类型转换");
    app.eval("__log = []").unwrap();
    // 不在了：detached；再出现：全新实例
    assert_eq!(reconcile(&app, "[]"), "1");
    assert_eq!(js(&app, "__log.join('|')"), "detached:3");
    assert_eq!(js(&app, "Object.keys(__compState().map).length"), "0");
    app.eval("__log = []").unwrap();
    reconcile(&app, &occ(2, "b"));
    assert!(js(&app, "__log[0]").starts_with("created:0"), "重新出现应是新实例：{}", js(&app, "__log.join('|')"));
}

#[test]
fn trigger_event到达使用方的绑定且detail保留类型() {
    let app = app_with(
        r#"Component({ properties: { n: Number },
            methods: { fire: function () { this.triggerEvent('pick', { n: this.data.n, list: [1, 2] }); } } });"#,
    );
    // 两个实例：一个归页面，一个归第一个实例（写在它模板里）
    reconcile(
        &app,
        r#"[{"key":"/x-c@0","tag":"x-c","owner":"","props":{"n":3},"id":"first","dataset":{"i":"4"},"events":{"pick":["onPick"]}},
            {"key":"/x-c@0/x-c@0","tag":"x-c","owner":"/x-c@0","props":{"n":8},"events":{"pick":["fire2"]}}]"#,
    );
    app.eval("__compState().map['/x-c@0'].fire()").unwrap();
    assert_eq!(js(&app, "__log.join('|')"), r#"page:pick:{"n":3,"list":[1,2]}:number:first:4"#);
    // 内层发的事件派发给外层实例（它没有 fire2 方法 → 不能误打到页面或自己身上）
    app.eval("__log = []; __compState().map['/x-c@0'].fire2 = function (e) { __log.push('outer:' + e.detail.n); }").unwrap();
    app.eval("__compState().map['/x-c@0/x-c@0'].fire()").unwrap();
    assert_eq!(js(&app, "__log.join('|')"), "outer:8");
}

#[test]
fn 事件按实例键派发_select_component按id与class找() {
    let app = app_with(
        r#"Component({ data: { hits: 0 }, methods: { tapMe: function () { this.setData({ hits: this.data.hits + 1 }); } },
            export: function () { return { mine: this.data.hits }; } });"#,
    );
    reconcile(
        &app,
        r#"[{"key":"/x-c@0","tag":"x-c","owner":"","props":{},"id":"a","class":"card big"},
            {"key":"/x-c@1","tag":"x-c","owner":"","props":{},"id":"b","class":"card"},
            {"key":"/x-c@1/x-c@0","tag":"x-c","owner":"/x-c@1","props":{},"class":"inner"}]"#,
    );
    app.eval("__dispatchEvent('tapMe', { type: 'tap' }, '/x-c@1')").unwrap();
    assert_eq!(js(&app, "__compState().map['/x-c@0'].data.hits"), "0", "第一个实例不该收到");
    assert_eq!(js(&app, "__compState().map['/x-c@1'].data.hits"), "1", "第二个实例收到");

    assert_eq!(js(&app, "__currentPage.selectComponent('#b').mine"), "1", "按 id 找，返回 export() 的结果");
    assert_eq!(js(&app, "__currentPage.selectAllComponents('.card').length"), "2");
    assert_eq!(js(&app, "__currentPage.selectAllComponents('.card.big').length"), "1");
    assert_eq!(js(&app, "__currentPage.selectComponent('.inner') === null"), "true", "页面只找自己模板里的");
    assert_eq!(js(&app, "__currentPage.selectAllComponents('.card >>> .inner').length"), "1", ">>> 跨组件往下找");
    assert_eq!(js(&app, "__compState().map['/x-c@1'].selectAllComponents('.inner').length"), "1");
    assert_eq!(js(&app, "__compState().map['/x-c@1/x-c@0'].selectOwnerComponent() === __compState().map['/x-c@1']"), "true");
}

/// pageLifetimes 发给本页**每一个**实例（从前一个标签只有一个实例）；别的页面树上的实例不收。
/// 卸载只回收这一页的实例，内层先于外层
#[test]
fn page_lifetimes发给每个实例_卸载只回收这一页的() {
    let app = app_with(
        r#"Component({ properties: { n: Number },
            pageLifetimes: { show: function () { __log.push('show:' + this.data.n); }, hide: function () { __log.push('hide:' + this.data.n); } },
            lifetimes: { detached: function () { __log.push('detached:' + this.data.n); } } });"#,
    );
    reconcile(
        &app,
        r#"[{"key":"/x-c@0","tag":"x-c","owner":"","props":{"n":1}},
            {"key":"/x-c@1","tag":"x-c","owner":"","props":{"n":2}},
            {"key":"/x-c@1/x-c@0","tag":"x-c","owner":"/x-c@1","props":{"n":3}}]"#,
    );
    // 另一页（栈里下面那页）的实例：挂在别的实例树上
    app.eval("__createComponentInstance('c/x', { n: 9 }, { key: '/x-c@0', tag: 'x-c', owner: '', tree: __newCompState({}) })")
        .unwrap();
    app.eval("__log = []; __dispatchPage('onShow')").unwrap();
    assert_eq!(js(&app, "__log.join('|')"), "show:1|show:2|show:3");
    app.eval("__log = []; __dispatchPage('onHide')").unwrap();
    assert_eq!(js(&app, "__log.join('|')"), "hide:1|hide:2|hide:3");
    app.eval("__log = []; __dispatchPage('onUnload')").unwrap();
    assert_eq!(js(&app, "__log.join('|')"), "detached:3|detached:2|detached:1", "内层先于外层，别的页面的不动");
    assert_eq!(js(&app, "Object.keys(__componentInstances).length"), "1", "别的页面的实例还在");
}

/// 属性 `value` 是数组、`data` 里是对象：每个实例拿一份**独立**拷贝（同一个组件现在会建很多实例）
#[test]
fn 默认值每个实例一份() {
    let app = app_with(r#"Component({ properties: { list: { type: Array, value: [1] } }, data: { box: { a: 1 } } });"#);
    reconcile(
        &app,
        r#"[{"key":"/x-c@0","tag":"x-c","owner":"","props":{}},{"key":"/x-c@1","tag":"x-c","owner":"","props":{}}]"#,
    );
    app.eval("var m = __compState().map; m['/x-c@0'].data.list.push(2); m['/x-c@0'].data.box.a = 5;").unwrap();
    assert_eq!(js(&app, "JSON.stringify(__compState().map['/x-c@1'].data.list)"), "[1]");
    assert_eq!(js(&app, "__compState().map['/x-c@1'].data.box.a"), "1");
}
