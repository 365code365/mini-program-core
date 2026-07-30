            // ==================== 页面内自定义组件（usingComponents） ====================
            // 标签名 -> 组件实例 id。渲染层按标签名取组件自己的 data 去渲染组件模板：
            // 组件的 `{{a}}` 与页面的 `{{a}}` 是两套数据，不能共用一个作用域。
            var __pageComponents = {};

            function __resetPageComponents() { __pageComponents = {}; }

            /// 在页面数据作用域里求一个表达式（用于 `u-p="{{r||''}}"` 这类「父传子」属性）
            function __evalInPageData(exprSrc) {
                try {
                    var d = (__currentPage && __currentPage.data) || {};
                    var f = new Function('d', 'with (d) { return (' + exprSrc + '); }');
                    return f(d);
                } catch (e) { return null; }
            }

            /// 挂载一个页面内自定义组件：建实例（会跑 created/attached/ready）并记下标签名。
            /// 框架型产物（uni-app / Taro）正是在 attached 里挂载自己的组件并触发首次 setData，
            /// 所以实例必须真的建出来，光有定义没有实例的话组件模板里全是空值。
            function __mountPageComponent(tag, path, propsExpr) {
                var props = propsExpr ? __evalInPageData(propsExpr) : null;
                if (props === null || typeof props !== 'object') { props = {}; }
                var inst = __createComponentInstance(path, props);
                if (!inst) { return ''; }
                __pageComponents[tag] = inst.id;
                return inst.id;
            }

            /// 渲染数据 = 页面 data + `$comp`（各组件实例的 data）
            function __getRenderData() {
                var out = {};
                if (__currentPage && __currentPage.data) {
                    for (var k in __currentPage.data) {
                        if (__currentPage.data.hasOwnProperty(k)) { out[k] = __currentPage.data[k]; }
                    }
                }
                var comp = {};
                var any = false;
                for (var tag in __pageComponents) {
                    if (!__pageComponents.hasOwnProperty(tag)) { continue; }
                    var inst = __componentInstances[__pageComponents[tag]];
                    if (inst) { comp[tag] = inst.data; any = true; }
                }
                if (any) { out['$comp'] = comp; }
                return JSON.stringify(out);
            }

            /// 把事件派发到「持有该处理函数的对象」。
            ///
            /// `ownerTag` 是**声明这条绑定的组件标签**（渲染层给出，见
            /// `COMPONENT_OWNER_ATTR`）。有它就先在该组件实例上找 —— 这是微信语义：
            /// 组件模板里的 `bindtap` 绑的是组件自己的方法，绝不会打到页面上。
            /// uni-app 给页面和每个组件各自生成 `e0_0`、`e1_1` 这种短名字，撞名是常态；
            /// 从前无条件先查页面，于是点底部导航执行的是**页面**的同名方法
            /// （tea-app 上的表现：点任何一个 tab 都跳到 AI 结果页）。
            ///
            /// 找不到时再退回「页面 → 任意组件实例」的老顺序：框架产物偶尔把方法
            /// 挂在别处，宁可派发出去也别静默丢事件。
            function __dispatchHandler(name, event, ownerTag) {
                if (!name) { return false; }
                if (ownerTag && __pageComponents.hasOwnProperty(ownerTag)) {
                    var owner = __componentInstances[__pageComponents[ownerTag]];
                    if (owner && typeof owner[name] === 'function') {
                        owner[name](event);
                        return true;
                    }
                }
                if (__currentPage && typeof __currentPage[name] === 'function') {
                    __currentPage[name](event);
                    return true;
                }
                for (var tag in __pageComponents) {
                    if (!__pageComponents.hasOwnProperty(tag)) { continue; }
                    var inst = __componentInstances[__pageComponents[tag]];
                    if (inst && typeof inst[name] === 'function') {
                        inst[name](event);
                        return true;
                    }
                }
                return false;
            }
