            // ==================== 页面内自定义组件（usingComponents） ====================
            // 实例按「每处使用一个」挂在页面的实例树上（component-tree.js），渲染层按**实例键**
            // 取组件自己的 data 去渲染组件模板：组件的 `{{a}}` 与页面的 `{{a}}` 是两套数据。
            //
            // 标签名 -> 该标签第一个实例的 id：老接口，跟着实例树同步（`__hasHandler` 与单测在用）
            var __pageComponents = {};

            /// 换页时作废**当前页**的组件实例（不跑 detached —— 与从前一致：页面还在栈里，
            /// 只是宿主会重建它的组件）。标签 -> 路径的登记保留：SDK 返回时靠它重建实例
            function __resetPageComponents() {
                var st = __compState();
                for (var k in st.map) {
                    if (st.map.hasOwnProperty(k) && st.map[k]) { delete __componentInstances[st.map[k].id]; }
                }
                st.map = {};
                st.order = [];
                __pageComponents = {};
            }

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
            /// 老式挂载：一个标签一个实例、实例键就是标签名（渲染层按标签名兜底取 data）。
            /// 宿主已改走 `__reconcileComponents`，这里留给单测与外部调用方
            function __mountPageComponent(tag, path, propsExpr) {
                var props = propsExpr ? __evalInPageData(propsExpr) : null;
                if (props === null || typeof props !== 'object') { props = {}; }
                var st = __compState();
                var inst = __createComponentInstance(path, props, { key: tag, tag: tag, owner: '', tree: st });
                if (!inst) { return ''; }
                if (st.order.indexOf(tag) < 0) { st.order.push(tag); }
                __pageComponents[tag] = inst.id;
                return inst.id;
            }

            /// 渲染数据 = 页面 data + `$comp`（实例键 -> 实例 data）+ `$copt`（标签 -> 组件选项）。
            /// `$copt` 只带展开时要用的 `multipleSlots` / `dynamicSlots`（slot 怎么分配）与 `virtualHost`（宿主节点出不出）
            function __getRenderData() {
                var out = {};
                if (__currentPage && __currentPage.data) {
                    for (var k in __currentPage.data) {
                        if (__currentPage.data.hasOwnProperty(k)) { out[k] = __currentPage.data[k]; }
                    }
                }
                var map = __compState().map;
                var comp = {};
                var opts = {};
                var any = false;
                var anyOpt = false;
                for (var key in map) {
                    if (!map.hasOwnProperty(key) || !map[key]) { continue; }
                    var inst = map[key];
                    comp[key] = inst.data;
                    any = true;
                    var entry = __componentDefs[inst.is];
                    var o = entry && entry.def && entry.def.options;
                    var bi = (entry && entry.merged.builtin) || [];
                    var ff = bi.indexOf('wx://form-field') >= 0, fg = bi.indexOf('wx://form-field-group') >= 0,
                        fb = bi.indexOf('wx://form-field-button') >= 0;
                    if ((o && (o.multipleSlots || o.virtualHost || o.dynamicSlots)) || ff || fg || fb) {
                        // __mergeHost：引擎内置组件专用（movable-view），模板根节点并入宿主（parser/template/component.rs）
                        opts[inst.__tag] = { multipleSlots: !!(o && o.multipleSlots), virtualHost: !!(o && o.virtualHost),
                                             dynamicSlots: !!(o && o.dynamicSlots), formField: ff, formFieldGroup: fg, formFieldButton: fb,
                                             __mergeHost: !!(o && o.virtualHost && o.__mergeHost) };
                        anyOpt = true;
                    }
                }
                if (any) { out['$comp'] = comp; }
                if (anyOpt) { out['$copt'] = opts; }
                return JSON.stringify(out);
            }

            /// 把事件派发到「持有该处理函数的对象」。
            ///
            /// `ownerKey` 是**声明这条绑定的组件实例键**（渲染层给出，见
            /// `COMPONENT_OWNER_ATTR`；slot 内容带的是使用方的键，页面为空串）。有它就先在
            /// 该实例上找 —— 这是微信语义：组件模板里的 `bindtap` 绑的是组件自己的方法，
            /// 绝不会打到页面上；同一个组件用两次，点第二个只会打到第二个实例。
            /// uni-app 给页面和每个组件各自生成 `e0_0`、`e1_1` 这种短名字，撞名是常态；
            /// 从前无条件先查页面，于是点底部导航执行的是**页面**的同名方法
            /// （tea-app 上的表现：点任何一个 tab 都跳到 AI 结果页）。
            /// 传进来的是标签名时（老接口）按标签取第一个实例。
            ///
            /// 找不到时再退回「页面 → 任意组件实例」的老顺序：框架产物偶尔把方法
            /// 挂在别处，宁可派发出去也别静默丢事件。
            function __dispatchHandler(name, event, ownerKey) {
                if (!name) { return false; }
                var st = __compState();
                if (ownerKey) {
                    var owner = st.map[ownerKey] ||
                        (__pageComponents.hasOwnProperty(ownerKey) ? __componentInstances[__pageComponents[ownerKey]] : null);
                    if (owner && typeof owner[name] === 'function') {
                        owner[name](event);
                        return true;
                    }
                }
                if (__currentPage && typeof __currentPage[name] === 'function') {
                    __currentPage[name](event);
                    return true;
                }
                for (var i = 0; i < st.order.length; i++) {
                    var inst = st.map[st.order[i]];
                    if (inst && typeof inst[name] === 'function') {
                        inst[name](event);
                        return true;
                    }
                }
                return false;
            }
