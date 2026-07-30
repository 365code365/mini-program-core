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
            
            // 创建组件实例（供渲染层调用）
            function __createComponentInstance(path, initialProps) {
                var entry = __componentDefs[path];
                if (!entry) { return null; }
                var merged = entry.merged;
                var id = '__ci_' + (++__componentSeq);
                
                var data = {};
                for (var pk in merged.properties) {
                    if (merged.properties.hasOwnProperty(pk)) { data[pk] = __propDefault(merged.properties[pk]); }
                }
                for (var dk in merged.data) {
                    if (merged.data.hasOwnProperty(dk)) { data[dk] = merged.data[dk]; }
                }
                initialProps = initialProps || {};
                for (var ik in initialProps) {
                    if (initialProps.hasOwnProperty(ik)) { data[ik] = initialProps[ik]; }
                }
                
                var instance = {
                    is: path,
                    id: id,
                    data: data,
                    properties: data,
                    __isComponent: true,
                    __observers: merged.observers,
                    setData: function(newData, callback) {
                        __applySetData(this, newData, callback);
                        __runObservers(this, newData);
                    },
                    triggerEvent: function(name, detail, options) {
                        __triggerComponentEvent(this, name, detail, options);
                    },
                    selectComponent: function() { return null; },
                    selectAllComponents: function() { return []; }
                };
                
                for (var mk in merged.methods) {
                    if (merged.methods.hasOwnProperty(mk)) { instance[mk] = merged.methods[mk].bind(instance); }
                }
                
                __componentInstances[id] = instance;
                
                var lc = merged.lifetimes;
                if (typeof lc.created === 'function') { try { lc.created.call(instance); } catch (e) { __native_print('[Component.created] ' + e.message); } }
                if (typeof lc.attached === 'function') { try { lc.attached.call(instance); } catch (e) { __native_print('[Component.attached] ' + e.message); } }
                if (typeof lc.ready === 'function') { try { lc.ready.call(instance); } catch (e) { __native_print('[Component.ready] ' + e.message); } }
                
                return instance;
            }
            
            // 属性变化触发 observers
            function __runObservers(instance, changed) {
                if (!instance.__observers || !changed) { return; }
                for (var key in changed) {
                    if (!changed.hasOwnProperty(key)) { continue; }
                    var field = key.split('.')[0].split('[')[0];
                    var obs = instance.__observers[key] || instance.__observers[field];
                    if (typeof obs === 'function') {
                        try { obs.call(instance, changed[key]); } catch (e) { __native_print('[observer] ' + e.message); }
                    }
                }
            }
            
            // 组件 triggerEvent -> 宿主页面绑定的处理器
            function __triggerComponentEvent(instance, name, detail, options) {
                var key = instance.id + ':' + name;
                var handlerName = __componentEventHandlers[key];
                if (handlerName && __currentPage && typeof __currentPage[handlerName] === 'function') {
                    var event = {
                        type: name,
                        timeStamp: Date.now(),
                        detail: detail || {},
                        target: { id: instance.id, dataset: {} },
                        currentTarget: { id: instance.id, dataset: {} }
                    };
                    try { __currentPage[handlerName](event); }
                    catch (e) { __native_print('[triggerEvent] ' + e.message); }
                }
            }
            
            // 渲染层登记组件事件绑定
            function __bindComponentEvent(instanceId, eventName, pageMethod) {
                __componentEventHandlers[instanceId + ':' + eventName] = pageMethod;
            }
            
            // ==================== 组件生命周期补全 ====================
            // 页面 show/hide/resize 联动所有组件的 pageLifetimes
            function __dispatchAllComponentsPageLifetime(name, arg) {
                for (var id in __componentInstances) {
                    if (!__componentInstances.hasOwnProperty(id)) { continue; }
                    var inst = __componentInstances[id];
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
            
            // 页面卸载时回收其全部组件实例
            function __detachAllComponents() {
                var ids = [];
                for (var id in __componentInstances) {
                    if (__componentInstances.hasOwnProperty(id)) { ids.push(id); }
                }
                for (var i = 0; i < ids.length; i++) { __detachComponentInstance(ids[i]); }
            }
