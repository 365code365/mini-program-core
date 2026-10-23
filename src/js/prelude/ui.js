            // ── 提示框（官方 api/ui/interaction/wx.showToast / showLoading / hideToast / hideLoading）──
            // 微信里 Toast 与 Loading 是**同一个提示框**：show 一个就顶掉另一个；hideToast /
            // hideLoading 默认能互相关掉（文档称「混用」），`noConflict: true` 才只关自己那种
            // （基础库 2.22.1）。宿主侧同一条规则在 host::ui_overlay::apply_feedback_event。
            var __toastTimer = null;
            var __tipKind = null;      // 槽位里是谁：'toast' | 'loading' | null
            var __tipConfig = null;
            var __modalVisible = false;
            var __modalConfig = null;
            var __modalCallback = null;

            function __uiTypeName(v) {
                if (v === null) return 'Null';
                if (v === undefined) return 'Undefined';
                return Object.prototype.toString.call(v).slice(8, -1);
            }
            // 回调与 Promise 两种风格：一个回调都没给时返回 Promise（与新版基础库一致）
            function __uiDone(o, res, ok) {
                if (ok) { o.success && o.success(res); } else { o.fail && o.fail(res); }
                o.complete && o.complete(res);
                if (!o.success && !o.fail && !o.complete) {
                    return ok ? Promise.resolve(res) : Promise.reject(res);
                }
            }
            // title 是必填的 string：基础库按类型校验，`wx.showLoading()` 在微信里就是 fail、什么也不弹
            function __tipTitleError(api, title) {
                if (typeof title === 'string') return null;
                return { errMsg: api + ':fail parameter error: parameter.title should be String instead of ' + __uiTypeName(title) + ';' };
            }
            function __clearToastTimer() {
                if (__toastTimer) { clearTimeout(__toastTimer); __toastTimer = null; }
            }

            wx.showToast = function(options) {
                options = options || {};
                var bad = __tipTitleError('showToast', options.title);
                if (bad) return __uiDone(options, bad, false);
                var title = options.title;
                // 合法值只有这四个；别的值（含不传）按 success 画
                var icon = options.icon;
                if (icon !== 'success' && icon !== 'error' && icon !== 'loading' && icon !== 'none') icon = 'success';
                var duration = options.duration === undefined ? 1500 : Number(options.duration);
                if (!isFinite(duration) || duration < 0) duration = 1500;
                var mask = !!options.mask;
                var image = typeof options.image === 'string' ? options.image : '';

                __clearToastTimer();
                __tipKind = 'toast';
                __tipConfig = { title: title, icon: icon, image: image, mask: mask };
                if (typeof __native_show_toast === 'function') {
                    __native_show_toast(title, icon, String(duration), mask ? 'true' : 'false', image);
                }
                __native_print('[Toast] ' + title + ' (' + icon + ')');
                // 到时只收 Toast 自己：期间被 showLoading 顶掉的话计时器已经清了，不会误关 Loading
                __toastTimer = setTimeout(function() {
                    __toastTimer = null;
                    if (__tipKind !== 'toast') return;
                    __tipKind = null;
                    __tipConfig = null;
                    if (typeof __native_hide_toast === 'function') { __native_hide_toast('true'); }
                }, duration);
                return __uiDone(options, { errMsg: 'showToast:ok' }, true);
            };

            wx.showLoading = function(options) {
                options = options || {};
                var bad = __tipTitleError('showLoading', options.title);
                if (bad) return __uiDone(options, bad, false);
                var mask = !!options.mask;
                __clearToastTimer();
                __tipKind = 'loading';
                __tipConfig = { title: options.title, mask: mask };
                if (typeof __native_show_loading === 'function') {
                    __native_show_loading(options.title, mask ? 'true' : 'false');
                }
                __native_print('[Loading] ' + options.title);
                return __uiDone(options, { errMsg: 'showLoading:ok' }, true);
            };

            // 槽位空着（或 noConflict 下槽位里是另一种）时 fail，errMsg 与微信一致
            function __hideTip(api, kind, options) {
                options = options || {};
                var noConflict = !!options.noConflict;
                if (!__tipKind || (noConflict && __tipKind !== kind)) {
                    return __uiDone(options, { errMsg: api + ":fail:toast can't be found" }, false);
                }
                __clearToastTimer();
                __tipKind = null;
                __tipConfig = null;
                var fn = kind === 'toast' ? __native_hide_toast : __native_hide_loading;
                if (typeof fn === 'function') { fn(noConflict ? 'true' : 'false'); }
                return __uiDone(options, { errMsg: api + ':ok' }, true);
            }
            wx.hideToast = function(options) { return __hideTip('hideToast', 'toast', options); };
            wx.hideLoading = function(options) { return __hideTip('hideLoading', 'loading', options); };

            // ── wx.showModal（官方 api/ui/interaction/wx.showModal）──
            // 按钮文字最多 4 个汉字（按「汉字 2、ASCII 1」计 8）；颜色必须是 16 进制串，
            // 不合法时用缺省（取消 #000000、确定 #576B95）。取舍：`editable` 输入框未实现。
            var __hexColor = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;
            function __modalTextLen(s) {
                var n = 0;
                for (var i = 0; i < s.length; i++) n += s.charCodeAt(i) > 0xFF ? 2 : 1;
                return n;
            }
            function __modalColor(v, def) {
                return typeof v === 'string' && __hexColor.test(v) ? v : def;
            }
            wx.showModal = function(options) {
                options = options || {};
                var cfg = {
                    title: options.title == null ? '' : String(options.title),
                    content: options.content == null ? '' : String(options.content),
                    showCancel: options.showCancel !== false,
                    cancelText: options.cancelText == null ? '取消' : String(options.cancelText),
                    confirmText: options.confirmText == null ? '确定' : String(options.confirmText),
                    cancelColor: __modalColor(options.cancelColor, '#000000'),
                    confirmColor: __modalColor(options.confirmColor, '#576B95')
                };
                if (__modalTextLen(cfg.cancelText) > 8) {
                    return __uiDone(options, { errMsg: 'showModal:fail cancelText length should not larger than 4 Chinese characters' }, false);
                }
                if (__modalTextLen(cfg.confirmText) > 8) {
                    return __uiDone(options, { errMsg: 'showModal:fail confirmText length should not larger than 4 Chinese characters' }, false);
                }
                var task = { o: options }, promise;
                if (!options.success && !options.fail && !options.complete) {
                    promise = new Promise(function(resolve) { task.resolve = resolve; });
                }
                __modalVisible = true;
                __modalConfig = cfg;
                __modalCallback = task;
                if (typeof __native_show_modal === 'function') {
                    __native_show_modal(cfg.title, cfg.content, cfg.showCancel ? 'true' : 'false',
                        cfg.cancelText, cfg.confirmText, cfg.cancelColor, cfg.confirmColor);
                }
                __native_print('[Modal] ' + cfg.title + ': ' + cfg.content);
                return promise;
            };

            // 宿主点了按钮（host::ui_overlay::modal_release）。**先清状态再回调**：
            // 回调里再调 showModal（连弹两个）时，新弹窗登记的回调不能被这里随后清掉 ——
            // 从前是先回调后清，第二个弹窗的 success 永远不会被调用
            function __handleModalResult(confirm) {
                var task = __modalCallback;
                __modalVisible = false;
                __modalConfig = null;
                __modalCallback = null;
                if (!task) return;
                var result = { errMsg: 'showModal:ok', confirm: !!confirm, cancel: !confirm };
                task.o.success && task.o.success(result);
                task.resolve && task.resolve(result);
                task.o.complete && task.o.complete(result);
            }
            
            // wx.showActionSheet 在 system.js（面板由宿主画：host::ui_overlay::action_sheet）
            
            // ── wx.createAnimation：小程序里「用 JS 操作样式」的正式做法 ──
            // 链式调用累积一步的目标值，step() 定格一步，export() 交给
            // `animation="{{animData}}"` 属性，渲染层按 actions 顺序插值。
            wx.createAnimation = function(option) {
                option = option || {};
                var defaults = {
                    duration: option.duration === undefined ? 400 : option.duration,
                    delay: option.delay === undefined ? 0 : option.delay,
                    timingFunction: option.timingFunction || 'linear',
                    transformOrigin: option.transformOrigin || '50% 50% 0'
                };
                var actions = [];
                var pending = { animates: [], option: null };
                function record(type, args) {
                    pending.animates.push({ type: type, args: args });
                    return api;
                }
                var api = {
                    // 变换（H5 端 runtime.js 有一份同语义实现）
                    translate: function(x, y) { return record('translate', [x || 0, y || 0]); },
                    translateX: function(v) { return record('translateX', [v || 0]); },
                    translateY: function(v) { return record('translateY', [v || 0]); },
                    rotate: function(deg) { return record('rotate', [deg || 0]); },
                    rotateZ: function(deg) { return record('rotate', [deg || 0]); },
                    scale: function(sx, sy) { return record('scale', [sx === undefined ? 1 : sx, sy === undefined ? sx : sy]); },
                    scaleX: function(v) { return record('scaleX', [v === undefined ? 1 : v]); },
                    scaleY: function(v) { return record('scaleY', [v === undefined ? 1 : v]); },
                    skew: function(x, y) { return record('skew', [x || 0, y || 0]); },
                    skewX: function(v) { return record('skewX', [v || 0]); },
                    skewY: function(v) { return record('skewY', [v || 0]); },
                    matrix: function(a, b, c, d, tx, ty) { return record('matrix', [a, b, c, d, tx, ty]); },
                    // 3D：渲染层是 2D 仿射，按正交投影降级 —— rotateX/Y 压缩成 scaleY/X(cos θ)，Z 方向的平移 / 缩放不出效果
                    rotateX: function(deg) { return record('rotateX', [deg || 0]); },
                    rotateY: function(deg) { return record('rotateY', [deg || 0]); },
                    rotate3d: function(x, y, z, deg) { return record('rotate3d', [x || 0, y || 0, z || 0, deg || 0]); },
                    translate3d: function(x, y, z) { return record('translate', [x || 0, y || 0]); },
                    translateZ: function() { return api; },
                    scale3d: function(sx, sy) { return record('scale', [sx === undefined ? 1 : sx, sy === undefined ? 1 : sy]); },
                    scaleZ: function() { return api; },
                    matrix3d: function() {
                        var m = [].slice.call(arguments);
                        // 列主序 4×4 取 2D 部分：a=m0 b=m1 c=m4 d=m5 tx=m12 ty=m13
                        return record('matrix', [m[0], m[1], m[4], m[5], m[12] || 0, m[13] || 0]);
                    },
                    // 定位：left / top / right / bottom 是布局属性，渲染层只做绘制期的插值（同 width / height 的取舍：不支持）
                    left: function(v) { return record('left', [v]); },
                    top: function(v) { return record('top', [v]); },
                    right: function(v) { return record('right', [v]); },
                    bottom: function(v) { return record('bottom', [v]); },
                    // 样式
                    opacity: function(v) { return record('opacity', [v]); },
                    backgroundColor: function(c) { return record('backgroundColor', [c]); },
                    width: function(v) { return record('width', [v]); },
                    height: function(v) { return record('height', [v]); },
                    // 定格一步
                    step: function(cfg) {
                        cfg = cfg || {};
                        actions.push({
                            animates: pending.animates,
                            option: {
                                transition: {
                                    duration: cfg.duration === undefined ? defaults.duration : cfg.duration,
                                    delay: cfg.delay === undefined ? defaults.delay : cfg.delay,
                                    timingFunction: cfg.timingFunction || defaults.timingFunction
                                },
                                transformOrigin: cfg.transformOrigin || defaults.transformOrigin
                            }
                        });
                        pending = { animates: [], option: null };
                        return api;
                    },
                    export: function() {
                        var out = { actions: actions };
                        actions = [];
                        return out;
                    }
                };
                return api;
            };

            // 下拉刷新：指示器与内容位移都由宿主负责，逻辑层只发「开始/结束」
            wx.startPullDownRefresh = function(options) {
                options = options || {};
                if (typeof __native_start_pull_down_refresh === 'function') {
                    __native_start_pull_down_refresh();
                }
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.stopPullDownRefresh = function(options) {
                options = options || {};
                if (typeof __native_stop_pull_down_refresh === 'function') {
                    __native_stop_pull_down_refresh();
                }
                options.success && options.success();
                options.complete && options.complete();
            };

            // 页面滚动到目标位置（官方 api/ui/scroll/wx.pageScrollTo）：`scrollTop` 或 `selector`
            // （二选一，selector 优先），`offsetTop` 只配合 selector，`duration` 默认 300ms。
            // 位置换算与动画由宿主做（它有布局树），这里只把请求交过去
            wx.pageScrollTo = function(options) {
                options = options || {};
                var top = Number(options.scrollTop);
                var sel = typeof options.selector === 'string' ? options.selector : '';
                if (!sel && !isFinite(top)) {
                    var bad = { errMsg: 'pageScrollTo:fail scrollTop or selector is required' };
                    options.fail && options.fail(bad);
                    options.complete && options.complete(bad);
                    if (!options.success && !options.fail && !options.complete) { return Promise.reject(bad); }
                    return;
                }
                var dur = options.duration === undefined ? 300 : Number(options.duration);
                __native_page_scroll_to(JSON.stringify({
                    scrollTop: isFinite(top) ? top : null,
                    selector: sel,
                    offsetTop: Number(options.offsetTop) || 0,
                    duration: isFinite(dur) && dur > 0 ? dur : 0
                }));
                var res = { errMsg: 'pageScrollTo:ok' };
                options.success && options.success(res);
                options.complete && options.complete(res);
                if (!options.success && !options.fail && !options.complete) { return Promise.resolve(res); }
            };
            
            function __getUIState() {
                return JSON.stringify({
                    toast: __tipKind === 'toast' ? __tipConfig : null,
                    loading: __tipKind === 'loading' ? __tipConfig : null,
                    modal: __modalVisible ? __modalConfig : null
                });
            }
