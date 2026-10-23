            // ───────── wx.worklet（Skyline 官方 api/ui/worklet）与 applyAnimatedStyle、wx.router ─────────
            // 微信里 worklet 函数跑在 UI 线程。本引擎只有一条逻辑线程，所以它们就在这里跑：
            // - shared(v)：共享值，`.value` 可读写；写入一个动画描述（timing / spring / decay / sequence / repeat / delay）就开始动；
            // - derived(fn)：派生值，任何共享值变了重算；
            // - this.applyAnimatedStyle(selector, updater)：updater 返回样式对象，共享值一变重算，写进节点样式
            //   （与 this.animate 同一个出口：data 的 __nodeStyles，见 animate.js）；
            // - runOnJS / runOnUI：返回「下一轮调用它」的函数（两边是同一条线程，语义上仍是异步）。
            // 帧循环与 animate 共用（__onFrame，16ms 一步，按墙上时钟算进度）。
            // 取舍：worklet 指令（'worklet'）照常是普通函数；手势回调里改共享值会经一次 setData 才上屏（多一帧）。
            var __wk = wx.worklet = {};
            var __sharedSeq = 0, __derived = [], __animatedStyles = [], __reactDirty = false, __inDerive = false;
            function __markReact() {
                // 派生值同步重算（回调里读到的就是新的）；它们自己变了也走到这里，靠 __inDerive 不递归
                if (!__inDerive) {
                    __inDerive = true;
                    try { for (var i = 0; i < __derived.length; i++) __derived[i].__recompute(); } finally { __inDerive = false; }
                }
                if (__reactDirty) return;
                __reactDirty = true;
                // 样式每帧合并重算一次（一帧里改多个共享值只 setData 一次）
                __onFrame(function() {
                    __reactDirty = false;
                    var touched = [];
                    for (var j = 0; j < __animatedStyles.length; j++) {
                        var s = __animatedStyles[j];
                        if (s.__update() && touched.indexOf(s.inst) < 0) touched.push(s.inst);
                    }
                    touched.forEach(function(inst) { __nsFlush(inst); });
                    return false;
                });
            }
            function __isAnim(v) { return v && typeof v === 'object' && v.__wkAnim; }
            // 动画描述 → 跑起来的实例：step(now) → { v, done }
            function __runAnim(d, from, now) {
                var cbDone = function(finished, cur) { if (typeof d.cb === 'function') { try { d.cb(finished, cur); } catch (e) { console.error(e); } } };
                if (d.kind === 'timing') {
                    var dur = Math.max(0, d.opt.duration === undefined ? 300 : Number(d.opt.duration));
                    var ease = d.opt.easing || __Easing.inOut(__Easing.quad), to = d.to;
                    return { step: function(t) {
                        var p = dur > 0 ? Math.min(1, (t - now) / dur) : 1;
                        var v = typeof from === 'number' && typeof to === 'number' ? from + (to - from) * ease(p) : __lerpValueExt(from, to, ease(p));
                        return { v: p >= 1 ? to : v, done: p >= 1 };
                    }, cb: cbDone };
                }
                if (d.kind === 'spring') {
                    var o = d.opt, k = o.stiffness === undefined ? 100 : o.stiffness, c = o.damping === undefined ? 10 : o.damping;
                    var m = o.mass === undefined ? 1 : o.mass, x = Number(from) || 0, vel = Number(o.velocity) || 0, last = now, target = d.to;
                    var restD = o.restDisplacementThreshold === undefined ? 0.01 : o.restDisplacementThreshold;
                    var restV = o.restSpeedThreshold === undefined ? 2 : o.restSpeedThreshold;
                    return { step: function(t) {
                        var steps = Math.min(64, Math.max(1, Math.round(t - last)));
                        for (var i = 0; i < steps; i++) {
                            var a = (-k * (x - target) - c * vel) / m;
                            vel += a * 0.001; x += vel * 0.001;
                            if (o.overshootClamping && ((from < target && x > target) || (from > target && x < target))) { x = target; vel = 0; }
                        }
                        last = t;
                        var done = Math.abs(vel) < restV && Math.abs(x - target) < restD;
                        return { v: done ? target : x, done: done };
                    }, cb: cbDone };
                }
                if (d.kind === 'decay') {
                    var dv = Number(d.opt.velocity) || 0, dec = d.opt.deceleration === undefined ? 0.998 : d.opt.deceleration;
                    var pos = Number(from) || 0, lt = now, clamp = d.opt.clamp;
                    return { step: function(t) {
                        var dt = Math.max(0, t - lt); lt = t;
                        pos += dv * dt / 1000; dv *= Math.pow(dec, dt);
                        var done = Math.abs(dv) < 0.5;
                        if (Array.isArray(clamp)) {
                            if (pos <= clamp[0]) { pos = clamp[0]; done = true; }
                            if (pos >= clamp[1]) { pos = clamp[1]; done = true; }
                        }
                        return { v: pos, done: done };
                    }, cb: cbDone };
                }
                if (d.kind === 'delay') {
                    var inner = null;
                    return { step: function(t) {
                        if (t - now < d.ms) return { v: from, done: false };
                        if (!inner) inner = __runAnim(d.anim, from, now + d.ms);
                        var r = inner.step(t);
                        if (r.done) inner.cb(true, r.v);
                        return r;
                    }, cb: cbDone };
                }
                if (d.kind === 'sequence') {
                    var idx = 0, cur = null, val = from, base = now;
                    return { step: function(t) {
                        while (idx < d.list.length) {
                            if (!cur) cur = __runAnim(d.list[idx], val, base);
                            var r = cur.step(t);
                            val = r.v;
                            if (!r.done) return { v: val, done: false };
                            cur.cb(true, val); cur = null; idx++; base = t;
                        }
                        return { v: val, done: true };
                    }, cb: cbDone };
                }
                if (d.kind === 'repeat') {
                    // 不 reverse：每一轮都从起点重来；reverse：来回交替（回程的终点是起点）。n <= 0 无限次
                    var n = d.n, round = 0, run = null, origin = from, forward = true, b0 = now, to = d.anim && d.anim.to;
                    return { step: function(t) {
                        if (!run) {
                            var desc = d.anim;
                            if (!forward && to !== undefined) { desc = {}; for (var q in d.anim) desc[q] = d.anim[q]; desc.to = origin; }
                            run = __runAnim(desc, forward ? origin : to, b0);
                        }
                        var r = run.step(t);
                        if (!r.done) return { v: r.v, done: false };
                        round++; run = null; b0 = t;
                        if (d.reverse) forward = !forward;
                        return { v: r.v, done: n > 0 && round >= n };
                    }, cb: cbDone };
                }
                return { step: function() { return { v: d.to, done: true }; }, cb: cbDone };
            }
            // 非数值（颜色、带单位的串、数组）按 animate.js 的插值规则
            function __lerpValueExt(a, b, t) { return __lerpValue(a, b, t); }
            function __SharedValue(v) {
                this.__id = ++__sharedSeq;
                this.__v = v; this.__run = null;
            }
            Object.defineProperty(__SharedValue.prototype, 'value', {
                get: function() { return this.__v; },
                set: function(v) {
                    var self = this;
                    if (self.__run) { var old = self.__run; self.__run = null; old.cb(false, self.__v); }
                    if (!__isAnim(v)) { if (v !== self.__v) { self.__v = v; __markReact(); } return; }
                    var run = __runAnim(v, self.__v, Date.now());
                    self.__run = run;
                    __onFrame(function(now) {
                        if (self.__run !== run) return false;
                        var r = run.step(now);
                        if (r.v !== self.__v) { self.__v = r.v; __markReact(); }
                        if (r.done) { self.__run = null; run.cb(true, r.v); return false; }
                        return true;
                    });
                }
            });
            __SharedValue.prototype.get = function() { return this.__v; };
            __SharedValue.prototype.set = function(v) { this.value = v; };
            __wk.shared = function(v) { return new __SharedValue(v); };
            __wk.derived = function(fn) {
                var d = new __SharedValue(undefined);
                d.__recompute = function() { var v; try { v = fn(); } catch (e) { console.error(e); return; } if (v !== d.__v) { d.__v = v; __markReact(); } };
                d.__recompute();
                __derived.push(d);
                return d;
            };
            __wk.cancelAnimation = function(sv) { if (sv && sv.__run) { var r = sv.__run; sv.__run = null; r.cb(false, sv.__v); } };
            __wk.runOnJS = function(fn) { return function() { var a = arguments; setTimeout(function() { fn.apply(null, a); }, 0); }; };
            __wk.runOnUI = function(fn) { return function() { var a = arguments; setTimeout(function() { fn.apply(null, a); }, 0); }; };
            var anim = function(o) { o.__wkAnim = true; return o; };
            __wk.timing = function(to, opt, cb) { if (typeof opt === 'function') { cb = opt; opt = {}; } return anim({ kind: 'timing', to: to, opt: opt || {}, cb: cb }); };
            __wk.spring = function(to, opt, cb) { if (typeof opt === 'function') { cb = opt; opt = {}; } return anim({ kind: 'spring', to: to, opt: opt || {}, cb: cb }); };
            __wk.decay = function(opt, cb) { if (typeof opt === 'function') { cb = opt; opt = {}; } return anim({ kind: 'decay', opt: opt || {}, cb: cb }); };
            __wk.delay = function(ms, a) { return anim({ kind: 'delay', ms: Number(ms) || 0, anim: a, to: a && a.to }); };
            __wk.sequence = function() { var list = Array.prototype.slice.call(arguments).filter(__isAnim); return anim({ kind: 'sequence', list: list }); };
            __wk.repeat = function(a, n, reverse, cb) { return anim({ kind: 'repeat', anim: a, n: n === undefined ? 2 : Number(n), reverse: !!reverse, cb: cb, to: a && a.to }); };
            // Easing（与 Reanimated / 官方同名同参）
            var __Easing = __wk.Easing = {
                linear: function(t) { return t; },
                ease: __bezierEase(0.42, 0, 1, 1),
                quad: function(t) { return t * t; },
                cubic: function(t) { return t * t * t; },
                poly: function(n) { return function(t) { return Math.pow(t, n); }; },
                sin: function(t) { return 1 - Math.cos(t * Math.PI / 2); },
                circle: function(t) { return 1 - Math.sqrt(1 - t * t); },
                exp: function(t) { return Math.pow(2, 10 * (t - 1)); },
                elastic: function(b) { var p = (b === undefined ? 1 : b) * Math.PI; return function(t) { return 1 - Math.pow(Math.cos(t * Math.PI / 2), 3) * Math.cos(t * p); }; },
                back: function(s) { s = s === undefined ? 1.70158 : s; return function(t) { return t * t * ((s + 1) * t - s); }; },
                bounce: function(t) {
                    if (t < 1 / 2.75) return 7.5625 * t * t;
                    if (t < 2 / 2.75) { t -= 1.5 / 2.75; return 7.5625 * t * t + 0.75; }
                    if (t < 2.5 / 2.75) { t -= 2.25 / 2.75; return 7.5625 * t * t + 0.9375; }
                    t -= 2.625 / 2.75; return 7.5625 * t * t + 0.984375;
                },
                bezier: function(x1, y1, x2, y2) { return __bezierEase(x1, y1, x2, y2); },
                bezierFn: function(x1, y1, x2, y2) { return __bezierEase(x1, y1, x2, y2); },
                in: function(e) { return e; },
                out: function(e) { return function(t) { return 1 - e(1 - t); }; },
                inOut: function(e) { return function(t) { return t < 0.5 ? e(t * 2) / 2 : 1 - e((1 - t) * 2) / 2; }; }
            };
            // worklet.scrollViewContext.scrollTo(scrollView 的 node, {top, left, duration, animated})
            __wk.scrollViewContext = {
                scrollTo: function(node, o) { if (node && typeof node.scrollTo === 'function') node.scrollTo(o || {}); }
            };

            // 页面 / 组件实例上的 applyAnimatedStyle / clearAnimatedStyle
            var __styleIdSeq = 0;
            globalThis.__attachWorkletStyle = function(inst) {
                inst.__nsWorklet = {};
                var mine = {};   // styleId → 登记项
                var rebuild = function(sel) {
                    var css = '';
                    for (var id in mine) if (mine[id].sel === sel) css += mine[id].css;
                    if (css) inst.__nsWorklet[sel] = css; else delete inst.__nsWorklet[sel];
                };
                inst.applyAnimatedStyle = function(selector, updater, config, callback) {
                    if (typeof config === 'function') { callback = config; config = null; }
                    var sel = String(selector || ''), id = ++__styleIdSeq;
                    var entry = { inst: inst, sel: sel, css: '' };
                    entry.__update = function() {
                        var o; try { o = updater(); } catch (e) { console.error(e); return false; }
                        var css = __styleText(o || {});
                        if (css === entry.css) return false;
                        entry.css = css; rebuild(sel); return true;
                    };
                    mine[id] = entry;
                    __animatedStyles.push(entry);
                    entry.__update();
                    __nsFlush(inst);
                    var res = { styleId: id };
                    if (typeof callback === 'function') setTimeout(function() { callback(res); }, 0);
                    return res;
                };
                inst.clearAnimatedStyle = function(selector, styleIds, callback) {
                    if (typeof styleIds === 'function') { callback = styleIds; styleIds = null; }
                    var sel = String(selector || '');
                    for (var id in mine) {
                        if (mine[id].sel !== sel) continue;
                        if (Array.isArray(styleIds) && styleIds.indexOf(Number(id)) < 0) continue;
                        var e = mine[id];
                        __animatedStyles = __animatedStyles.filter(function(x) { return x !== e; });
                        delete mine[id];
                    }
                    rebuild(sel);
                    __nsFlush(inst);
                    if (typeof callback === 'function') setTimeout(callback, 0);
                };
            };
            // animate.js 的 __attachAnimate 之后再挂这两个（页面 / 组件都经它）
            (function() {
                var raw = __attachAnimate;
                __attachAnimate = function(inst) { raw(inst); __attachWorkletStyle(inst); };
            })();

            // ───── wx.router（Skyline 自定义路由，官方 api/route/router）─────
            // 路由转场仍是引擎自己的那套（route_anim.rs）：addRouteBuilder 登记的构建函数不参与转场，
            // navigateTo({ routeType }) 照常打开页面（写明的降级）；getRouteContext 给一份共享值，转场结束时置 1。
            var __routeBuilders = {};
            wx.router = {
                addRouteBuilder: function(type, builder) { __routeBuilders[String(type)] = builder; },
                removeRouteBuilder: function(type) { delete __routeBuilders[String(type)]; },
                getRouteContext: function(inst) {
                    if (!inst) return null;
                    if (!inst.__routeContext) {
                        inst.__routeContext = {
                            primaryAnimation: __wk.shared(1), primaryAnimationStatus: __wk.shared(0),
                            secondaryAnimation: __wk.shared(0), secondaryAnimationStatus: __wk.shared(0),
                            userGestureInProgress: __wk.shared(0),
                            startUserGesture: function() { inst.__routeContext.userGestureInProgress.value = 1; },
                            stopUserGesture: function() { inst.__routeContext.userGestureInProgress.value = 0; },
                            didPop: function() { wx.navigateBack({}); }
                        };
                    }
                    return inst.__routeContext;
                }
            };
