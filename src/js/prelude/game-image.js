            // ==================== 小游戏：wx.createImage() 与字体（game-doc/02 §5.3、03） ====================
            //
            // 解码在原生的后台线程（game/image）；结果在下一次 pump（rAF 之前）经 __gameImageDone(id, json) 回来。
            // - 加载期间 JS 这边强引用着对象：常见写法是局部变量 + onload，没人持有也得等它回调完；
            // - 加载完之后只靠游戏自己的引用活着，对象被回收时 FinalizationRegistry 通知原生放掉解码结果；
            // - width / height 是自然尺寸（加载前 0），可写（与 HTMLImageElement 一样），drawImage 3 参数版本用的是原生的自然尺寸；
            // - onerror 收到 { errMsg }（取证前的默认形状，game-doc/06 §7）。
            (function() {
                var loading = {};
                var reg = typeof FinalizationRegistry === 'function' ? new FinalizationRegistry(function(id) { __game_img_free(id); }) : null;
                function GameImage() {
                    var id = __game_img_new(), src = '', nw = 0, nh = 0, done = true;
                    this.__id = id;
                    this.width = 0;
                    this.height = 0;
                    this.onload = null;
                    this.onerror = null;
                    this.referrerPolicy = '';
                    var self = this;
                    Object.defineProperty(this, 'src', {
                        get: function() { return src; },
                        set: function(v) {
                            src = v === undefined || v === null ? '' : String(v);
                            nw = nh = 0; self.width = self.height = 0;
                            done = src === '';
                            if (done) delete loading[id]; else loading[id] = self;
                            __game_img_src(id, src);
                        },
                        enumerable: true
                    });
                    Object.defineProperty(this, 'complete', { get: function() { return done; }, enumerable: true });
                    Object.defineProperty(this, 'naturalWidth', { get: function() { return nw; }, enumerable: true });
                    Object.defineProperty(this, 'naturalHeight', { get: function() { return nh; }, enumerable: true });
                    // addEventListener('load' | 'error')：Cocos Creator 3.x 的图片下载器（Ax）只用这个、不设 onload，
                    // 没有它 `new Image()` 之后第一行就 TypeError，游戏卡在启动。顺序同浏览器：先 onxxx，再按注册顺序的监听
                    var listeners = { load: [], error: [] };
                    this.addEventListener = function(type, fn) {
                        var l = listeners[type];
                        if (l && typeof fn === 'function' && l.indexOf(fn) < 0) l.push(fn);
                    };
                    this.removeEventListener = function(type, fn) {
                        var l = listeners[type], i = l ? l.indexOf(fn) : -1;
                        if (i >= 0) l.splice(i, 1);
                    };
                    function fire(type, ev) {
                        var h = type === 'load' ? self.onload : self.onerror;
                        if (typeof h === 'function') __gameCall(h, self, [ev]);
                        // 拷一份再派：回调里 removeEventListener 自己（Cocos 就这么写）不能跳过下一个
                        listeners[type].slice().forEach(function(fn) { __gameCall(fn, self, [ev]); });
                    }
                    this.__done = function(r) {
                        done = true;
                        if (r.ok) {
                            nw = r.w; nh = r.h; self.width = r.w; self.height = r.h;
                            fire('load', { type: 'load', target: self });
                        } else {
                            fire('error', { type: 'error', target: self, errMsg: r.errMsg });
                        }
                    };
                    if (reg) reg.register(this, id);
                }
                globalThis.__GameImage = GameImage;
                globalThis.__gameImageDone = function(id, json) {
                    var img = loading[id];
                    if (!img) return 0;
                    delete loading[id];
                    img.__done(JSON.parse(json));
                    return 1;
                };
                wx.createImage = function() { return new GameImage(); };
                // 字体（game/fonts.rs）：loadFont 同步返回字族名，失败 null；getTextLineHeight 返回行高 number
                wx.loadFont = function(path) { return __game_load_font(String(path === undefined ? '' : path)); };
                wx.getTextLineHeight = function(o) {
                    o = o || {};
                    return __game_text_line_height(JSON.stringify({ fontSize: Number(o.fontSize) || 16, fontFamily: o.fontFamily ? String(o.fontFamily) : 'sans-serif',
                        fontWeight: o.fontWeight, fontStyle: o.fontStyle, text: o.text === undefined ? '' : String(o.text) }));
                };
            })();
