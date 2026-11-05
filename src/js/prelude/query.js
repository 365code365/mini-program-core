            // SelectorQuery / IntersectionObserver / nextTick（官方 api/wxml/*）。
            //
            // 位置换算要布局树，所以查询本身由宿主做：`exec()` 把整串查询经
            // `__native_host_request('selectorQuery')` 交过去，宿主下一帧算好调 `__resolveSelectorQuery`
            // （与微信一样是异步回调）。IntersectionObserver 同理：登记交给宿主，宿主每帧算相交比例，
            // 跨过阈值时调 `__intersectionFire`。`in(component)` 只是返回自己：本引擎的 id / class 是页面级的。
            var __sqSeq = 0, __sqPending = {};
            function __hostReq(kind, obj) {
                if (typeof __native_host_request === 'function') __native_host_request(kind, JSON.stringify(obj));
            }
            function __WxNodesRef(sq, sel, all, viewport) { this._sq = sq; this._sel = sel; this._all = all; this._vp = viewport; }
            __WxNodesRef.prototype._push = function(fields, cb) {
                this._sq._q.push({ selector: this._sel, all: this._all, viewport: this._vp, fields: fields });
                this._sq._cbs.push(typeof cb === 'function' ? cb : null);
                return this._sq;
            };
            __WxNodesRef.prototype.boundingClientRect = function(cb) { return this._push({ id: true, dataset: true, rect: true, size: true }, cb); };
            __WxNodesRef.prototype.scrollOffset = function(cb) { return this._push({ id: true, dataset: true, scrollOffset: true }, cb); };
            __WxNodesRef.prototype.fields = function(fields, cb) { return this._push(fields || {}, cb); };
            __WxNodesRef.prototype.node = function(cb) { return this._push({ node: true }, cb); };
            __WxNodesRef.prototype.context = function(cb) { return this._push({ context: true }, cb); };
            function __WxSelectorQuery() { this._q = []; this._cbs = []; }
            __WxSelectorQuery.prototype.in = function() { return this; };
            __WxSelectorQuery.prototype.select = function(sel) { return new __WxNodesRef(this, String(sel), false, false); };
            __WxSelectorQuery.prototype.selectAll = function(sel) { return new __WxNodesRef(this, String(sel), true, false); };
            __WxSelectorQuery.prototype.selectViewport = function() { return new __WxNodesRef(this, '', false, true); };
            __WxSelectorQuery.prototype.exec = function(cb) {
                var id = ++__sqSeq;
                __sqPending[id] = { cbs: this._cbs, cb: typeof cb === 'function' ? cb : null };
                __hostReq('selectorQuery', { id: id, queries: this._q });
                return this;
            };
            wx.createSelectorQuery = function() { return new __WxSelectorQuery(); };

            // 宿主回填的「节点 / 上下文」描述换成真对象：canvas type=2d 的 Canvas、scroll-view 的
            // ScrollViewContext、video 的 VideoContext、旧 canvas 的 CanvasContext
            function __sqObject(d) {
                if (!d || typeof d !== 'object') return d;
                if (d.__node) {
                    var n = d.__node, out = {};
                    for (var k in d) if (k !== '__node') out[k] = d[k];
                    if (n.kind === 'canvas' && typeof __canvas2dNode === 'function') out.node = __canvas2dNode(n);
                    else if (n.kind === 'scroll-view') out.node = __scrollViewContext(n.id);
                    else if (n.kind === 'snapshot') out.node = __snapshotNode(n.cid || n.id);
                    else out.node = __builtinInstance(n.id, n.inst) || null;
                    d = out;
                }
                if (d.__ctx) {
                    var c = d.__ctx, o2 = {};
                    for (var k2 in d) if (k2 !== '__ctx') o2[k2] = d[k2];
                    if (c.kind === 'video') o2.context = wx.createVideoContext(c.id);
                    else if (c.kind === 'canvas') o2.context = wx.createCanvasContext(c.id);
                    else if (c.kind === 'map') o2.context = wx.createMapContext(c.id);
                    else if (c.kind === 'live-player') o2.context = wx.createLivePlayerContext(c.id);
                    else if (c.kind === 'camera') o2.context = wx.createCameraContext();
                    else if (c.kind === 'live-pusher') o2.context = wx.createLivePusherContext();
                    else if (c.kind === 'editor') { var ed = __builtinInstance(c.id); o2.context = ed && ed.__editorContext ? ed.__editorContext() : null; }
                    else o2.context = null;
                    d = o2;
                }
                return d;
            }
            globalThis.__resolveSelectorQuery = function(id, results) {
                var p = __sqPending[id];
                if (!p) return;
                delete __sqPending[id];
                var res = (results || []).map(function(r) { return Array.isArray(r) ? r.map(__sqObject) : __sqObject(r); });
                for (var i = 0; i < p.cbs.length; i++) {
                    if (p.cbs[i]) { try { p.cbs[i](res[i]); } catch (e) { console.error('SelectorQuery 回调出错', e); } }
                }
                if (p.cb) p.cb(res);
            };

            // ScrollViewContext（scroll-view 的 node，官方 api/ui/scroll/ScrollViewContext.html）：
            // scrollTo / scrollIntoView（alignment）/ triggerRefresh / closeRefresh；可写属性 scrollEnabled / bounces /
            // showScrollbar / pagingEnabled / fastDeceleration / decelerationDisabled 写了就发给宿主（host/selector_query.rs）。
            // 同一页同一个 scroll-view 拿到的是同一个对象（属性读回来是写过的值）
            var __svCtxs = {};
            function __scrollViewContext(id) {
                var key = (typeof getCurrentPages === 'function' ? getCurrentPages().length : 0) + '|' + id;
                if (__svCtxs[key]) return __svCtxs[key];
                var req = function(op, extra) {
                    var o = { id: id, op: op };
                    if (extra) for (var k in extra) o[k] = extra[k];
                    __hostReq('scrollView', o);
                };
                var ctx = {
                    scrollTo: function(opt) {
                        opt = opt || {};
                        req('scrollTo', { top: opt.top, left: opt.left, animated: opt.animated !== false, duration: opt.duration === undefined ? 300 : opt.duration });
                    },
                    scrollIntoView: function(selector, opt) {
                        opt = opt || {};
                        req('scrollIntoView', { selector: String(selector), offset: Number(opt.offset) || 0, alignment: opt.alignment || 'start', animated: opt.animated !== false });
                    },
                    triggerRefresh: function(opt) { req('triggerRefresh'); },
                    closeRefresh: function() { req('closeRefresh'); },
                    triggerTwoLevel: function(opt) { req('triggerTwoLevel'); },
                    closeTwoLevel: function(opt) { req('closeTwoLevel'); }
                };
                var props = { scrollEnabled: true, bounces: true, showScrollbar: true, pagingEnabled: false, fastDeceleration: false, decelerationDisabled: false };
                Object.keys(props).forEach(function(k) {
                    Object.defineProperty(ctx, k, {
                        enumerable: true,
                        get: function() { return props[k]; },
                        set: function(v) { v = !!v; if (props[k] === v) return; props[k] = v; req('set', { key: k, value: v }); }
                    });
                });
                __svCtxs[key] = ctx;
                return ctx;
            }

            // ---------- IntersectionObserver ----------
            var __ioSeq = 0, __ioCbs = {}, __ioOwners = {};
            function __WxIntersectionObserver(options) {
                options = options || {};
                this._opt = {
                    thresholds: Array.isArray(options.thresholds) && options.thresholds.length ? options.thresholds.map(Number) : [0],
                    initialRatio: Number(options.initialRatio) || 0,
                    observeAll: !!options.observeAll
                };
                this._rel = null;
                this._ids = [];
            }
            __WxIntersectionObserver.prototype.relativeTo = function(selector, margins) {
                this._rel = { selector: String(selector), margins: margins || {} };
                return this;
            };
            __WxIntersectionObserver.prototype.relativeToViewport = function(margins) {
                this._rel = { viewport: true, margins: margins || {} };
                return this;
            };
            __WxIntersectionObserver.prototype.observe = function(target, cb) {
                var id = ++__ioSeq;
                __ioCbs[id] = cb;
                this._ids.push(id);
                var page = (typeof getCurrentPages === 'function') ? getCurrentPages() : [];
                var owner = page.length ? page[page.length - 1] : null;
                __ioOwners[id] = owner;
                var route = owner ? (owner.route || '') : '';
                __hostReq('intersection', {
                    op: 'observe', id: id, target: String(target), rel: this._rel || { viewport: true, margins: {} },
                    thresholds: this._opt.thresholds, initialRatio: this._opt.initialRatio, observeAll: this._opt.observeAll,
                    route: route
                });
            };
            __WxIntersectionObserver.prototype.disconnect = function() {
                this._ids.forEach(function(id) { delete __ioCbs[id]; delete __ioOwners[id]; __hostReq('intersection', { op: 'disconnect', id: id }); });
                this._ids = [];
            };
            wx.createIntersectionObserver = function(component, options) {
                // 第一个参数可省（官方两种写法：wx.createIntersectionObserver(this, opt) / this.createIntersectionObserver(opt)）
                if (component && !options && (component.thresholds || component.initialRatio !== undefined || component.observeAll !== undefined)) {
                    options = component;
                }
                return new __WxIntersectionObserver(options);
            };
            globalThis.__intersectionFire = function(id, entry) {
                // 创建它的页面已经销毁（不在页面栈里了）：自动断开（微信在页面卸载时也会断开）
                var owner = __ioOwners[id];
                if (owner && typeof getCurrentPages === 'function' && getCurrentPages().indexOf(owner) < 0) {
                    delete __ioCbs[id]; delete __ioOwners[id];
                    __hostReq('intersection', { op: 'disconnect', id: id });
                    return;
                }
                var cb = __ioCbs[id];
                if (typeof cb === 'function') { try { cb(entry); } catch (e) { console.error('IntersectionObserver 回调出错', e); } }
            };

            // 推迟到当前同步代码之后（微任务）。宿主每次 eval 之后都会把微任务跑完
            wx.nextTick = function(fn) { if (typeof fn === 'function') Promise.resolve().then(fn); };

            // 按宿主 id 找当前页里的组件实例（引擎内置组件：draggable-sheet 的 scrollTo、editor 的 EditorContext）
            // `inst`：组件实例键（宿主从节点的 __inst 取）—— 没写 id 的节点（`select('.sheet').node()`）靠它
            function __builtinInstance(id, inst) {
                if (!__currentPage) return null;
                var st = __compStateOf(__currentPage);
                if (inst && st && st.map && st.map[inst]) return st.map[inst];
                if (!id || typeof __selectComponentsFrom !== 'function') return null;
                try { return __selectComponentsFrom(st, '', '#' + id, false); } catch (e) { return null; }
            }
            // <snapshot> 的 node：takeSnapshot({type: 'file' | 'arraybuffer', format: 'png' | 'rgba'})（官方 Snapshot.takeSnapshot，
            // 两者默认 file / png，不支持 Promise 风格）。宿主下一帧把子树离屏画出来（host::snapshot）：
            // file 回 {tempFilePath}，arraybuffer 回 {data}，另带 width / height（物理像素）
            var __snapSeq = 0, __snapTasks = {};
            function __snapshotNode(cid) {
                return {
                    takeSnapshot: function(o) {
                        o = o || {};
                        var seq = ++__snapSeq;
                        __snapTasks[seq] = o;
                        __hostReq('snapshot', { seq: seq, cid: cid, type: o.type === 'arraybuffer' ? 'arraybuffer' : 'file',
                                                format: o.format === 'rgba' ? 'rgba' : 'png' });
                    }
                };
            }
            globalThis.__snapshotDone = function(seq, ok, res) {
                var o = __snapTasks[seq];
                if (!o) return;
                delete __snapTasks[seq];
                if (ok) {
                    if (typeof res.base64 === 'string') { res.data = wx.base64ToArrayBuffer(res.base64); delete res.base64; }
                    res.errMsg = 'takeSnapshot:ok';
                }
                try { (ok ? o.success : o.fail) && (ok ? o.success : o.fail)(res); } catch (e) { console.error(e); }
                try { o.complete && o.complete(res); } catch (e) { console.error(e); }
            };
