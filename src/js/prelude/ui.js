            var __toastTimer = null;
            var __toastVisible = false;
            var __toastConfig = null;
            var __loadingVisible = false;
            var __loadingConfig = null;
            var __modalVisible = false;
            var __modalConfig = null;
            var __modalCallback = null;
            
            wx.showToast = function(options) {
                options = options || {};
                var title = options.title || '';
                var icon = options.icon || 'success';
                var duration = options.duration || 1500;
                var mask = options.mask || false;
                
                if (__toastTimer) { clearTimeout(__toastTimer); __toastTimer = null; }
                __toastVisible = true;
                __toastConfig = { title: title, icon: icon, mask: mask };
                
                if (typeof __native_show_toast === 'function') {
                    __native_show_toast(title, icon, String(duration), mask ? 'true' : 'false');
                }
                __native_print('[Toast] ' + title + ' (' + icon + ')');
                
                __toastTimer = setTimeout(function() { wx.hideToast(); }, duration);
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.hideToast = function(options) {
                options = options || {};
                if (__toastTimer) { clearTimeout(__toastTimer); __toastTimer = null; }
                __toastVisible = false;
                __toastConfig = null;
                if (typeof __native_hide_toast === 'function') { __native_hide_toast(); }
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.showLoading = function(options) {
                options = options || {};
                __loadingVisible = true;
                __loadingConfig = { title: options.title || '', mask: options.mask || false };
                if (typeof __native_show_loading === 'function') {
                    __native_show_loading(options.title || '', options.mask ? 'true' : 'false');
                }
                __native_print('[Loading] ' + (options.title || ''));
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.hideLoading = function(options) {
                options = options || {};
                __loadingVisible = false;
                __loadingConfig = null;
                if (typeof __native_hide_loading === 'function') { __native_hide_loading(); }
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.showModal = function(options) {
                options = options || {};
                __modalVisible = true;
                __modalConfig = {
                    title: options.title || '',
                    content: options.content || '',
                    showCancel: options.showCancel !== false,
                    cancelText: options.cancelText || '取消',
                    confirmText: options.confirmText || '确定'
                };
                __modalCallback = options;
                
                if (typeof __native_show_modal === 'function') {
                    __native_show_modal(__modalConfig.title, __modalConfig.content, 
                        __modalConfig.showCancel ? 'true' : 'false', __modalConfig.cancelText, __modalConfig.confirmText);
                }
                __native_print('[Modal] ' + __modalConfig.title + ': ' + __modalConfig.content);
            };
            
            function __handleModalResult(confirm) {
                if (__modalCallback) {
                    var result = { confirm: confirm, cancel: !confirm };
                    __modalCallback.success && __modalCallback.success(result);
                    __modalCallback.complete && __modalCallback.complete(result);
                }
                __modalVisible = false;
                __modalConfig = null;
                __modalCallback = null;
            }
            
            wx.showActionSheet = function(options) {
                options = options || {};
                __native_print('[ActionSheet] ' + (options.itemList || []).join(', '));
                options.success && options.success({ tapIndex: 0 });
                options.complete && options.complete();
            };
            
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
            
            function __getUIState() {
                return JSON.stringify({
                    toast: __toastVisible ? __toastConfig : null,
                    loading: __loadingVisible ? __loadingConfig : null,
                    modal: __modalVisible ? __modalConfig : null
                });
            }
