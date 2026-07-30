            // ==================== 完整生命周期分发 ====================
            // App 级生命周期：onLaunch/onShow/onHide/onError/onPageNotFound/
            //                 onUnhandledRejection/onThemeChange
            function __dispatchApp(name, arg) {
                // `App({onError})` 之外，还有用 `wx.onError(cb)` 注册的监听者（微信里两者并存）
                var extra = __appHooks[name];
                if (extra) {
                    for (var i = 0; i < extra.length; i++) {
                        try { extra[i](arg); }
                        catch (e) { __native_print('[wx.' + name + '] ' + e.message); }
                    }
                }
                if (__app && typeof __app[name] === 'function') {
                    try { return __app[name](arg); }
                    catch (e) { __native_print('[App.' + name + '] ' + e.message); }
                }
            }
            
            // Page 级生命周期：onLoad/onShow/onReady/onHide/onUnload/
            //   onPullDownRefresh/onReachBottom/onPageScroll/onResize/
            //   onTabItemTap/onShareAppMessage/onShareTimeline/onAddToFavorites
            // 同时联动页面内组件的 pageLifetimes(show/hide/resize)。
            function __dispatchPage(name, arg) {
                var result;
                if (__currentPage && typeof __currentPage[name] === 'function') {
                    try { result = __currentPage[name](arg); }
                    catch (e) { __native_print('[Page.' + name + '] ' + e.message); }
                }
                var mapped = { onShow: 'show', onHide: 'hide', onResize: 'resize' };
                if (mapped[name] && typeof __dispatchAllComponentsPageLifetime === 'function') {
                    __dispatchAllComponentsPageLifetime(mapped[name], arg);
                }
                // 页面卸载：销毁其组件实例，触发 detached
                if (name === 'onUnload' && typeof __detachAllComponents === 'function') {
                    __detachAllComponents();
                }
                return result;
            }
            
            // 调用页面方法（供 native 调用事件处理）。
            // 处理函数可能挂在页面上，也可能挂在页面内的自定义组件实例上
            // （组件模板里的 bindtap 绑的是组件自己的方法）。
            function __callPageMethod(methodName, eventData) {
                if (__hasHandler(methodName)) {
                    // 确保 eventData 是对象
                    if (typeof eventData === 'string') {
                        try {
                            eventData = JSON.parse(eventData);
                        } catch (e) {
                            eventData = {};
                        }
                    }
                    eventData = eventData || {};
                    // dataset 的字面量还原与触摸链路共用一份实现（见 __coerceDataset）
                    var dataset = __coerceDataset(eventData);
                    var targetInfo = { id: '', offsetLeft: 0, offsetTop: 0, dataset: dataset };
                    var event = {
                        type: 'tap',
                        timeStamp: Date.now(),
                        target: targetInfo,
                        currentTarget: targetInfo,
                        detail: dataset,
                        touches: [],
                        changedTouches: []
                    };
                    try {
                        __dispatchHandler(methodName, event);
                    } catch (e) {
                        __native_print('[Error] ' + methodName + ': ' + (e && e.message ? e.message : e));
                    }
                    return true;
                }
                return false;
            }

            /// 派发一个**原生侧拼好的完整事件对象**（touchstart/touchmove/touchend/
            /// touchcancel/longpress/tap 都走这里）。
            ///
            /// 与 `__callPageMethod` 的分工：那个是「只有 dataset、事件对象由 JS 兜」的
            /// 老入口（picker、交互组件回调仍在用）；触摸链路的事件对象必须由原生构造 ——
            /// `touches` / `changedTouches` 的坐标、`target` 与 `currentTarget` 的区分
            /// 只有渲染层知道。
            function __dispatchEvent(handlerName, event, ownerTag) {
                if (!handlerName) { return false; }
                event = event || {};
                if (event.target) { event.target.dataset = __coerceDataset(event.target.dataset); }
                if (event.currentTarget) { event.currentTarget.dataset = __coerceDataset(event.currentTarget.dataset); }
                if (typeof event.timeStamp !== 'number') { event.timeStamp = Date.now(); }
                // `detail` 由原生按事件类型拼好，**不过** `__coerceDataset` ——
                // 输入框的 `detail.value` 必须保持字符串（微信语义），
                // 走 dataset 的字面量还原会把 "1212121" 变成数字。
                event.detail = event.detail || {};
                event.touches = event.touches || [];
                event.changedTouches = event.changedTouches || [];
                try {
                    return __dispatchHandler(handlerName, event, ownerTag);
                } catch (e) {
                    __native_print('[Error] ' + handlerName + ': ' + (e && e.message ? e.message : e));
                    return false;
                }
            }

            /// `data-*` 都是字符串，微信会按字面量还原成数字/对象/数组。
            /// 列表项常写 `data-id="12"`，页面里直接和数字比较 —— 不还原就永远不相等。
            function __coerceDataset(raw) {
                var out = {};
                if (!raw) { return out; }
                for (var key in raw) {
                    if (!raw.hasOwnProperty(key)) { continue; }
                    var val = raw[key];
                    if (typeof val !== 'string') { out[key] = val; continue; }
                    if ((val.charAt(0) === '{' && val.charAt(val.length - 1) === '}') ||
                        (val.charAt(0) === '[' && val.charAt(val.length - 1) === ']')) {
                        try { out[key] = JSON.parse(val.replace(/'/g, '"')); }
                        catch (e) { out[key] = val; }
                    } else if (/^-?\d+(\.\d+)?$/.test(val)) {
                        out[key] = parseFloat(val);
                    } else {
                        out[key] = val;
                    }
                }
                return out;
            }

            /// 页面或其组件实例上是否存在该处理函数
            function __hasHandler(name) {
                if (!name) { return false; }
                if (__currentPage && typeof __currentPage[name] === 'function') { return true; }
                for (var tag in __pageComponents) {
                    if (!__pageComponents.hasOwnProperty(tag)) { continue; }
                    var inst = __componentInstances[__pageComponents[tag]];
                    if (inst && typeof inst[name] === 'function') { return true; }
                }
                return false;
            }
            
            // 获取页面数据（供 native 调用）
            function __getPageData() {
                if (__currentPage) {
                    return JSON.stringify(__currentPage.data);
                }
                return '{}';
            }
