            // ───────── 音频（官方 api/media/audio、background-audio、recorder）─────────
            // InnerAudioContext 的解码、播放时钟与出声在原生侧（src/audio_ctx），属性写入同步过桥，
            // currentTime / duration / paused / buffered 每次读都问原生要（按时钟算，不是缓存值）。
            // 事件由宿主每帧取回后调 __resolveAudio(id, type, json)。
            var __audioSeq = 0, __audios = {};
            var __AUDIO_EVENTS = ['Canplay', 'Play', 'Pause', 'Stop', 'Ended', 'TimeUpdate', 'Error', 'Waiting', 'Seeking', 'Seeked'];
            function __audioState(id) {
                try { return JSON.parse(__native_audio('state', String(id), '{}')); } catch (e) { return {}; }
            }
            function __makeAudio(extraEvents) {
                var id = ++__audioSeq, ls = {}, props = { src: '', startTime: 0, autoplay: false, loop: false,
                                                          obeyMuteSwitch: true, volume: 1, playbackRate: 1, referrerPolicy: 'origin' };
                __native_audio('create', String(id), '{}');
                var ctx = { __id: id, __ls: ls };
                Object.keys(props).forEach(function(k) {
                    Object.defineProperty(ctx, k, {
                        enumerable: true,
                        get: function() { return props[k]; },
                        set: function(v) {
                            props[k] = (k === 'src' || k === 'referrerPolicy') ? String(v == null ? '' : v) : v;
                            var p = {}; p[k] = props[k];
                            __native_audio('set', String(id), JSON.stringify(p));
                        }
                    });
                });
                ['currentTime', 'duration', 'paused', 'buffered'].forEach(function(k) {
                    Object.defineProperty(ctx, k, { enumerable: true, get: function() {
                        var v = __audioState(id)[k];
                        return k === 'paused' ? v !== false : (v || 0);
                    } });
                });
                ctx.play = function() { __native_audio('play', String(id), '{}'); };
                ctx.pause = function() { __native_audio('pause', String(id), '{}'); };
                ctx.stop = function() { __native_audio('stop', String(id), '{}'); };
                ctx.seek = function(position) { __native_audio('seek', String(id), JSON.stringify({ position: Number(position) || 0 })); };
                ctx.destroy = function() { __native_audio('destroy', String(id), '{}'); delete __audios[id]; };
                __AUDIO_EVENTS.concat(extraEvents || []).forEach(function(N) {
                    var k = N.charAt(0).toLowerCase() + N.slice(1);
                    ls[k] = [];
                    ctx['on' + N] = function(cb) { if (typeof cb === 'function') ls[k].push(cb); };
                    ctx['off' + N] = function(cb) { ls[k] = cb ? ls[k].filter(function(f) { return f !== cb; }) : []; };
                });
                __audios[id] = ctx;
                return ctx;
            }
            globalThis.__resolveAudio = function(id, type, json) {
                var ctx = __audios[id];
                if (!ctx) return;
                var res = {};
                try { res = JSON.parse(json || '{}'); } catch (e) {}
                (ctx.__ls[type] || []).slice().forEach(function(cb) { try { cb(res); } catch (e) { console.error(e); } });
            };
            wx.createInnerAudioContext = function() { return __makeAudio(); };
            wx.setInnerAudioOption = function(o) { return __syncOk(o, 'setInnerAudioOption'); };
            wx.getAvailableAudioSources = function(o) {
                return __syncOk(o, 'getAvailableAudioSources', { audioSources: ['auto', 'buildInMic', 'headsetMic'] });
            };

            // 背景音频：全局唯一实例。设 src 即开播（微信语义）；标题等元数据交给宿主（事件 backgroundAudio），
            // 宿主据此画锁屏 / 通知栏的播放控件。onPrev / onNext 由宿主的控件触发（__resolveAudio(id,'prev'|'next')）
            var __bgAudio = null;
            wx.getBackgroundAudioManager = function() {
                if (__bgAudio) return __bgAudio;
                var m = __makeAudio(['Prev', 'Next']);
                var meta = { title: '', epname: '', singer: '', coverImgUrl: '', webUrl: '', protocol: 'http' };
                var srcDesc = Object.getOwnPropertyDescriptor(m, 'src');
                Object.defineProperty(m, 'src', {
                    enumerable: true,
                    get: srcDesc.get,
                    set: function(v) {
                        srcDesc.set.call(m, v);
                        __emitHostEvent({ type: 'backgroundAudio', src: String(v || ''), title: meta.title, singer: meta.singer,
                                          epname: meta.epname, coverImgUrl: meta.coverImgUrl });
                        if (v) m.play();
                    }
                });
                Object.keys(meta).forEach(function(k) {
                    Object.defineProperty(m, k, { enumerable: true, get: function() { return meta[k]; },
                                                  set: function(v) { meta[k] = String(v == null ? '' : v); } });
                });
                __bgAudio = m;
                return m;
            };

            // 旧接口 wx.createAudioContext(audioId)：配 <audio> 组件用，按 id 各一个内部实例
            var __legacyAudio = {};
            wx.createAudioContext = function(audioId) {
                var key = String(audioId || '');
                var inner = __legacyAudio[key] || (__legacyAudio[key] = __makeAudio());
                return {
                    setSrc: function(src) { inner.src = src; },
                    play: function() { inner.play(); },
                    pause: function() { inner.pause(); },
                    seek: function(p) { inner.seek(p); }
                };
            };

            // 录音要麦克风与系统权限，属于「要宿主才能做」的能力：管理器总是存在，
            // start() 时以 onError 告知（与 system.js 里默认 fail 的那批同一个原则）
            var __recorder = null;
            wx.getRecorderManager = function() {
                if (__recorder) return __recorder;
                var ls = {};
                var r = {};
                ['Start', 'Stop', 'Pause', 'Resume', 'Error', 'FrameRecorded', 'InterruptionBegin', 'InterruptionEnd'].forEach(function(N) {
                    ls[N] = [];
                    r['on' + N] = function(cb) { if (typeof cb === 'function') ls[N].push(cb); };
                });
                var fail = function(op) {
                    setTimeout(function() {
                        var res = { errMsg: 'operateRecorder:fail ' + op + ' 当前宿主未实现录音' };
                        ls.Error.slice().forEach(function(cb) { cb(res); });
                    }, 0);
                };
                r.start = function() { fail('start'); };
                r.pause = function() { fail('pause'); };
                r.resume = function() { fail('resume'); };
                r.stop = function() { fail('stop'); };
                __recorder = r;
                return r;
            };
