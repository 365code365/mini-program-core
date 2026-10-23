            // ───────── 按选择器改节点样式：this.animate / clearAnimation（官方 api/ui/animation/animate）─────────
            // 写进页面 / 组件实例 data 的 `__nodeStyles`（选择器 → 样式文本），模板展开时拼到命中节点的 style 后面
            // （parser/template/node_styles.rs）。所以每一帧就是一次 setData：增量失效、局部重画照常生效。
            // worklet 的 applyAnimatedStyle（worklet.js）也写这里，两边的样式按选择器合并。
            //
            // 关键帧：offset（缺省均分）、ease（这一段的缓动：linear / ease / ease-in / ease-out / ease-in-out /
            // step-start / step-end / cubic-bezier(...)）、其余是样式（驼峰）：opacity、backgroundColor、width / height /
            // left / top / right / bottom（数字按 px）、translate / translateX / rotate / scale / skew / matrix …（合成 transform）。
            // 数值、带单位的数值、颜色、数组逐项插值；别的值在一段的中点跳变。动画结束后最终样式保留，clearAnimation 才清
            // （官方语义）。第四个参数是滚动时间线 {scrollSource, startScrollOffset, endScrollOffset, timeRange} 时，
            // 进度跟着那个 scroll-view 的滚动位置走（每 50ms 查一次位置）。
            // 取舍：3D 变换（rotateX / translateZ / matrix3d …）交给引擎的 transform 解析，CPU 端按正交投影近似；composite 不支持。
            var __TRANSFORM_KEYS = { matrix: 1, matrix3d: 1, rotate: 1, rotate3d: 1, rotateX: 1, rotateY: 1, rotateZ: 1,
                scale: 1, scale3d: 1, scaleX: 1, scaleY: 1, scaleZ: 1, skew: 1, skewX: 1, skewY: 1,
                translate: 1, translate3d: 1, translateX: 1, translateY: 1, translateZ: 1 };
            var __LEN_KEYS = { width: 1, height: 1, left: 1, top: 1, right: 1, bottom: 1, borderRadius: 1, fontSize: 1,
                marginLeft: 1, marginTop: 1, marginRight: 1, marginBottom: 1, paddingLeft: 1, paddingTop: 1, borderWidth: 1 };
            function __kebab(k) { return String(k).replace(/[A-Z]/g, function(c) { return '-' + c.toLowerCase(); }); }
            function __parseColor(s) {
                if (typeof s !== 'string') return null;
                var m = s.trim().match(/^#([0-9a-f]{3,8})$/i);
                if (m) {
                    var h = m[1];
                    if (h.length === 3 || h.length === 4) h = h.split('').map(function(c) { return c + c; }).join('');
                    return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16), h.length === 8 ? parseInt(h.slice(6, 8), 16) / 255 : 1];
                }
                m = s.trim().match(/^rgba?\(([^)]+)\)$/i);
                if (m) { var p = m[1].split(',').map(function(x) { return parseFloat(x); }); return [p[0], p[1], p[2], p.length > 3 ? p[3] : 1]; }
                return null;
            }
            function __numUnit(v) {
                if (typeof v === 'number') return [v, ''];
                var m = typeof v === 'string' && v.trim().match(/^(-?[\d.]+)([a-z%]*)$/i);
                return m ? [parseFloat(m[1]), m[2]] : null;
            }
            // 两个值按 t 插值
            function __lerpValue(a, b, t) {
                if (Array.isArray(a) || Array.isArray(b)) {
                    var aa = Array.isArray(a) ? a : [a], bb = Array.isArray(b) ? b : [b], out = [];
                    for (var i = 0; i < Math.max(aa.length, bb.length); i++) out.push(__lerpValue(aa[i] === undefined ? bb[i] : aa[i], bb[i] === undefined ? aa[i] : bb[i], t));
                    return out;
                }
                var ca = __parseColor(a), cb = __parseColor(b);
                if (ca && cb) {
                    var c = ca.map(function(x, i) { return x + (cb[i] - x) * t; });
                    return 'rgba(' + Math.round(c[0]) + ',' + Math.round(c[1]) + ',' + Math.round(c[2]) + ',' + (Math.round(c[3] * 1000) / 1000) + ')';
                }
                var na = __numUnit(a), nb = __numUnit(b);
                if (na && nb && (na[1] === nb[1] || !na[1] || !nb[1])) {
                    var n = na[0] + (nb[0] - na[0]) * t, u = nb[1] || na[1];
                    return u ? n + u : n;
                }
                return t < 0.5 ? a : b;
            }
            // ───── 缓动 ─────
            function __bezier(x1, y1, x2, y2) {
                return function(x) {
                    if (x <= 0 || x >= 1) return x <= 0 ? 0 : 1;
                    var lo = 0, hi = 1, t = x;
                    for (var i = 0; i < 24; i++) {
                        t = (lo + hi) / 2;
                        var bx = 3 * (1 - t) * (1 - t) * t * x1 + 3 * (1 - t) * t * t * x2 + t * t * t;
                        if (bx < x) lo = t; else hi = t;
                    }
                    return 3 * (1 - t) * (1 - t) * t * y1 + 3 * (1 - t) * t * t * y2 + t * t * t;
                };
            }
            function __easeFn(name) {
                if (typeof name === 'function') return name;
                var s = String(name || 'linear').trim();
                var m = s.match(/^cubic-bezier\(([^)]+)\)$/);
                if (m) { var p = m[1].split(',').map(parseFloat); return __bezier(p[0], p[1], p[2], p[3]); }
                switch (s) {
                    case 'ease': return __bezier(0.25, 0.1, 0.25, 1);
                    case 'ease-in': return __bezier(0.42, 0, 1, 1);
                    case 'ease-out': return __bezier(0, 0, 0.58, 1);
                    case 'ease-in-out': return __bezier(0.42, 0, 0.58, 1);
                    case 'step-start': return function(t) { return t > 0 ? 1 : 0; };
                    case 'step-end': return function(t) { return t >= 1 ? 1 : 0; };
                    default: return function(t) { return t; };
                }
            }
            globalThis.__easeFn = __easeFn;
            globalThis.__bezierEase = __bezier;
            // 一组样式值（驼峰）→ 样式文本
            function __styleText(props) {
                var css = '', tf = [];
                for (var k in props) {
                    var v = props[k];
                    if (v === undefined || v === null) continue;
                    if (__TRANSFORM_KEYS[k]) {
                        var unit = /^rotate|^skew/.test(k) ? 'deg' : (/^translate/.test(k) ? 'px' : '');
                        var arr = Array.isArray(v) ? v : [v];
                        if (k === 'rotate3d') unit = '';
                        var parts = arr.map(function(x, i) {
                            if (typeof x !== 'number') return String(x);
                            if (k === 'rotate3d') return i === 3 ? x + 'deg' : String(x);
                            return x + unit;
                        });
                        tf.push(k + '(' + parts.join(', ') + ')');
                    } else {
                        var val = typeof v === 'number' && __LEN_KEYS[k] ? v + 'px' : String(v);
                        css += __kebab(k) + ':' + val + ';';
                    }
                }
                if (tf.length) css += 'transform:' + tf.join(' ') + ';';
                return css;
            }
            globalThis.__styleText = __styleText;

            // 实例上的节点样式：__nsAnim[选择器] = {属性: 值}（animate 写）、__nsWorklet[选择器] = 样式文本（worklet 写）
            function __nsFlush(inst) {
                var out = {}, a = inst.__nsAnim || {}, w = inst.__nsWorklet || {};
                for (var s in a) { var t = __styleText(a[s]); if (t) out[s] = t; }
                for (var s2 in w) if (w[s2]) out[s2] = (out[s2] || '') + w[s2];
                var json = JSON.stringify(out);
                if (json === inst.__nsLast) return;
                inst.__nsLast = json;
                if (typeof inst.setData === 'function') inst.setData({ __nodeStyles: out });
            }
            globalThis.__nsFlush = __nsFlush;

            // ───── 共用的帧循环（animate 与 worklet 都挂在这）─────
            var __frameJobs = [], __frameTimer = null, __inFrame = false;
            function __frameLoop() {
                __frameTimer = null;
                __inFrame = true;
                var now = Date.now(), jobs = __frameJobs;
                __frameJobs = [];
                var keep = [];
                for (var i = 0; i < jobs.length; i++) {
                    var alive = false;
                    try { alive = jobs[i](now); } catch (e) { console.error('动画帧出错', e); }
                    if (alive) keep.push(jobs[i]);
                }
                // 这一帧里新挂上的（回调里又起了动画）排在后面
                __frameJobs = keep.concat(__frameJobs);
                __inFrame = false;
                if (__frameJobs.length) __frameTimer = setTimeout(__frameLoop, 16);
            }
            // job(now) 返回 true 继续下一帧
            function __onFrame(job) {
                __frameJobs.push(job);
                if (!__frameTimer && !__inFrame) __frameTimer = setTimeout(__frameLoop, 0);
            }
            globalThis.__onFrame = __onFrame;

            // 关键帧补齐 offset、算出某个进度下的样式
            function __normalizeKeyframes(kfs) {
                var list = (Array.isArray(kfs) ? kfs : [kfs]).map(function(k) { var o = {}; for (var p in k) o[p] = k[p]; return o; });
                if (list.length === 1) list.unshift({});
                var n = list.length;
                list.forEach(function(k, i) { if (typeof k.offset !== 'number') k.offset = n === 1 ? 1 : i / (n - 1); });
                return list;
            }
            function __frameAt(list, p) {
                var props = {}, keys = {};
                list.forEach(function(k) { for (var q in k) if (q !== 'offset' && q !== 'ease' && q !== 'composite') keys[q] = 1; });
                for (var key in keys) {
                    // 这个属性出现过的关键帧
                    var ks = list.filter(function(k) { return k[key] !== undefined; });
                    if (!ks.length) continue;
                    if (p <= ks[0].offset) { props[key] = ks[0][key]; continue; }
                    if (p >= ks[ks.length - 1].offset) { props[key] = ks[ks.length - 1][key]; continue; }
                    for (var i = 0; i < ks.length - 1; i++) {
                        var a = ks[i], b = ks[i + 1];
                        if (p >= a.offset && p <= b.offset) {
                            var span = b.offset - a.offset, t = span > 0 ? (p - a.offset) / span : 1;
                            props[key] = __lerpValue(a[key], b[key], __easeFn(a.ease)(t));
                            break;
                        }
                    }
                }
                return props;
            }

            function __attachAnimate(inst) {
                inst.__nsAnim = {};
                var runs = {};   // 选择器 → 正在跑的动画（同一选择器再 animate 会替换）
                inst.animate = function(selector, keyframes, duration, cbOrTimeline) {
                    var sel = String(selector || '');
                    if (!sel) return;
                    var list = __normalizeKeyframes(keyframes || []);
                    var dur = Math.max(0, Number(duration) || 0);
                    var tl = cbOrTimeline && typeof cbOrTimeline === 'object' ? cbOrTimeline : null;
                    var cb = typeof cbOrTimeline === 'function' ? cbOrTimeline : null;
                    var run = { stopped: false };
                    if (runs[sel]) runs[sel].stopped = true;
                    runs[sel] = run;
                    var apply = function(p) {
                        var cur = inst.__nsAnim[sel] || (inst.__nsAnim[sel] = {});
                        var f = __frameAt(list, p);
                        for (var k in f) cur[k] = f[k];
                        __nsFlush(inst);
                    };
                    if (tl) {
                        // 滚动驱动：进度 = (滚动位置 - 起点) / (终点 - 起点)
                        var start = Number(tl.startScrollOffset) || 0, end = Number(tl.endScrollOffset);
                        var vertical = tl.orientation !== 'horizontal', last = null, polling = false;
                        apply(0);
                        __onFrame(function(now) {
                            if (run.stopped) return false;
                            if (!polling && (!run.at || now - run.at >= 50)) {
                                polling = true; run.at = now;
                                wx.createSelectorQuery().select(String(tl.scrollSource || '')).scrollOffset().exec(function(res) {
                                    polling = false;
                                    var r = res && res[0];
                                    if (!r) return;
                                    var off = vertical ? r.scrollTop : r.scrollLeft;
                                    var e = isNaN(end) ? (vertical ? r.scrollHeight : r.scrollWidth) : end;
                                    var p = e > start ? Math.max(0, Math.min(1, (off - start) / (e - start))) : 0;
                                    if (p !== last) { last = p; apply(p); }
                                });
                            }
                            return true;
                        });
                        return;
                    }
                    var t0 = Date.now();
                    apply(0);
                    __onFrame(function(now) {
                        if (run.stopped) return false;
                        var p = dur > 0 ? Math.min(1, (now - t0) / dur) : 1;
                        apply(p);
                        if (p >= 1) {
                            if (runs[sel] === run) delete runs[sel];
                            if (cb) { try { cb(); } catch (e) { console.error(e); } }
                            return false;
                        }
                        return true;
                    });
                };
                inst.clearAnimation = function(selector, options, callback) {
                    if (typeof options === 'function') { callback = options; options = null; }
                    var sel = String(selector || '');
                    if (runs[sel]) { runs[sel].stopped = true; delete runs[sel]; }
                    var cur = inst.__nsAnim[sel];
                    if (cur) {
                        if (options && typeof options === 'object') {
                            for (var k in options) if (options[k]) delete cur[k];
                            if (!Object.keys(cur).length) delete inst.__nsAnim[sel];
                        } else {
                            delete inst.__nsAnim[sel];
                        }
                        __nsFlush(inst);
                    }
                    if (typeof callback === 'function') setTimeout(callback, 0);
                };
            }
