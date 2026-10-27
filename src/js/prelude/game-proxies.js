            // ==================== 小游戏：宿主承载的对象（game-doc/03 §8、04 G7.4 / G7.6，GT-H1~H3） ====================
            // 广告（Banner / Grid / Custom / RewardedVideo / Interstitial / AdSkipCard）、相机、PageManager、StoreGift、VideoDecoder、
            // GameRecorder、GameServerManager、RankManager 这些对象的能力只有宿主（或微信后台）有。规则一条：
            // - 每个方法是 `__hostInvoke('<类>.<方法>', {contextId, …})`：宿主在 hostApis 里点名就交给它（hostCall 事件），
            //   没点名就 fail，返回 Promise 的方法 reject（errMsg 与微信「不支持 / 无广告」同形）；
            // - 对象的事件（onLoad / onError / onClose / onResize …）由宿主推：mr_game_dispatch_event("contextEvent",
            //   {context:'BannerAd', contextId:'ad1', event:'load', data:{…}})；
            // - 广告建出来就「拉一次」（微信语义）：宿主没接 `<类>.load` 时下一轮 onError({errMsg:'…:fail no advertisement', errCode:1004})；
            // - 引擎自己能做的（GameLogManager、ScenePerformanceManager、LogManager 的本地部分）就地实现，同时交宿主日志。
            // setMenuStyle / setStatusBarStyle 交宿主（宿主事件），立即 success。
            (function() {
                var ls = {};
                function on(cls, id, ev, cb) { if (typeof cb === 'function') (ls[cls + '#' + id + '#' + ev] = ls[cls + '#' + id + '#' + ev] || []).push(cb); }
                function off(cls, id, ev, cb) {
                    var k = cls + '#' + id + '#' + ev;
                    if (ls[k]) ls[k] = typeof cb === 'function' ? ls[k].filter(function(f) { return f !== cb; }) : [];
                }
                function fire(cls, id, ev, data) { (ls[cls + '#' + id + '#' + ev] || []).slice().forEach(function(cb) { __gameCall(cb, null, [data]); }); }
                // host-capabilities.js 的 contextEvent 通道按名字调它（小程序那份在 host-objects.js，小游戏不装那段）
                if (typeof __dispatchContextEvent !== 'function') {
                    globalThis.__dispatchContextEvent = function(p) {
                        p = p || {};
                        var k = p.context + '#' + (p.contextId || '') + '#' + p.event;
                        var n = (ls[k] || []).length;
                        fire(p.context, p.contextId || '', p.event, p.data || {});
                        return n;
                    };
                }
                var seq = 0;
                function hosted(api) { return typeof __hostApiSet === 'object' && !!__hostApiSet[api]; }
                // 一个宿主对象：methods 交宿主，events 是 on / off 成对的事件名（'Load' → onLoad / offLoad，事件键 'load'）
                function obj(cls, opts, methods, events) {
                    var id = cls.charAt(0).toLowerCase() + cls.slice(1) + (++seq);
                    var o = { __id: id };
                    methods.forEach(function(m) {
                        o[m] = function(a) {
                            var args = {};
                            for (var k in (a || {})) args[k] = a[k];
                            args.contextId = id;
                            return __hostInvoke(cls + '.' + m, args);
                        };
                    });
                    (events || []).forEach(function(E) {
                        var e = E.charAt(0).toLowerCase() + E.slice(1);
                        o['on' + E] = function(cb) { on(cls, id, e, cb); };
                        o['off' + E] = function(cb) { off(cls, id, e, cb); };
                    });
                    // 通用 on(event, cb) / off(event, cb)（GameRecorder / PageManager / VideoDecoder 的写法）
                    if (methods.indexOf('on') >= 0) {
                        o.on = function(ev, cb) { on(cls, id, ev, cb); };
                        o.off = function(ev, cb) { off(cls, id, ev, cb); };
                    }
                    if (hosted(cls + '.create')) __hostInvoke(cls + '.create', { contextId: id, options: opts || {} });
                    return o;
                }
                function ad(cls, opts, methods, events) {
                    var o = obj(cls, opts, methods, events);
                    var bad = { errMsg: cls + ':fail no advertisement', errCode: 1004 };
                    // 宿主没接：show / load 返回 reject 的 Promise，建出来下一轮 onError
                    ['show', 'load'].forEach(function(m) {
                        if (methods.indexOf(m) < 0 || hosted(cls + '.' + m)) return;
                        o[m] = function() {
                            if (m === 'load') setTimeout(function() { fire(cls, o.__id, 'error', bad); }, 0);
                            return Promise.reject(bad);
                        };
                    });
                    if (!hosted(cls + '.load') && !hosted(cls + '.create')) setTimeout(function() { fire(cls, o.__id, 'error', bad); }, 0);
                    if (opts && opts.style) {
                        var st = {}, raw = opts.style;
                        ['left', 'top', 'width', 'height', 'realWidth', 'realHeight'].forEach(function(k) {
                            Object.defineProperty(st, k, { enumerable: true, get: function() { return raw[k]; },
                                set: function(v) { raw[k] = v; if (hosted(cls + '.setStyle')) __hostInvoke(cls + '.setStyle', { contextId: o.__id, style: raw }); } });
                        });
                        o.style = st;
                    }
                    if (methods.indexOf('isShow') >= 0 && !hosted(cls + '.isShow')) o.isShow = function() { return false; };
                    return o;
                }
                wx.createBannerAd = function(o) { return ad('BannerAd', o, ['show', 'hide', 'destroy'], ['Load', 'Error', 'Resize']); };
                wx.createGridAd = function(o) { return ad('GridAd', o, ['show', 'hide', 'destroy'], ['Load', 'Error', 'Resize']); };
                wx.createCustomAd = function(o) { return ad('CustomAd', o, ['show', 'hide', 'destroy', 'isShow'], ['Load', 'Error', 'Resize', 'Close', 'Hide']); };
                wx.createRewardedVideoAd = function(o) {
                    return ad('RewardedVideoAd', o, ['show', 'load', 'destroy', 'setMiniGameAutoIntelligenceInfo', 'setServerSideVerificationData'], ['Load', 'Error', 'Close']);
                };
                wx.createInterstitialAd = function(o) { return ad('InterstitialAd', o, ['show', 'load', 'destroy'], ['Load', 'Error', 'Close']); };
                wx.createAdSkipCard = function(o) { return ad('AdSkipCard', o, ['show', 'hide', 'destroy'], ['Load', 'Error', 'Close']); };
                wx.createCamera = function(o) {
                    return obj('Camera', o, ['takePhoto', 'startRecord', 'stopRecord', 'setZoom', 'listenFrameChange', 'closeFrameChange', 'destroy'],
                        ['CameraFrame', 'AuthCancel', 'Stop']);
                };
                wx.createPageManager = function(o) { return obj('PageManager', o, ['load', 'show', 'destroy', 'on'], []); };
                wx.createStoreGift = function(o) {
                    var s = obj('StoreGift', o, ['open', 'getOrderInfo'], []);
                    s.isSupported = function() { return hosted('StoreGift.open'); };
                    return s;
                };
                wx.createVideoDecoder = function() { return obj('VideoDecoder', null, ['start', 'stop', 'seek', 'remove', 'getFrameData', 'on'], []); };
                var recorder = null;
                wx.getGameRecorder = function() {
                    if (recorder) return recorder;
                    recorder = obj('GameRecorder', null, ['start', 'stop', 'pause', 'resume', 'abort', 'on'], []);
                    ['isAtempoSupported', 'isFrameSupported', 'isSoundSupported', 'isVolumeSupported'].forEach(function(m) {
                        recorder[m] = function() { return hosted('GameRecorder.start'); };
                    });
                    return recorder;
                };
                var server = null;
                wx.getGameServerManager = function() {
                    if (server) return server;
                    var ev = ['BeKickedOut', 'Broadcast', 'Disconnect', 'GameEnd', 'GameStart', 'Invite', 'LockStepError', 'Logout', 'Match',
                        'RoomInfoChange', 'StateUpdate', 'SyncFrame'];
                    server = obj('GameServerManager', null, ['broadcastInRoom', 'cancelMatch', 'changeSeat', 'createRoom', 'endGame', 'endStateService',
                        'getFriendsStateData', 'getJoinVoIPChatSignature', 'getLastRoomInfo', 'getLostFrames', 'inviteFriend', 'joinRoom', 'kickoutMember',
                        'login', 'logout', 'memberLeaveRoom', 'ownerLeaveRoom', 'reconnect', 'restart', 'setInviteData', 'setState', 'startGame',
                        'startMatch', 'startStateService', 'updateReadyStatus', 'uploadFrame'], ev);
                    return server;
                };
                var rank = null;
                wx.getRankManager = function() {
                    if (rank) return rank;
                    rank = obj('RankManager', null, ['abort', 'createChallenge', 'getScore', 'middleUpdate', 'update'], ['ChallengeStart']);
                    return rank;
                };
                wx.getMiniReportManager = function() { return obj('MiniReportManager', null, ['report'], []); };
                // 引擎就地做的两个：通用信息存在本地，log 同时进宿主日志
                var logInfo = {};
                wx.getGameLogManager = function() {
                    return {
                        log: function(o) { __native_host_event(JSON.stringify({ type: 'report', api: 'GameLogManager.log', data: o || {} })); },
                        tag: function(t) { return { log: function(o) { __native_host_event(JSON.stringify({ type: 'report', api: 'GameLogManager.log', tag: t, data: o || {} })); } }; },
                        getCommonInfo: function() { return JSON.parse(JSON.stringify(logInfo)); },
                        updateCommonInfo: function(o) { for (var k in (o || {})) logInfo[k] = o[k]; }
                    };
                };
                var perfInfo = {};
                wx.getScenePerformanceManager = function() {
                    return {
                        getCommonInfo: function() { return JSON.parse(JSON.stringify(perfInfo)); },
                        setCommonInfo: function(o) { for (var k in (o || {})) perfInfo[k] = o[k]; },
                        setData: function(o) { __native_host_event(JSON.stringify({ type: 'report', api: 'ScenePerformanceManager.setData', data: o || {} })); }
                    };
                };
                // ── wx.createVideo（G7.7）：原生视频由宿主画（Android VideoView，GameVideo.kt），引擎给完整的对象与事件 ──
                // 宿主事件 {type:'video', op:'create' | 'update' | 'play' | 'pause' | 'stop' | 'seek' | 'requestFullScreen' |
                // 'exitFullScreen' | 'destroy', id, props?, time?, direction?}；属性（x / y / width / height / src …）直接改就发 update。
                // 宿主推 contextEvent {context:'Video', contextId, event:'play' | 'pause' | 'ended' | 'timeUpdate' | 'error' | 'waiting' | 'progress'}。
                // underGameView：视频在游戏画面下面、靠游戏画布透明的地方透出来 —— 引擎上屏是不透明的，这里交给宿主自己决定（Android 示例盖在上面，记在验收文档）
                var VPROPS = ['x', 'y', 'width', 'height', 'src', 'poster', 'initialTime', 'playbackRate', 'live', 'objectFit', 'controls',
                    'showProgress', 'showProgressInControlMode', 'backgroundColor', 'autoplay', 'loop', 'muted', 'obeyMuteSwitch',
                    'enableProgressGesture', 'enablePlayGesture', 'showCenterPlayBtn', 'underGameView'];
                var VDEF = { x: 0, y: 0, width: 300, height: 150, src: '', poster: '', initialTime: 0, playbackRate: 1, live: false,
                    objectFit: 'contain', controls: true, showProgress: true, showProgressInControlMode: true, backgroundColor: '#000000',
                    autoplay: false, loop: false, muted: false, obeyMuteSwitch: false, enableProgressGesture: true, enablePlayGesture: false,
                    showCenterPlayBtn: true, underGameView: false };
                wx.createVideo = function(o) {
                    o = o || {};
                    var id = 'video' + (++seq), props = {}, v = { __id: id }, alive = true;
                    VPROPS.forEach(function(k) {
                        props[k] = o[k] === undefined ? VDEF[k] : o[k];
                        Object.defineProperty(v, k, { enumerable: true, get: function() { return props[k]; },
                            set: function(val) { props[k] = val; if (alive) send('update', { props: props }); } });
                    });
                    function send(op, extra) {
                        var e = { type: 'video', op: op, id: id };
                        for (var k in (extra || {})) e[k] = extra[k];
                        __native_host_event(JSON.stringify(e));
                    }
                    ['play', 'pause', 'stop', 'exitFullScreen'].forEach(function(m) {
                        v[m] = function() { if (alive) send(m); return Promise.resolve(); };
                    });
                    v.seek = function(t) { if (alive) send('seek', { time: Number(t) || 0 }); return Promise.resolve(); };
                    v.requestFullScreen = function(direction) { if (alive) send('requestFullScreen', { direction: direction === undefined ? 0 : Number(direction) }); return Promise.resolve(); };
                    v.destroy = function() { if (alive) { alive = false; send('destroy'); } };
                    ['Play', 'Pause', 'Ended', 'TimeUpdate', 'Error', 'Waiting', 'Progress'].forEach(function(E) {
                        var e = E.charAt(0).toLowerCase() + E.slice(1);
                        v['on' + E] = function(cb) { on('Video', id, e, cb); };
                        v['off' + E] = function(cb) { off('Video', id, e, cb); };
                    });
                    send('create', { props: props });
                    return v;
                };
                wx.setMenuStyle = function(o) {
                    o = o || {};
                    __native_host_event(JSON.stringify({ type: 'menuStyle', style: o.style === 'dark' ? 'dark' : 'light' }));
                    return __asyncOk(o, 'setMenuStyle');
                };
                wx.setStatusBarStyle = function(o) {
                    o = o || {};
                    __native_host_event(JSON.stringify({ type: 'statusBar', style: o.style === 'white' ? 'white' : 'black' }));
                    return __asyncOk(o, 'setStatusBarStyle');
                };
            })();
