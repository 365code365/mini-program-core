            // ───────── 由宿主承载的对象：地图 / 相机 / 直播 / NFC / 画面录制 / 视频解码 / 音视频合成 / 跨境支付 ─────────
            //
            // 这些对象的方法都要原生能力，每个方法是一次 `__hostInvoke('<类>.<方法>', {contextId, …})`：
            // 宿主在 hostApis 里点名这个带点的名字（如 `MapContext.getCenterLocation`）就交给宿主，没点名立刻 fail。
            // contextId 是组件的 id（与 `nativeViews` 事件里的 key 一致），宿主据此找到对应的原生控件。
            // 对象自己的事件（地图点聚合、相机帧、NFC 发现标签……）由宿主推：
            //   mr_app_dispatch_event("contextEvent", {"context":"MapContext","contextId":"map1","event":"markerClusterClick","data":{…}})
            var __ctxListeners = {};
            function __ctxOn(cls, id, ev, cb) {
                if (typeof cb !== 'function') return;
                var k = cls + '#' + id + '#' + ev;
                (__ctxListeners[k] = __ctxListeners[k] || []).push(cb);
            }
            function __ctxOff(cls, id, ev, cb) {
                var k = cls + '#' + id + '#' + ev;
                if (__ctxListeners[k]) __ctxListeners[k] = cb ? __ctxListeners[k].filter(function(f) { return f !== cb; }) : [];
            }
            globalThis.__dispatchContextEvent = function(p) {
                p = p || {};
                var list = __ctxListeners[p.context + '#' + (p.contextId || '') + '#' + p.event] || [];
                var data = p.data || {};
                if (p.event === 'cameraFrame' && typeof data.data === 'string') data.data = __b64decode(data.data);
                list.slice().forEach(function(cb) { try { cb(data); } catch (e) { console.error(e); } });
                return list.length;
            };
            function __hostObject(cls, id, methods) {
                var o = {};
                methods.forEach(function(m) {
                    o[m] = function(opt) {
                        var a = {};
                        for (var k in (opt || {})) a[k] = opt[k];
                        a.contextId = id;
                        return __hostInvoke(cls + '.' + m, a);
                    };
                });
                return o;
            }
            function __ctxId(id, cmp) {
                // 组件里的节点：官方第二个参数是组件实例（this），id 只在组件内唯一 —— 拼上实例键让宿主分得清
                return String(id || '') + (cmp && cmp.__key ? '@' + cmp.__key : '');
            }

            wx.createMapContext = function(mapId, cmp) {
                var id = __ctxId(mapId, cmp);
                var o = __hostObject('MapContext', id, ['addArc', 'addCustomLayer', 'addGroundOverlay', 'addMarkers', 'addVisualLayer', 'eraseLines',
                    'executeVisualLayerCommand', 'fromScreenLocation', 'getCenterLocation', 'getRegion', 'getRotate', 'getScale', 'getSkew',
                    'includePoints', 'initMarkerCluster', 'moveAlong', 'moveToLocation', 'openMapApp', 'removeArc', 'removeCustomLayer',
                    'removeGroundOverlay', 'removeMarkers', 'removeVisualLayer', 'setBoundary', 'setCenterOffset', 'setLocMarkerIcon',
                    'toScreenLocation', 'translateMarker', 'updateGroundOverlay']);
                o.on = function(ev, cb) { __ctxOn('MapContext', id, ev, cb); };
                o.off = function(ev, cb) { __ctxOff('MapContext', id, ev, cb); };
                return o;
            };

            wx.createCameraContext = function() {
                // 一个页面只有一个 camera（官方限制），不带 id
                var id = '';
                var o = __hostObject('CameraContext', id, ['setZoom', 'startRecord', 'stopRecord', 'takePhoto']);
                o.onCameraFrame = function(cb) {
                    __ctxOn('CameraContext', id, 'cameraFrame', cb);
                    return {
                        start: function(opt) { return __hostInvoke('CameraFrameListener.start', opt); },
                        stop: function(opt) { return __hostInvoke('CameraFrameListener.stop', opt); }
                    };
                };
                return o;
            };

            // LivePlayerContext：宿主开了同层渲染的直播（`__sameLayerLive`，创建选项 sameLayer 含 live-player）时，
            // 播放类命令交给引擎（按 id 找到节点、转成 videoCommand 给宿主的播放器，host/engine_live_player.rs），
            // 结果经 __liveReply 回来；其余方法（小窗、投屏、后台播放）与宿主盖控件的情形照旧走 hostCall
            var __liveSeq = 0, __liveCbs = {};
            globalThis.__liveReply = function(seq, ok, res) {
                var c = __liveCbs[seq];
                if (!c) return false;
                delete __liveCbs[seq];
                var o = c.opt;
                try {
                    if (ok) { if (typeof o.success === 'function') o.success(res); }
                    else if (typeof o.fail === 'function') o.fail(res);
                    if (typeof o.complete === 'function') o.complete(res);
                } catch (e) { console.error(e); }
                if (c.resolve) (ok ? c.resolve : c.reject)(res);
                return true;
            };
            function __liveCmd(id, type, opt) {
                opt = opt || {};
                var seq = ++__liveSeq;
                var req = { id: id, type: type, seq: seq };
                ['direction', 'quality', 'sourceType'].forEach(function(k) { if (opt[k] !== undefined) req[k] = opt[k]; });
                var cb = typeof opt.success === 'function' || typeof opt.fail === 'function' || typeof opt.complete === 'function';
                var entry = { opt: opt };
                var p = cb ? undefined : new Promise(function(resolve, reject) { entry.resolve = resolve; entry.reject = reject; });
                __liveCbs[seq] = entry;
                __native_host_request('livePlayer', JSON.stringify(req));
                return p;
            }
            wx.createLivePlayerContext = function(playerId, cmp) {
                var o = __hostObject('LivePlayerContext', __ctxId(playerId, cmp), ['exitBackgroundPlayback', 'exitCasting', 'exitFullScreen',
                    'exitPictureInPicture', 'mute', 'pause', 'play', 'reconnectCasting', 'requestBackgroundPlayback', 'requestFullScreen',
                    'resume', 'snapshot', 'startCasting', 'stop', 'switchCasting']);
                if (globalThis.__sameLayerLive && typeof __native_host_request === 'function') {
                    ['play', 'stop', 'pause', 'resume', 'mute', 'requestFullScreen', 'exitFullScreen', 'snapshot'].forEach(function(m) {
                        o[m] = function(opt) { return __liveCmd(String(playerId || ''), m, opt); };
                    });
                }
                return o;
            };

            wx.createLivePusherContext = function(cmp) {
                var id = __ctxId('', cmp);
                var o = __hostObject('LivePusherContext', id, ['applyBlusherStickMakeup', 'applyEyeBrowMakeup', 'applyEyeShadowMakeup',
                    'applyFaceContourMakeup', 'applyFilter', 'applyLipStickMakeup', 'applySticker', 'clearFilters', 'clearMakeups', 'clearStickers',
                    'exitPictureInPicture', 'getMaxZoom', 'pause', 'pauseBGM', 'playBGM', 'resume', 'resumeBGM', 'sendMessage', 'setBGMVolume',
                    'setMICVolume', 'setZoom', 'snapshot', 'start', 'startPreview', 'stop', 'stopBGM', 'stopPreview', 'switchCamera', 'toggleTorch']);
                o.onCustomRendererEvent = function(ev, cb) { __ctxOn('LivePusherContext', id, ev, cb); };
                o.createOffscreenCanvas = function() { return wx.createOffscreenCanvas({ type: '2d', width: 720, height: 1280 }); };
                return o;
            };

            // ---------- NFC ----------
            var __nfcAdapter = null;
            wx.getNFCAdapter = function() {
                if (__nfcAdapter) return __nfcAdapter;
                var common = ['close', 'connect', 'getMaxTransceiveLength', 'isConnected', 'setTimeout', 'transceive'];
                var techs = {
                    IsoDep: common.concat(['getHistoricalBytes']), MifareClassic: common, MifareUltralight: common,
                    NfcA: common.concat(['getAtqa', 'getSak']), NfcB: common, NfcF: common, NfcV: common,
                    Ndef: ['close', 'connect', 'isConnected', 'setTimeout', 'writeNdefMessage']
                };
                var a = __hostObject('NFCAdapter', '', ['startDiscovery', 'stopDiscovery']);
                a.tech = { isoDep: 'ISO-DEP', mifareClassic: 'MIFARE-Classic', mifareUltralight: 'MIFARE-Ultralight', ndef: 'NDEF',
                           nfcA: 'NFC-A', nfcB: 'NFC-B', nfcF: 'NFC-F', nfcV: 'NFC-V' };
                a.onDiscovered = function(cb) { __ctxOn('NFCAdapter', '', 'discovered', cb); };
                a.offDiscovered = function(cb) { __ctxOff('NFCAdapter', '', 'discovered', cb); };
                Object.keys(techs).forEach(function(cls) {
                    a['get' + cls] = function() {
                        var t = __hostObject(cls, '', techs[cls]);
                        if (cls === 'Ndef') {
                            t.onNdefMessage = function(cb) { __ctxOn('Ndef', '', 'ndefMessage', cb); };
                            t.offNdefMessage = function(cb) { __ctxOff('Ndef', '', 'ndefMessage', cb); };
                        }
                        return t;
                    };
                });
                __nfcAdapter = a;
                return a;
            };

            // ---------- 画面录制 / 视频解码 / 音视频合成 / 跨境支付 ----------
            var __mediaSeq = 0;
            function __eventedHostObject(cls, methods) {
                var id = cls + '_' + (++__mediaSeq);
                var o = __hostObject(cls, id, methods);
                o.on = function(ev, cb) { __ctxOn(cls, id, ev, cb); };
                o.off = function(ev, cb) { __ctxOff(cls, id, ev, cb); };
                return o;
            }
            wx.createMediaRecorder = function(canvas, opt) {
                return __eventedHostObject('MediaRecorder', ['destroy', 'pause', 'requestFrame', 'resume', 'start', 'stop']);
            };
            wx.createVideoDecoder = function() {
                var d = __eventedHostObject('VideoDecoder', ['remove', 'seek', 'start', 'stop']);
                // 同步取帧：宿主经 contextEvent 推 `frame` 时缓存最新一帧
                var last = null;
                d.on('frame', function(f) { last = f; });
                d.getFrameData = function() { var f = last; last = null; return f; };
                return d;
            };
            wx.createMediaContainer = function() {
                return __eventedHostObject('MediaContainer', ['addTrack', 'destroy', 'export', 'extractDataSource', 'removeTrack']);
            };
            wx.createGlobalPayment = function(opt) {
                return __eventedHostObject('GlobalPayment', ['abort', 'openMethodPicker', 'requestGlobalPayment']);
            };
