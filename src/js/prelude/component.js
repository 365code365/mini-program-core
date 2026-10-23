            var __componentDefs = {};       // path -> { def, merged }
            var __componentInstances = {};  // instanceId -> instance
            var __componentSeq = 0;
            var __pendingComponentPath = '';
            var __componentEventHandlers = {}; // instanceId:eventName -> pageMethodName
            
            function __setPendingComponentPath(p) { __pendingComponentPath = p || ''; }
            
            // Behavior 返回定义本身，供 behaviors 数组引用
            function Behavior(def) { return def || {}; }
            
            // 合并 behaviors 与顶层生命周期
            function __mergeBehaviors(def) {
                var result = {
                    properties: {}, data: {}, methods: {},
                    observers: {}, lifetimes: {}, pageLifetimes: {}
                };
                function mergeOne(src) {
                    if (!src) { return; }
                    // 内置 behavior（官方 custom-component/behaviors.html「内置 behaviors」）：
                    // wx://form-field 让组件当表单控件（有 name / value 两个属性，form 提交时收它的 value），
                    // wx://form-field-group 让 form 收组件内部的控件，wx://form-field-button 让组件里的
                    // form-type 按钮能提交 / 重置外层 form。后两个只是标记，交给渲染层（$copt）
                    if (typeof src === 'string') {
                        (result.builtin = result.builtin || []).push(src);
                        if (src === 'wx://form-field') {
                            if (!result.properties.name) { result.properties.name = { type: String, value: '' }; }
                            if (!result.properties.value) { result.properties.value = { type: null, value: null }; }
                        }
                        return;
                    }
                    if (src.behaviors) {
                        for (var i = 0; i < src.behaviors.length; i++) { mergeOne(src.behaviors[i]); }
                    }
                    var maps = ['properties', 'data', 'methods', 'observers', 'lifetimes', 'pageLifetimes'];
                    for (var m = 0; m < maps.length; m++) {
                        var name = maps[m];
                        if (src[name]) {
                            for (var k in src[name]) {
                                if (src[name].hasOwnProperty(k)) { result[name][k] = src[name][k]; }
                            }
                        }
                    }
                    // 旧式：生命周期直接写在 def 顶层
                    var top = ['created', 'attached', 'ready', 'moved', 'detached'];
                    for (var t = 0; t < top.length; t++) {
                        if (typeof src[top[t]] === 'function') { result.lifetimes[top[t]] = src[top[t]]; }
                    }
                }
                mergeOne(def);
                return result;
            }
            
            function Component(def) {
                def = def || {};
                var path = __pendingComponentPath || ('__comp_' + (++__componentSeq));
                __componentDefs[path] = { def: def, merged: __mergeBehaviors(def) };
                // 微信允许**用 Component 构造器定义页面**，uni-app / Taro 正是走这条：
                // 它们的 `wx.createPage` 把 Vue 组件选项转成组件定义后调 `Component()`，
                // 而不是 `Page()`。只认 `Page()` 的话页面永远注册不上 ——
                // `__currentPage` 为 null、data 全空，模板里 `{{a}}` 渲染成空串、
                // `bindtap="{{c}}"` 也拿不到处理函数名，最后只剩一张静态骨架。
                //
                // 判据是「当前正在加载某个页面的 js」（__pendingRoute 非空且还没建页面）。
                // 页面 js 里注册真正的自定义组件时 __pendingComponentPath 会被置上，
                // 那种情况不当页面处理。
                if (__pendingRoute && !__pendingPageCreated && !__pendingComponentPath) {
                    var __page = Page(__componentDefToPage(def));
                    // 组件实例该有的东西：`properties` 是**求值后的属性值**（不是类型表），
                    // uni-app 的 initVueIds 会去读 `properties.uI` 并 split；
                    // 还有 triggerEvent 等实例方法。缺了就在 attached 里抛 not a function。
                    var __props = {};
                    if (def.properties) {
                        for (var __p in def.properties) {
                            if (def.properties.hasOwnProperty(__p)) {
                                __props[__p] = __propDefault(def.properties[__p]);
                            }
                        }
                    }
                    __page.properties = __props;
                    __page.is = __pendingRoute;
                    if (typeof __page.triggerEvent !== 'function') {
                        __page.triggerEvent = function() {};
                    }
                    if (typeof __page.getRelationNodes !== 'function') {
                        __page.getRelationNodes = function() { return []; };
                    }
                    if (typeof __page.selectOwnerComponent !== 'function') {
                        __page.selectOwnerComponent = function() { return null; };
                    }
                    // 组件式页面的**创建钩子**必须真的跑：`created` / `attached` 是组件
                    // 自己的生命周期，与页面的 onLoad 是两套。uni-app 正是在 attached 里
                    // 挂载 Vue 组件并触发首次 setData —— 漏掉它页面就只有一份空 data，
                    // 模板全部渲染成空串（表现为「只有静态骨架」）。
                    var __lt = def.lifetimes || {};
                    var __hooks = [def.created, __lt.created, def.attached, __lt.attached];
                    for (var __i = 0; __i < __hooks.length; __i++) {
                        if (typeof __hooks[__i] === 'function') {
                            try { __hooks[__i].call(__page); }
                            catch (e) { if (typeof console !== 'undefined') { console.error(e); } }
                        }
                    }
                }
                return def;
            }

            /// 把「用 Component 写的页面」的定义摊平成 Page() 认识的形状。
            ///
            /// 组件构造器里页面生命周期（onLoad / onShow / onReady…）与事件处理函数都写在
            /// `methods` 下，而 Page() 期望它们在顶层；`lifetimes.attached` 对应页面的
            /// 创建时机，也一并映射过去。
            function __componentAsPageConfig(def) { return __componentDefToPage(def); }
            function __componentDefToPage(def) {
                var out = {};
                for (var k in def) {
                    if (!def.hasOwnProperty(k)) { continue; }
                    if (k === 'methods' || k === 'lifetimes' || k === 'pageLifetimes' || k === 'behaviors') { continue; }
                    out[k] = def[k];
                }
                if (def.methods) {
                    for (var m in def.methods) {
                        if (def.methods.hasOwnProperty(m)) { out[m] = def.methods[m]; }
                    }
                }
                // properties 的默认值也算页面初始 data（组件式页面常把状态放这儿）
                if (def.properties) {
                    out.data = out.data || {};
                    for (var p in def.properties) {
                        if (def.properties.hasOwnProperty(p) && !(p in out.data)) {
                            out.data[p] = __propDefault(def.properties[p]);
                        }
                    }
                }
                if (def.lifetimes) {
                    // attached ≈ 页面 onLoad 之前的创建期；没有 onLoad 时用它兜底
                    if (typeof def.lifetimes.attached === 'function' && typeof out.onLoad !== 'function') {
                        out.__attached = def.lifetimes.attached;
                    }
                    if (typeof def.lifetimes.ready === 'function' && typeof out.onReady !== 'function') {
                        out.onReady = def.lifetimes.ready;
                    }
                }
                return out;
            }
            
            // 解析 properties 默认值
            function __propDefault(prop) {
                if (prop === null || prop === undefined) { return null; }
                if (prop === String) { return ''; }
                if (prop === Number) { return 0; }
                if (prop === Boolean) { return false; }
                if (prop === Array) { return []; }
                if (prop === Object) { return null; }
                if (typeof prop === 'object') {
                    if (prop.value !== undefined) { return prop.value; }
                    var ty = prop.type;
                    if (ty === String) { return ''; }
                    if (ty === Number) { return 0; }
                    if (ty === Boolean) { return false; }
                    if (ty === Array) { return []; }
                    return null;
                }
                return null;
            }
            
            // 创建组件实例（供渲染层调用）。
            //
            // `meta` 是这处使用的身份（实例键、标签、归属、宿主 id/class/dataset、宿主上的事件绑定），
            // 由 `__reconcileComponents` 给出；必须在跑生命周期**之前**挂上 —— uni-app 在 attached
            // 里就 `triggerEvent('__l')` 找父级建立 Vue 父子关系，那时还不知道发给谁就丢了。
            //
            // 顺序按 glass-easel（component.ts）：data 先是各属性的默认值 → created →
            // 写入使用方传来的属性（触发 observer / observers）→ attached → ready。
            // 所以 created 里看到的是默认值，与微信一致。
            function __createComponentInstance(path, initialProps, meta) {
                var entry = __componentDefs[path];
                if (!entry) { return null; }
                var merged = entry.merged;
                var id = '__ci_' + (++__componentSeq);
                
                // 每个实例一份**深拷贝**：同一个组件现在会建出很多实例，共用定义里的
                // 数组/对象的话，一个实例 push 进去，其余实例的列表跟着变
                var data = {};
                for (var pk in merged.properties) {
                    if (merged.properties.hasOwnProperty(pk)) { data[pk] = __propSpec(merged.properties[pk]).dflt(); }
                }
                for (var dk in merged.data) {
                    if (merged.data.hasOwnProperty(dk)) { data[dk] = __compDeepCopy(merged.data[dk]); }
                }
                
                var instance = {
                    is: path,
                    id: id,
                    data: data,
                    properties: data,
                    dataset: {},
                    __isComponent: true,
                    __observers: merged.observers,
                    __rawProps: {},
                    setData: function(newData, callback) {
                        __compSetData(this, newData, callback);
                    },
                    triggerEvent: function(name, detail, options) {
                        __triggerComponentEvent(this, name, detail, options);
                    },
                    selectComponent: function(sel) { return __selectComponentsFrom(this.__tree, this.__key, sel, false); },
                    selectAllComponents: function(sel) { return __selectComponentsFrom(this.__tree, this.__key, sel, true); },
                    selectOwnerComponent: function() { return __ownerOf(this); },
                    createSelectorQuery: function() { return wx.createSelectorQuery().in(this); },
                    createIntersectionObserver: function(opt) { return wx.createIntersectionObserver(this, opt); }
                };
                if (meta) { __compApplyMeta(instance, meta); }
                // getPageId / router / pageRouter / animate / createMediaQueryObserver …（page-ext.js）
                if (typeof __decorateComponent === 'function') { __decorateComponent(instance); }
                
                for (var mk in merged.methods) {
                    if (merged.methods.hasOwnProperty(mk)) { instance[mk] = merged.methods[mk].bind(instance); }
                }
                
                __componentInstances[id] = instance;
                if (meta && meta.tree) { meta.tree.map[meta.key] = instance; }
                
                var lc = merged.lifetimes;
                if (typeof lc.created === 'function') { try { lc.created.call(instance); } catch (e) { __native_print('[Component.created] ' + e.message); } }
                __compApplyProps(instance, initialProps || {});
                if (typeof lc.attached === 'function') { try { lc.attached.call(instance); } catch (e) { __native_print('[Component.attached] ' + e.message); } }
                if (typeof lc.ready === 'function') { try { lc.ready.call(instance); } catch (e) { __native_print('[Component.ready] ' + e.message); } }
                
                return instance;
            }
            
            // 组件 triggerEvent -> **使用方**模板里的绑定（组件标签上的 `bind:x` / `catch:x`，冒泡时还有它的祖先）。
            //
            // 派发给使用方（页面，或外层组件实例 —— 组件写在另一个组件模板里时），且只找它：
            // 自定义事件不像触摸事件那样「找不到就随便找个同名方法」，那会打到发事件的组件自己身上。
            // `detail` 原样传引用（不经 JSON）：uni-app 的 `__l` 靠处理函数往 detail 上回写父级 vm。
            //
            // 选项（官方 custom-component/events.html 与 glass-easel）：
            // - `bubbles`：沿使用方模板里的祖先冒泡（`catch:x` 截断）；
            // - `composed`：可以穿过组件边界 —— 使用方是组件时，接着从**使用方的宿主**往外，派给使用方的使用方；
            // - `capturePhase`：先走一遍捕获（`capture-bind:x` / `capture-catch:x`，由外向内）。
            // 每一层的 `target` 都是该层看到的宿主节点（跨边界时重定向），`currentTarget` 是绑定所在节点。
            // 取舍：穿过边界时走的是「使用方宿主在它的使用方模板里的祖先」，不进入外层组件模板的内部节点
            // （slot 内容在外层组件内部的那段 composed 路径没有）。
            function __triggerComponentEvent(instance, name, detail, options) {
                options = options || {};
                var path = [];   // 由内向外：{ frame, owner, target }
                var level = instance, guard = 0;
                while (level && guard++ < 32) {
                    var chain = level.__chain;
                    if (!chain || !chain.length) {
                        // 没有冒泡路径（老接口、单测直接建的实例）：只有宿主上的绑定
                        var hs = (level.__events && level.__events[name]) || [];
                        var legacy = __componentEventHandlers[level.id + ':' + name];
                        if (!hs.length && legacy) hs = [legacy];
                        chain = [{ id: level.__hostId || '', dataset: level.dataset || {},
                                   ev: hs.length ? (function() { var e = {}; e[name] = hs.map(function(h) { return [h, 'b']; }); return e; })() : {} }];
                    }
                    var owner = __ownerOf(level);
                    var tgt = { id: chain[0].id || '', dataset: __coerceDataset(chain[0].dataset || {}) };
                    var frames = options.bubbles ? chain : chain.slice(0, 1);
                    for (var i = 0; i < frames.length; i++) path.push({ frame: frames[i], owner: owner, target: tgt });
                    if (!options.composed || !owner || !owner.__isComponent) break;
                    level = owner;
                }
                var stopped = false, mutDone = false;
                function fire(p, flags) {
                    var list = (p.frame.ev && p.frame.ev[name]) || [];
                    var cur = { id: p.frame.id || '', dataset: __coerceDataset(p.frame.dataset || {}) };
                    for (var j = 0; j < list.length; j++) {
                        var h = list[j][0], fl = list[j][1];
                        if (flags.indexOf(fl) < 0) continue;
                        if (fl === 'm') { if (mutDone) continue; mutDone = true; }
                        if (fl === 'c' || fl === 'X') stopped = true;
                        var fn = p.owner && p.owner[h];
                        if (typeof fn !== 'function') {
                            __native_print('[triggerEvent] ' + name + ': 使用方没有方法 ' + h);
                            continue;
                        }
                        var event = { type: name, timeStamp: Date.now(), detail: detail === undefined ? {} : detail,
                                      target: p.target, currentTarget: cur, mark: {}, touches: [], changedTouches: [] };
                        try { fn.call(p.owner, event); }
                        catch (e) { __native_print('[triggerEvent] ' + h + ': ' + e.message); }
                    }
                }
                if (options.capturePhase) {
                    for (var c = path.length - 1; c >= 0 && !stopped; c--) fire(path[c], 'CX');
                }
                // 宿主自己是事件目标：没走捕获阶段时，它身上的 capture-bind 也在这里触发（与从前一致）
                for (var b = 0; b < path.length && !stopped; b++) fire(path[b], b === 0 && !options.capturePhase ? 'CXbcm' : 'bcm');
            }
            
            // 渲染层登记组件事件绑定
            function __bindComponentEvent(instanceId, eventName, pageMethod) {
                __componentEventHandlers[instanceId + ':' + eventName] = pageMethod;
            }
            
            // ==================== 组件生命周期补全 ====================
            // 页面 show/hide/resize 联动全部组件实例的 pageLifetimes。
            // 挂在**别的页面**实例树上的跳过：出栈页面留下的实例（SDK 返回时不走 detached）
            // 不该再收到当前页的 show。不在任何树上的（直接 `__createComponentInstance` 建的）照发
            function __dispatchAllComponentsPageLifetime(name, arg) {
                for (var id in __componentInstances) {
                    if (!__componentInstances.hasOwnProperty(id)) { continue; }
                    var inst = __componentInstances[id];
                    if (inst.__tree && inst.__tree.page && inst.__tree.page !== __currentPage) { continue; }
                    var entry = __componentDefs[inst.is];
                    if (!entry) { continue; }
                    var pl = entry.merged.pageLifetimes;
                    if (pl && typeof pl[name] === 'function') {
                        try { pl[name].call(inst, arg); }
                        catch (e) { __native_print('[Component.pageLifetimes.' + name + '] ' + e.message); }
                    }
                }
            }
            
            // 组件从节点树移除：触发 detached 生命周期并回收实例
            function __detachComponentInstance(id) {
                var inst = __componentInstances[id];
                if (!inst) { return; }
                var entry = __componentDefs[inst.is];
                if (entry && typeof entry.merged.lifetimes.detached === 'function') {
                    try { entry.merged.lifetimes.detached.call(inst); }
                    catch (e) { __native_print('[Component.detached] ' + e.message); }
                }
                delete __componentInstances[id];
                if (inst.__tree && inst.__tree.map[inst.__key] === inst) { delete inst.__tree.map[inst.__key]; }
            }
            
            // 组件在节点树中被移动：触发 moved 生命周期
            function __moveComponentInstance(id) {
                var inst = __componentInstances[id];
                if (!inst) { return; }
                var entry = __componentDefs[inst.is];
                if (entry && typeof entry.merged.lifetimes.moved === 'function') {
                    try { entry.merged.lifetimes.moved.call(inst); }
                    catch (e) { __native_print('[Component.moved] ' + e.message); }
                }
            }
            
            // 页面卸载时回收**这一页**的全部组件实例（内层先于外层）。
            // 只认当前页实例树上的（和不在任何树上的老式实例）：从前是把全局表里的实例一锅端，
            // 栈里下面那页的组件也跟着 detached 了 —— 返回时它们的 data 已经没了
            function __detachAllComponents() {
                var st = __compState();
                var ids = [];
                for (var o = st.order.length - 1; o >= 0; o--) {
                    var x = st.map[st.order[o]];
                    if (x) { ids.push(x.id); }
                }
                for (var id in __componentInstances) {
                    if (!__componentInstances.hasOwnProperty(id) || ids.indexOf(id) >= 0) { continue; }
                    var t = __componentInstances[id].__tree;
                    if (!t || t === st) { ids.push(id); }
                }
                for (var i = 0; i < ids.length; i++) { __detachComponentInstance(ids[i]); }
                var st = __compState();
                st.map = {};
                st.order = [];
                __pageComponents = {};
            }
