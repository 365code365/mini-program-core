            // ───────── 媒体补齐：旧版语音 / 背景音频 / 录音接口、previewMedia、getVideoInfo、离屏画布 ─────────

            // ---------- 旧版语音（api/media/audio/wx.playVoice 等，基础库 1.6 起不再维护，仍可用）：一个内部 InnerAudioContext ----------
            var __voice = null;
            wx.playVoice = function(o) {
                o = o || {};
                if (!__voice) __voice = wx.createInnerAudioContext();
                var a = __voice;
                a.offEnded(); a.offError();
                a.onEnded(function() { var r = { errMsg: 'playVoice:ok' }; o.success && o.success(r); o.complete && o.complete(r); });
                a.onError(function(e) { var r = { errMsg: 'playVoice:fail ' + (e.errMsg || '') }; o.fail && o.fail(r); o.complete && o.complete(r); });
                if (a.src !== String(o.filePath || '')) a.src = String(o.filePath || '');
                a.play();
                // duration（秒）：到点停
                if (o.duration > 0) setTimeout(function() { if (__voice === a) a.stop(); }, o.duration * 1000);
            };
            wx.pauseVoice = function(o) { if (__voice) __voice.pause(); return __asyncOk(o, 'pauseVoice'); };
            wx.stopVoice = function(o) { if (__voice) __voice.stop(); return __asyncOk(o, 'stopVoice'); };

            // ---------- 旧版背景音频（wx.playBackgroundAudio 等）：同一个 BackgroundAudioManager ----------
            wx.playBackgroundAudio = function(o) {
                o = o || {};
                var m = wx.getBackgroundAudioManager();
                m.title = o.title || '';
                m.coverImgUrl = o.coverImgUrl || '';
                m.src = o.dataUrl || '';
                return __asyncOk(o, 'playBackgroundAudio');
            };
            wx.pauseBackgroundAudio = function(o) { wx.getBackgroundAudioManager().pause(); return __asyncOk(o, 'pauseBackgroundAudio'); };
            wx.stopBackgroundAudio = function(o) { wx.getBackgroundAudioManager().stop(); return __asyncOk(o, 'stopBackgroundAudio'); };
            wx.seekBackgroundAudio = function(o) { o = o || {}; wx.getBackgroundAudioManager().seek(Number(o.position) || 0); return __asyncOk(o, 'seekBackgroundAudio'); };
            // status：2 没有音乐在播放、1 播放中、0 暂停中（官方取值）
            wx.getBackgroundAudioPlayerState = function(o) {
                var m = wx.getBackgroundAudioManager();
                var status = !m.src ? 2 : (m.paused ? 0 : 1);
                return __asyncOk(o, 'getBackgroundAudioPlayerState', { duration: m.duration, currentPosition: m.currentTime, status: status,
                                                                       downloadPercent: m.src ? 100 : 0, dataUrl: m.src });
            };
            // 旧版监听：挂到同一个管理器上（host-capabilities.js 里的 on-only 版本被这里替换）
            wx.onBackgroundAudioPlay = function(cb) { wx.getBackgroundAudioManager().onPlay(cb); };
            wx.onBackgroundAudioPause = function(cb) { wx.getBackgroundAudioManager().onPause(cb); };
            wx.onBackgroundAudioStop = function(cb) { wx.getBackgroundAudioManager().onStop(cb); };

            // ---------- 旧版录音（wx.startRecord / stopRecord）：RecorderManager 上的一层 ----------
            var __legacyRec = null;
            wx.startRecord = function(o) {
                o = o || {};
                var r = wx.getRecorderManager();
                __legacyRec = o;
                r.onStop(function(res) {
                    var cb = __legacyRec; __legacyRec = null;
                    if (!cb) return;
                    var ok = { errMsg: 'startRecord:ok', tempFilePath: res.tempFilePath };
                    cb.success && cb.success(ok); cb.complete && cb.complete(ok);
                });
                r.onError(function(e) {
                    var cb = __legacyRec; __legacyRec = null;
                    if (!cb) return;
                    var bad = { errMsg: 'startRecord:fail ' + String(e.errMsg || '').replace(/^[^:]*:fail\s*/, '') };
                    cb.fail && cb.fail(bad); cb.complete && cb.complete(bad);
                });
                // 旧接口最长 60 秒
                r.start({ duration: 60000, format: 'aac' });
            };
            wx.stopRecord = function(o) { wx.getRecorderManager().stop(); return __asyncOk(o, 'stopRecord'); };

            // ---------- previewMedia（基础库 2.12.0）：图片走引擎的全屏预览；视频用封面图占位（引擎的预览层只放图） ----------
            wx.previewMedia = function(o) {
                o = o || {};
                var src = o.sources || [];
                if (!src.length) return __asyncFail(o, 'previewMedia', 'parameter error: sources should not be empty');
                var urls = [], cur = 0;
                src.forEach(function(s, i) {
                    var u = s.type === 'video' ? (s.poster || '') : s.url;
                    if (!u) return;
                    if (i === (Number(o.current) || 0)) cur = urls.length;
                    urls.push(String(u));
                });
                if (!urls.length) return __asyncFail(o, 'previewMedia', '没有可预览的图片（视频需要 poster）');
                wx.previewImage({ urls: urls, current: urls[cur] });
                return __asyncOk(o, 'previewMedia');
            };

            // ---------- getVideoInfo：原生读 MP4 / MOV 的元信息（src/wxfs/video_info.rs） ----------
            wx.getVideoInfo = function(o) {
                o = o || {};
                var r = {};
                try { r = JSON.parse(__native_video_info(String(o.src || ''))); } catch (e) { r = { err: String(e) }; }
                if (r.err) return __asyncFail(o, 'getVideoInfo', r.err);
                return __asyncOk(o, 'getVideoInfo', { orientation: r.orientation, type: r.type, duration: r.duration, size: r.size,
                                                     height: r.height, width: r.width, fps: r.fps, bitrate: r.bitrate });
            };

            // ---------- 离屏画布（api/canvas/wx.createOffscreenCanvas）：type 2d，与 <canvas type="2d"> 同一套上下文 ----------
            // 后备缓冲按 width × height 像素建（离屏画布没有 CSS 尺寸，也就没有像素比换算）
            var __offscreenSeq = 0;
            wx.createOffscreenCanvas = function(opt, height) {
                var o = (opt && typeof opt === 'object') ? opt : { width: opt, height: height };
                if (o.type && o.type !== '2d') return null;
                var w = Math.max(1, Math.floor(Number(o.width) || 300)), h = Math.max(1, Math.floor(Number(o.height) || 150));
                var id = 'offscreen_' + (++__offscreenSeq);
                __native_canvas_offscreen('2d#' + id, String(w), String(h));
                var node = __canvas2dNode({ id: id, width: w, height: h });
                // 改尺寸要连后备缓冲一起改
                var wd = Object.getOwnPropertyDescriptor(node, 'width'), hd = Object.getOwnPropertyDescriptor(node, 'height');
                Object.defineProperty(node, 'width', { get: wd.get, set: function(v) { wd.set(v); node._css[0] = node._w; __native_canvas_offscreen(node._key, String(node._w), String(node._h)); } });
                Object.defineProperty(node, 'height', { get: hd.get, set: function(v) { hd.set(v); node._css[1] = node._h; __native_canvas_offscreen(node._key, String(node._w), String(node._h)); } });
                return node;
            };
