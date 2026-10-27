            // ==================== 小游戏：原生按钮（game-doc/03、04 G7.3，GT-U3；原生在 game/buttons.rs） ====================
            // createUserInfoButton / createOpenSettingButton / createFeedbackButton / createGameClubButton / createGameRecorderShareButton。
            // 引擎画、引擎命中；点了之后的动作交给宿主：
            // - UserInfoButton：宿主的 getUserInfo（hostApis 接了才有数据，没接 → onTap 收到 getUserInfo:fail …，与微信无授权同形）；
            // - OpenSettingButton：宿主的 openSetting（onTap 收到 {authSetting} 或 fail）；
            // - FeedbackButton / GameClubButton / GameRecorderShareButton：宿主事件 {type:'openFeedback' | 'openGameClub' | 'shareGameRecorder'}，onTap({})。
            // button.style 的字段可以直接改（按钮跟着变），text / image 同样。type: 'image' 的 image 按包内路径加载（与 wx.createImage 同一个解码与缓存）。
            (function() {
                var FIELDS = ['left', 'top', 'width', 'height', 'backgroundColor', 'borderColor', 'borderWidth', 'borderRadius', 'color',
                    'textAlign', 'fontSize', 'lineHeight'];
                var seq = 0, all = {};
                function make(kind, o) {
                    o = o || {};
                    var id = ++seq, taps = [], visible = true, img = null;
                    var st = {}, raw = {};
                    var src = o.style || {};
                    FIELDS.forEach(function(k) {
                        raw[k] = src[k];
                        Object.defineProperty(st, k, { enumerable: true, get: function() { return raw[k]; }, set: function(v) { raw[k] = v; sync(); } });
                    });
                    var b = { type: o.type === 'image' ? 'image' : 'text', style: st };
                    var text = o.text == null ? '' : String(o.text), image = o.image == null ? '' : String(o.image);
                    if (kind === 'gameClub') b.icon = o.icon || 'green';
                    function loadImage() {
                        img = null;
                        if (b.type !== 'image' || !image) return;
                        img = wx.createImage();
                        img.onload = sync;
                        img.src = image;
                    }
                    function state() {
                        var s = {};
                        FIELDS.forEach(function(k) { if (raw[k] !== undefined && raw[k] !== null) s[k] = typeof raw[k] === 'number' ? raw[k] : (/^(left|top|width|height|borderWidth|borderRadius|fontSize|lineHeight)$/.test(k) ? Number(raw[k]) : String(raw[k])); });
                        return { id: id, kind: kind, type: b.type, text: text, image: img && img.__id, icon: b.icon, style: s, visible: visible };
                    }
                    function sync() { if (all[id]) __game_button('update', JSON.stringify(state())); }
                    Object.defineProperty(b, 'text', { enumerable: true, get: function() { return text; }, set: function(v) { text = v == null ? '' : String(v); sync(); } });
                    Object.defineProperty(b, 'image', { enumerable: true, get: function() { return image; }, set: function(v) { image = v == null ? '' : String(v); loadImage(); sync(); } });
                    b.show = function() { visible = true; sync(); };
                    b.hide = function() { visible = false; sync(); };
                    b.destroy = function() { if (all[id]) { delete all[id]; __game_button('destroy', JSON.stringify({ id: id })); } };
                    b.onTap = function(cb) { if (typeof cb === 'function' && taps.indexOf(cb) < 0) taps.push(cb); };
                    b.offTap = function(cb) {
                        var keep = typeof cb === 'function' ? taps.filter(function(f) { return f !== cb; }) : [];
                        taps.length = 0; Array.prototype.push.apply(taps, keep);
                    };
                    b.__fire = function(res) { taps.slice().forEach(function(cb) { __gameCall(cb, null, [res]); }); };
                    b.__opts = o;
                    b.__kind = kind;
                    all[id] = b;
                    loadImage();
                    __game_button('create', JSON.stringify(state()));
                    return b;
                }
                function hostTap(api, args, b) {
                    var a = args || {};
                    a.success = function(r) { b.__fire(r); };
                    a.fail = function(r) { b.__fire(r); };
                    __hostInvoke(api, a);
                }
                // 原生命中之后调（按下与抬起在同一颗按钮上）
                globalThis.__gameButtonTap = function(id) {
                    var b = all[id];
                    if (!b) return 0;
                    var o = b.__opts;
                    if (b.__kind === 'userInfo') hostTap('getUserInfo', { withCredentials: !!o.withCredentials, lang: o.lang || 'en' }, b);
                    else if (b.__kind === 'openSetting') hostTap('openSetting', { withSubscriptions: !!o.withSubscriptions }, b);
                    else {
                        var ev = { feedback: 'openFeedback', gameClub: 'openGameClub', recorderShare: 'shareGameRecorder' }[b.__kind];
                        __native_host_event(JSON.stringify({ type: ev, share: o.share || null }));
                        b.__fire({});
                    }
                    return 1;
                };
                wx.createUserInfoButton = function(o) { return make('userInfo', o); };
                wx.createOpenSettingButton = function(o) { return make('openSetting', o); };
                wx.createFeedbackButton = function(o) { return make('feedback', o); };
                wx.createGameClubButton = function(o) { return make('gameClub', o); };
                wx.createGameRecorderShareButton = function(o) { return make('recorderShare', o); };
            })();
