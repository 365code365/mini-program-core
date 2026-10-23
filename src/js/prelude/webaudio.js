            // ───────── wx.createWebAudioContext（官方 api/media/audio/WebAudioContext）与 createMediaAudioPlayer ─────────
            // 节点图、AudioParam、AudioBuffer、时钟、状态机按官方形状做全；**出声**走 InnerAudioContext：
            // - decodeAudioData：数据写进临时文件，原生解码（__native_audio_decode，与 InnerAudioContext 同一套：WAV / mp3 / AAC）
            //   出交错 PCM，拆成每声道的 Float32Array；
            // - AudioBufferSourceNode.start：缓冲没被改过就直接播原文件，改过（getChannelData / copyToChannel）就编成 16 位 WAV 再播；
            //   OscillatorNode.start：按波形与频率合成一段 WAV（时长 = stop 的时刻 - start 的时刻，没调 stop 按 30 秒）。
            //   音量 = 到 destination 那条路上所有 GainNode 的 gain 之积（start 那一刻取值）。
            // 取舍（写明）：不做逐样本的混音图 —— 滤波 / 压缩 / 延迟 / 声像节点能连能调参数，但不改变声音；
            // gain 的自动化曲线不实时生效；AnalyserNode 给静音数据；ScriptProcessorNode 的 onaudioprocess 不触发。
            function __AudioParam(v, min, max) {
                this.value = v; this.defaultValue = v;
                this.minValue = min === undefined ? -3.4028235e38 : min; this.maxValue = max === undefined ? 3.4028235e38 : max;
            }
            ['setValueAtTime', 'linearRampToValueAtTime', 'exponentialRampToValueAtTime', 'setTargetAtTime'].forEach(function(m) {
                __AudioParam.prototype[m] = function(v) { this.value = Number(v); return this; };
            });
            __AudioParam.prototype.setValueCurveAtTime = function(curve) { if (curve && curve.length) this.value = Number(curve[curve.length - 1]); return this; };
            __AudioParam.prototype.cancelScheduledValues = function() { return this; };
            __AudioParam.prototype.cancelAndHoldAtTime = function() { return this; };

            function __audioNode(ctx, kind, ins, outs, extra) {
                var n = { context: ctx, numberOfInputs: ins, numberOfOutputs: outs, channelCount: 2,
                    channelCountMode: 'max', channelInterpretation: 'speakers', __kind: kind, __out: [] };
                n.connect = function(dest) { if (dest && n.__out.indexOf(dest) < 0) n.__out.push(dest); return dest; };
                n.disconnect = function(dest) { n.__out = dest ? n.__out.filter(function(d) { return d !== dest; }) : []; };
                if (extra) for (var k in extra) n[k] = extra[k];
                return n;
            }
            // 从 node 出发能不能连到 destination，路上 gain 之积（取最大的那条）
            function __pathGain(node, dest, seen) {
                if (node === dest) return 1;
                seen = seen || [];
                if (seen.indexOf(node) >= 0) return 0;
                seen.push(node);
                var best = 0;
                (node.__out || []).forEach(function(o) {
                    var target = o instanceof __AudioParam ? null : o;
                    if (!target) return;
                    var g = __pathGain(target, dest, seen);
                    if (g > best) best = g;
                });
                var own = node.__kind === 'gain' ? Math.max(0, node.gain.value) : 1;
                return best * own;
            }
            function __AudioBuffer(o) {
                o = o || {};
                var ch = Math.max(1, o.numberOfChannels || 1), len = Math.max(0, o.length | 0), rate = o.sampleRate || 44100;
                this.numberOfChannels = ch; this.length = len; this.sampleRate = rate; this.duration = len / rate;
                this.__data = [];
                for (var i = 0; i < ch; i++) this.__data.push(o.__channels ? o.__channels[i] : new Float32Array(len));
                this.__src = o.__src || null;
            }
            __AudioBuffer.prototype.getChannelData = function(c) { this.__dirty = true; return this.__data[c | 0]; };
            __AudioBuffer.prototype.copyFromChannel = function(dst, c, start) { dst.set(this.__data[c | 0].subarray(start | 0, (start | 0) + dst.length)); };
            __AudioBuffer.prototype.copyToChannel = function(src, c, start) { this.__dirty = true; this.__data[c | 0].set(src, start | 0); };
            // 交错 16 位 PCM 的 WAV → wxfile://tmp/…
            function __wavUri(chs, rate) {
                var n = chs[0] ? chs[0].length : 0, c = chs.length, bytes = new Uint8Array(44 + n * c * 2), dv = new DataView(bytes.buffer);
                var w = function(o, s) { for (var i = 0; i < s.length; i++) bytes[o + i] = s.charCodeAt(i); };
                w(0, 'RIFF'); dv.setUint32(4, 36 + n * c * 2, true); w(8, 'WAVE'); w(12, 'fmt ');
                dv.setUint32(16, 16, true); dv.setUint16(20, 1, true); dv.setUint16(22, c, true); dv.setUint32(24, rate, true);
                dv.setUint32(28, rate * c * 2, true); dv.setUint16(32, c * 2, true); dv.setUint16(34, 16, true);
                w(36, 'data'); dv.setUint32(40, n * c * 2, true);
                for (var i = 0, o = 44; i < n; i++) for (var k = 0; k < c; k++, o += 2) {
                    var s = Math.max(-1, Math.min(1, chs[k][i] || 0));
                    dv.setInt16(o, s < 0 ? s * 0x8000 : s * 0x7fff, true);
                }
                return __native_temp_write(__b64encode(bytes.buffer));
            }
            // 一个会出声的源：start / stop 交给 InnerAudioContext
            function __sourceNode(ctx, kind, extra, makeUri) {
                var n = __audioNode(ctx, kind, 0, 1, extra);
                n.onended = null;
                var player = null, endTimer = null, startAt = null, stopAt = null;
                var fireEnded = function() {
                    if (endTimer) { clearTimeout(endTimer); endTimer = null; }
                    if (player) { player.destroy(); player = null; }
                    if (typeof n.onended === 'function') { try { n.onended({ type: 'ended', target: n }); } catch (e) { console.error(e); } }
                };
                n.start = function(when, offset, duration) {
                    startAt = Math.max(ctx.currentTime, Number(when) || 0);
                    var self = this;
                    // 下一轮再起：同一段代码里紧接着调的 stop(t) 要算进合成时长
                    setTimeout(function() {
                        if (ctx.state === 'closed') return;
                        var vol = Math.min(1, __pathGain(self, ctx.destination));
                        var uri = makeUri(startAt, stopAt);
                        var lead = Math.max(0, (startAt - ctx.currentTime) * 1000);
                        var len = kind === 'buffer' && self.buffer ? (Number(duration) || self.buffer.duration - (Number(offset) || 0)) / Math.max(0.01, self.playbackRate.value) : null;
                        if (stopAt !== null) len = Math.min(len === null ? Infinity : len, stopAt - startAt);
                        setTimeout(function() {
                            if (!uri || vol <= 0) { endTimer = setTimeout(fireEnded, len === null ? 0 : len * 1000); return; }
                            player = wx.createInnerAudioContext();
                            player.src = uri; player.volume = vol;
                            if (kind === 'buffer') { player.loop = !!self.loop; player.startTime = Number(offset) || 0; player.playbackRate = self.playbackRate.value; }
                            player.play();
                            if (len !== null && !(kind === 'buffer' && self.loop && stopAt === null)) endTimer = setTimeout(fireEnded, len * 1000);
                        }, lead);
                    }, 0);
                };
                n.stop = function(when) {
                    stopAt = Math.max(startAt === null ? 0 : startAt, Number(when) || ctx.currentTime);
                    if (player || endTimer) {
                        if (endTimer) clearTimeout(endTimer);
                        endTimer = setTimeout(fireEnded, Math.max(0, (stopAt - ctx.currentTime) * 1000));
                    }
                };
                n.__pause = function() { if (player) player.pause(); };
                n.__resume = function() { if (player) player.play(); };
                return n;
            }
            wx.createWebAudioContext = function() {
                var t0 = Date.now(), acc = 0, state = 'running', sources = [];
                var ctx = { sampleRate: 44100, onstatechange: null };
                Object.defineProperty(ctx, 'state', { get: function() { return state; } });
                Object.defineProperty(ctx, 'currentTime', { get: function() { return (acc + (state === 'running' ? Date.now() - t0 : 0)) / 1000; } });
                var setState = function(s) {
                    if (s === state) return;
                    if (state === 'running') acc += Date.now() - t0;
                    if (s === 'running') t0 = Date.now();
                    state = s;
                    if (typeof ctx.onstatechange === 'function') setTimeout(function() { ctx.onstatechange({ type: 'statechange' }); }, 0);
                };
                ctx.destination = __audioNode(ctx, 'destination', 1, 0, { maxChannelCount: 2 });
                ctx.listener = { positionX: new __AudioParam(0), positionY: new __AudioParam(0), positionZ: new __AudioParam(0),
                    forwardX: new __AudioParam(0), forwardY: new __AudioParam(0), forwardZ: new __AudioParam(-1),
                    upX: new __AudioParam(0), upY: new __AudioParam(1), upZ: new __AudioParam(0),
                    setPosition: function() {}, setOrientation: function() {} };
                var track = function(n) { sources.push(n); return n; };
                ctx.createBuffer = function(ch, length, rate) { return new __AudioBuffer({ numberOfChannels: ch, length: length, sampleRate: rate || ctx.sampleRate }); };
                ctx.createBufferSource = function() {
                    var n = __sourceNode(ctx, 'buffer', { buffer: null, loop: false, loopStart: 0, loopEnd: 0,
                        playbackRate: new __AudioParam(1), detune: new __AudioParam(0) }, function() {
                        var b = n.buffer;
                        if (!b) return null;
                        if (b.__src && !b.__dirty) return b.__src;
                        if (!b.__wav || b.__dirty) { b.__wav = __wavUri(b.__data, b.sampleRate); b.__dirty = false; }
                        return b.__wav;
                    });
                    return track(n);
                };
                ctx.createOscillator = function() {
                    var n = __sourceNode(ctx, 'oscillator', { type: 'sine', frequency: new __AudioParam(440), detune: new __AudioParam(0),
                        setPeriodicWave: function() { n.type = 'custom'; } }, function(start, stop) {
                        var secs = Math.min(30, stop === null ? 30 : Math.max(0, stop - start)), rate = 22050, len = Math.round(secs * rate);
                        var f = n.frequency.value * Math.pow(2, n.detune.value / 1200), d = new Float32Array(len);
                        for (var i = 0; i < len; i++) {
                            var ph = (i * f / rate) % 1;
                            d[i] = n.type === 'square' ? (ph < 0.5 ? 1 : -1) : n.type === 'sawtooth' ? 2 * ph - 1
                                : n.type === 'triangle' ? 1 - 4 * Math.abs(ph - 0.5) : Math.sin(2 * Math.PI * ph);
                            d[i] *= 0.5;
                        }
                        return len ? __wavUri([d], rate) : null;
                    });
                    return track(n);
                };
                ctx.createConstantSource = function() { return track(__sourceNode(ctx, 'constant', { offset: new __AudioParam(1) }, function() { return null; })); };
                ctx.createGain = function() { return __audioNode(ctx, 'gain', 1, 1, { gain: new __AudioParam(1) }); };
                ctx.createDelay = function(max) { return __audioNode(ctx, 'delay', 1, 1, { delayTime: new __AudioParam(0, 0, max || 1) }); };
                ctx.createBiquadFilter = function() {
                    return __audioNode(ctx, 'biquad', 1, 1, { type: 'lowpass', frequency: new __AudioParam(350), Q: new __AudioParam(1),
                        gain: new __AudioParam(0), detune: new __AudioParam(0), getFrequencyResponse: function(fs, mag, ph) { if (mag) mag.fill(1); if (ph) ph.fill(0); } });
                };
                ctx.createIIRFilter = function() { return __audioNode(ctx, 'iir', 1, 1, { getFrequencyResponse: function(fs, mag, ph) { if (mag) mag.fill(1); if (ph) ph.fill(0); } }); };
                ctx.createDynamicsCompressor = function() {
                    return __audioNode(ctx, 'compressor', 1, 1, { threshold: new __AudioParam(-24), knee: new __AudioParam(30), ratio: new __AudioParam(12),
                        attack: new __AudioParam(0.003), release: new __AudioParam(0.25), reduction: 0 });
                };
                ctx.createWaveShaper = function() { return __audioNode(ctx, 'waveshaper', 1, 1, { curve: null, oversample: 'none' }); };
                ctx.createPanner = function() {
                    return __audioNode(ctx, 'panner', 1, 1, { panningModel: 'equalpower', distanceModel: 'inverse', refDistance: 1, maxDistance: 10000,
                        rolloffFactor: 1, coneInnerAngle: 360, coneOuterAngle: 0, coneOuterGain: 0,
                        positionX: new __AudioParam(0), positionY: new __AudioParam(0), positionZ: new __AudioParam(0),
                        orientationX: new __AudioParam(1), orientationY: new __AudioParam(0), orientationZ: new __AudioParam(0),
                        setPosition: function() {}, setOrientation: function() {} });
                };
                ctx.createChannelMerger = function(n) { return __audioNode(ctx, 'merger', n || 6, 1); };
                ctx.createChannelSplitter = function(n) { return __audioNode(ctx, 'splitter', 1, n || 6); };
                ctx.createAnalyser = function() {
                    var a = __audioNode(ctx, 'analyser', 1, 1, { fftSize: 2048, minDecibels: -100, maxDecibels: -30, smoothingTimeConstant: 0.8,
                        getByteFrequencyData: function(arr) { arr.fill(0); }, getFloatFrequencyData: function(arr) { arr.fill(-Infinity); },
                        getByteTimeDomainData: function(arr) { arr.fill(128); }, getFloatTimeDomainData: function(arr) { arr.fill(0); } });
                    Object.defineProperty(a, 'frequencyBinCount', { get: function() { return a.fftSize / 2; } });
                    return a;
                };
                ctx.createScriptProcessor = function(size, ins, outs) { return __audioNode(ctx, 'script', 1, 1, { bufferSize: size || 4096, onaudioprocess: null, numberOfInputChannels: ins || 2, numberOfOutputChannels: outs || 2 }); };
                ctx.createPeriodicWave = function(real, imag) { return { real: real, imag: imag }; };
                ctx.decodeAudioData = function(data, success, fail) {
                    return new Promise(function(resolve, reject) {
                        var bad = function(msg) {
                            var err = { errMsg: 'decodeAudioData:fail ' + msg };
                            if (typeof fail === 'function') fail(err);
                            reject(err);
                        };
                        if (!(data instanceof ArrayBuffer) && !(data && data.buffer instanceof ArrayBuffer)) return bad('data 必须是 ArrayBuffer');
                        var ab = data instanceof ArrayBuffer ? data : data.buffer;
                        var uri = __native_temp_write(__b64encode(ab));
                        var r = {};
                        try { r = JSON.parse(__native_audio_decode(uri)); } catch (e) { r = { errMsg: String(e) }; }
                        if (!r.pcm && r.pcm !== '') { __native_temp_remove(uri); return bad(r.errMsg || 'decode error'); }
                        var all = new Float32Array(__b64decode(r.pcm)), c = Math.max(1, r.channels | 0), len = Math.floor(all.length / c), chs = [];
                        for (var k = 0; k < c; k++) {
                            var d = new Float32Array(len);
                            for (var i = 0; i < len; i++) d[i] = all[i * c + k];
                            chs.push(d);
                        }
                        var buf = new __AudioBuffer({ numberOfChannels: c, length: len, sampleRate: r.sampleRate, __channels: chs, __src: uri });
                        setTimeout(function() { if (typeof success === 'function') success(buf); resolve(buf); }, 0);
                    });
                };
                ctx.suspend = function() { if (state === 'running') { sources.forEach(function(s) { s.__pause(); }); setState('suspended'); } return Promise.resolve(); };
                ctx.resume = function() { if (state === 'suspended') { setState('running'); sources.forEach(function(s) { s.__resume(); }); } return Promise.resolve(); };
                ctx.close = function() { sources.forEach(function(s) { try { s.stop(); } catch (e) {} }); setState('closed'); return Promise.resolve(); };
                return ctx;
            };

            // createMediaAudioPlayer（官方 api/media/audio/MediaAudioPlayer）：播 VideoDecoder 解出的音轨。
            // VideoDecoder 由宿主承载（host-objects.js），引擎拿不到它的音频样本 —— 对象与方法按官方形状给，
            // start / stop / addAudioSource 都是 resolve 的 Promise，但没有声音（写明的降级）。
            wx.createMediaAudioPlayer = function() {
                var sources = [];
                return {
                    volume: 1,
                    start: function() { return Promise.resolve(); },
                    stop: function() { return Promise.resolve(); },
                    destroy: function() { sources = []; return Promise.resolve(); },
                    addAudioSource: function(src) { if (src && sources.indexOf(src) < 0) sources.push(src); return Promise.resolve(); },
                    removeAudioSource: function(src) { sources = sources.filter(function(s) { return s !== src; }); return Promise.resolve(); }
                };
            };
