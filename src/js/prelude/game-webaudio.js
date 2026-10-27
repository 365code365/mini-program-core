            // ==================== 小游戏：wx.createWebAudioContext 接真混音图（game/audio/graph、game/audio/web） ====================
            // 小程序仍用 webaudio.js 那份（节点能连能调、出声走 InnerAudioContext）；小游戏有 __game_webaudio 时整个换成这里：
            // 节点、连线、AudioParam 时间线都在原生，音频线程按 128 帧的量子逐节点处理，currentTime 是图实际渲染的帧数。
            // - 真做的节点：BufferSource、Oscillator（含 PeriodicWave）、ConstantSource、Gain、Delay、BiquadFilter、IIRFilter、
            //   WaveShaper（oversample 按 none）、Panner（equalpower；HRTF 按 equalpower）、ChannelMerger / Splitter、Analyser；
            // - 降级（写明）：DynamicsCompressor 直通、ScriptProcessor 输出静音且 onaudioprocess 不触发；
            // - AudioBuffer 的数据在 JS 与原生各一份：getChannelData / copyToChannel 之后标脏，下次交给源节点时重新上传；
            // - 节点对象被回收时（FinalizationRegistry）通知原生，原生在它没有活着的上游之后才拿掉（规范的寿命规则）。
            (function() {
                if (typeof __game_webaudio !== 'function') return;
                var N = __game_webaudio;
                function mkErr(s) {
                    var i = s.indexOf(': '), name = s.slice(0, i), msg = s.slice(i + 2), e;
                    if (name === 'RangeError') e = new RangeError(msg);
                    else if (name === 'TypeError') e = new TypeError(msg);
                    else { e = new Error(msg); e.name = name; }
                    return e;
                }
                function call(op, args, bin) {
                    var r = N(op, JSON.stringify(args || {}), bin);
                    var o = JSON.parse(r[0]);
                    if (o && typeof o === 'object' && o.error) throw mkErr(o.error);
                    return { v: o, bin: r[1] };
                }
                function fin(v, what) {
                    v = Number(v);
                    if (!isFinite(v)) throw new TypeError(what + ' 必须是有限数');
                    return v;
                }
                var reg = typeof FinalizationRegistry === 'function' ? new FinalizationRegistry(function(h) {
                    try { N(h.op, JSON.stringify(h.arg)); } catch (e) {}
                }) : null;
                var playing = {}, decodes = {};

                function AudioParam(target, name, range) {
                    this.__t = target; this.__name = name;
                    this.defaultValue = range[0]; this.minValue = range[1]; this.maxValue = range[2];
                    this.automationRate = 'a-rate';
                }
                AudioParam.prototype.__p = function(op, o, bin) {
                    o.op = op; o.name = this.__name;
                    for (var k in this.__t) o[k] = this.__t[k];
                    call('param', o, bin);
                    return this;
                };
                Object.defineProperty(AudioParam.prototype, 'value', {
                    get: function() { var o = { name: this.__name }; for (var k in this.__t) o[k] = this.__t[k]; return call('pget', o).v; },
                    set: function(v) { this.__p('value', { v: fin(v, 'value') }); }
                });
                AudioParam.prototype.setValueAtTime = function(v, t) { return this.__p('set', { v: fin(v, 'value'), t: fin(t, 'startTime') }); };
                AudioParam.prototype.linearRampToValueAtTime = function(v, t) { return this.__p('linear', { v: fin(v, 'value'), t: fin(t, 'endTime') }); };
                AudioParam.prototype.exponentialRampToValueAtTime = function(v, t) { return this.__p('exp', { v: fin(v, 'value'), t: fin(t, 'endTime') }); };
                AudioParam.prototype.setTargetAtTime = function(v, t, tau) { return this.__p('target', { v: fin(v, 'target'), t: fin(t, 'startTime'), tau: fin(tau, 'timeConstant') }); };
                AudioParam.prototype.setValueCurveAtTime = function(values, t, dur) {
                    var arr = new Float32Array(values || []);
                    for (var i = 0; i < arr.length; i++) if (!isFinite(arr[i])) throw new TypeError('曲线里不能有非有限数');
                    return this.__p('curve', { t: fin(t, 'startTime'), dur: fin(dur, 'duration') }, arr.buffer);
                };
                AudioParam.prototype.cancelScheduledValues = function(t) { return this.__p('cancel', { t: fin(t, 'cancelTime') }); };
                AudioParam.prototype.cancelAndHoldAtTime = function(t) { return this.__p('hold', { t: fin(t, 'cancelTime') }); };

                function AudioBuffer(o) {
                    o = o || {};
                    var ch = o.numberOfChannels | 0, len = o.length | 0, rate = Number(o.sampleRate);
                    if (ch < 1 || ch > 32 || len < 1 || !(rate >= 3000 && rate <= 768000)) {
                        var e = new Error('numberOfChannels 1~32、length > 0、sampleRate 3000~768000'); e.name = 'NotSupportedError'; throw e;
                    }
                    this.numberOfChannels = ch; this.length = len; this.sampleRate = rate; this.duration = len / rate;
                    this.__h = { op: 'bufFree', arg: { buf: o.__id || 0 } };
                    this.__data = null; this.__dirty = !o.__id;
                    if (reg && o.__id) reg.register(this, this.__h);
                }
                AudioBuffer.prototype.__load = function() {
                    if (this.__data) return this.__data;
                    var d = [], len = this.length, id = this.__h.arg.buf;
                    var bin = id ? call('bufGet', { buf: id }).bin : null;
                    for (var c = 0; c < this.numberOfChannels; c++) d.push(bin ? new Float32Array(bin, c * len * 4, len) : new Float32Array(len));
                    return (this.__data = d);
                };
                AudioBuffer.prototype.getChannelData = function(c) {
                    if (!(c >= 0 && c < this.numberOfChannels)) { var e = new Error('channel 越界'); e.name = 'IndexSizeError'; throw e; }
                    this.__dirty = true;
                    return this.__load()[c | 0];
                };
                AudioBuffer.prototype.copyFromChannel = function(dst, c, start) {
                    if (!(c >= 0 && c < this.numberOfChannels)) { var e = new Error('channel 越界'); e.name = 'IndexSizeError'; throw e; }
                    var s = this.__load()[c | 0], o = start | 0;
                    if (o < s.length) dst.set(s.subarray(o, Math.min(s.length, o + dst.length)));
                };
                AudioBuffer.prototype.copyToChannel = function(src, c, start) {
                    if (!(c >= 0 && c < this.numberOfChannels)) { var e = new Error('channel 越界'); e.name = 'IndexSizeError'; throw e; }
                    var d = this.__load()[c | 0], o = start | 0;
                    if (o < d.length) d.set(src.subarray(0, Math.min(src.length, d.length - o)), o);
                    this.__dirty = true;
                };
                // 原生那份是最新的就直接用；改过就整块重传（平面 f32）
                AudioBuffer.prototype.__sync = function() {
                    if (!this.__dirty && this.__h.arg.buf) return this.__h.arg.buf;
                    var d = this.__load(), len = this.length, all = new Float32Array(len * d.length);
                    for (var c = 0; c < d.length; c++) all.set(d[c], c * len);
                    var id = call('bufNew', { channels: d.length, length: len, sampleRate: this.sampleRate }, all.buffer).v.buf;
                    if (this.__h.arg.buf) { try { N('bufFree', JSON.stringify({ buf: this.__h.arg.buf })); } catch (e) {} }
                    else if (reg) reg.register(this, this.__h);
                    this.__h.arg.buf = id; this.__dirty = false;
                    return id;
                };

                function err(name, msg) { var e = new Error(msg); e.name = name; return e; }
                function props(n, list) {
                    list.forEach(function(k) {
                        var cur = n['__' + k];
                        Object.defineProperty(n, k, { enumerable: true, get: function() { return cur; },
                            set: function(v) { call('prop', { node: n.__id, k: k, v: v }); cur = v; } });
                    });
                }
                function AudioNode(ctx, kind, opts, given) {
                    var r = given || call('node', { ctx: ctx.__id, kind: kind, opts: opts || {} }).v, self = this;
                    this.__id = r.node; this.context = ctx; this.numberOfInputs = r.inputs; this.numberOfOutputs = r.outputs;
                    this.__cc = r.channelCount; this.__cm = r.channelCountMode; this.__ci = r.channelInterpretation;
                    (r.params || []).forEach(function(name, i) {
                        if (kind === 'panner' && i >= 6) return;
                        self[name] = new AudioParam({ node: r.node }, name, r.ranges[i]);
                    });
                    this.__ls = {};
                    if (reg && !given) reg.register(this, { op: 'free', arg: { node: r.node } });
                }
                Object.defineProperty(AudioNode.prototype, 'channelCount', { get: function() { return this.__cc; },
                    set: function(v) { call('prop', { node: this.__id, k: 'channelCount', v: Number(v) }); this.__cc = Number(v); } });
                Object.defineProperty(AudioNode.prototype, 'channelCountMode', { get: function() { return this.__cm; },
                    set: function(v) { call('prop', { node: this.__id, k: 'channelCountMode', v: String(v) }); if (/^(max|clamped-max|explicit)$/.test(v)) this.__cm = v; } });
                Object.defineProperty(AudioNode.prototype, 'channelInterpretation', { get: function() { return this.__ci; },
                    set: function(v) { call('prop', { node: this.__id, k: 'channelInterpretation', v: String(v) }); if (/^(speakers|discrete)$/.test(v)) this.__ci = v; } });
                AudioNode.prototype.connect = function(dest, output, input) {
                    if (dest instanceof AudioParam) {
                        if (!dest.__t.node) throw err('InvalidAccessError', 'listener 的参数不能接连线');
                        call('connect', { from: this.__id, to: dest.__t.node, out: output | 0, param: dest.__name });
                        return undefined;
                    }
                    if (!(dest instanceof AudioNode)) throw new TypeError('connect 的目标必须是 AudioNode 或 AudioParam');
                    if (dest.context !== this.context) throw err('InvalidAccessError', '不能跨上下文连线');
                    call('connect', { from: this.__id, to: dest.__id, out: output | 0, input: input | 0 });
                    return dest;
                };
                AudioNode.prototype.disconnect = function(a, b, c) {
                    var o = { from: this.__id };
                    if (typeof a === 'number') o.out = a;
                    else if (a instanceof AudioParam) { o.to = a.__t.node; o.param = a.__name; if (b !== undefined) o.out = b | 0; }
                    else if (a instanceof AudioNode) { o.to = a.__id; if (b !== undefined) o.out = b | 0; if (c !== undefined) o.input = c | 0; }
                    call('disconnect', o);
                };
                AudioNode.prototype.addEventListener = function(type, fn) { (this.__ls[type] = this.__ls[type] || []).push(fn); };
                AudioNode.prototype.removeEventListener = function(type, fn) { this.__ls[type] = (this.__ls[type] || []).filter(function(f) { return f !== fn; }); };

                function source(n) {
                    n.onended = null;
                    n.start = function(when, offset, duration) {
                        var o = { node: n.__id, when: when === undefined ? 0 : fin(when, 'when') };
                        if (offset !== undefined) o.offset = fin(offset, 'offset');
                        if (duration !== undefined) o.duration = fin(duration, 'duration');
                        if (n.__buf && n.__buf.__dirty) call('prop', { node: n.__id, k: 'bufferRefresh', v: n.__buf.__sync() });
                        call('start', o);
                        playing[n.__id] = n; // 播着的源节点不能被回收（ended 时放开）
                    };
                    n.stop = function(when) { call('stop', { node: n.__id, when: when === undefined ? 0 : fin(when, 'when') }); };
                    return n;
                }
                globalThis.__webAudioEvent = function(ev) {
                    if (ev.type === 'ended') {
                        var n = playing[ev.node];
                        if (!n) return;
                        delete playing[ev.node];
                        var e = { type: 'ended', target: n };
                        if (typeof n.onended === 'function') { try { n.onended(e); } catch (x) { console.error(x); } }
                        (n.__ls.ended || []).forEach(function(f) { try { f(e); } catch (x) { console.error(x); } });
                        return;
                    }
                    var d = decodes[ev.job];
                    if (!d) return;
                    delete decodes[ev.job];
                    if (ev.type === 'decoded') {
                        var b = new AudioBuffer({ numberOfChannels: ev.channels, length: ev.length, sampleRate: ev.sampleRate, __id: ev.buf });
                        if (typeof d.ok === 'function') { try { d.ok(b); } catch (x) { console.error(x); } }
                        d.resolve(b);
                    } else {
                        var er = { errMsg: ev.errMsg };
                        if (typeof d.fail === 'function') { try { d.fail(er); } catch (x) { console.error(x); } }
                        d.reject(er);
                    }
                };

                function pow2(v) { return v >= 32 && v <= 32768 && (v & (v - 1)) === 0; }
                function freqResp(n) {
                    return function(freqs, mag, phase) {
                        if (!(freqs instanceof Float32Array) || !(mag instanceof Float32Array) || !(phase instanceof Float32Array)) throw new TypeError('参数必须是 Float32Array');
                        if (mag.length !== freqs.length || phase.length !== freqs.length) throw err('InvalidAccessError', '三个数组长度必须相同');
                        var b = call('freqResponse', { node: n.__id }, new Float32Array(freqs).buffer).bin;
                        if (!b) return;
                        var all = new Float32Array(b), k = freqs.length;
                        mag.set(all.subarray(0, k)); phase.set(all.subarray(k, 2 * k));
                    };
                }
                function analyserData(n, what, Arr) {
                    return function(arr) {
                        if (!(arr instanceof Arr)) throw new TypeError('参数类型不对');
                        var b = call('analyser', { node: n.__id, what: what, n: arr.length }).bin;
                        if (b) arr.set(new Arr(b));
                    };
                }

                wx.createWebAudioContext = function() {
                    var r = call('ctxNew').v, state = 'running';
                    var ctx = { __id: r.ctx, sampleRate: r.sampleRate, onstatechange: null };
                    Object.defineProperty(ctx, 'state', { get: function() { return state; } });
                    Object.defineProperty(ctx, 'currentTime', { get: function() { return state === 'closed' ? ctx.__last : (ctx.__last = call('time', { ctx: ctx.__id }).v); } });
                    ctx.__last = 0;
                    var setState = function(s) {
                        if (s === state) return;
                        state = s;
                        setTimeout(function() { if (typeof ctx.onstatechange === 'function') ctx.onstatechange({ type: 'statechange', target: ctx }); }, 0);
                    };
                    ctx.destination = new AudioNode(ctx, 'destination', null, { node: r.dest, inputs: 1, outputs: 0, params: [], channelCount: 2, channelCountMode: 'explicit', channelInterpretation: 'speakers' });
                    ctx.destination.maxChannelCount = 2;
                    var L = {};
                    ['positionX', 'positionY', 'positionZ', 'forwardX', 'forwardY', 'forwardZ', 'upX', 'upY', 'upZ'].forEach(function(k, i) {
                        L[k] = new AudioParam({ listener: r.ctx }, k, [[0, 0, 0, 0, 0, -1, 0, 1, 0][i], -3.4028234663852886e38, 3.4028234663852886e38]);
                    });
                    L.setPosition = function(x, y, z) { L.positionX.value = x; L.positionY.value = y; L.positionZ.value = z; };
                    L.setOrientation = function(x, y, z, ux, uy, uz) {
                        L.forwardX.value = x; L.forwardY.value = y; L.forwardZ.value = z; L.upX.value = ux; L.upY.value = uy; L.upZ.value = uz;
                    };
                    ctx.listener = L;
                    var live = function() { if (state === 'closed') throw err('InvalidStateError', '上下文已经关闭'); };
                    var mk = function(kind, opts) { live(); return new AudioNode(ctx, kind, opts); };

                    ctx.createBuffer = function(ch, length, rate) { return new AudioBuffer({ numberOfChannels: ch, length: length, sampleRate: rate }); };
                    ctx.createBufferSource = function() {
                        var n = source(mk('buffer'));
                        n.__buffer = null; n.__loop = false; n.__loopStart = 0; n.__loopEnd = 0;
                        props(n, ['loop', 'loopStart', 'loopEnd']);
                        Object.defineProperty(n, 'buffer', { enumerable: true, get: function() { return n.__buf || null; },
                            set: function(b) {
                                if (b !== null && !(b instanceof AudioBuffer)) throw new TypeError('buffer 必须是 AudioBuffer');
                                call('prop', { node: n.__id, k: 'buffer', v: b ? b.__sync() : null });
                                n.__buf = b;
                            } });
                        return n;
                    };
                    ctx.createOscillator = function() {
                        var n = source(mk('oscillator')), type = 'sine';
                        Object.defineProperty(n, 'type', { enumerable: true, get: function() { return type; },
                            set: function(v) { call('prop', { node: n.__id, k: 'type', v: String(v) }); if (/^(sine|square|sawtooth|triangle)$/.test(v)) type = v; } });
                        n.setPeriodicWave = function(w) { call('prop', { node: n.__id, k: 'periodicWave', v: w && w.__id }); type = 'custom'; };
                        return n;
                    };
                    ctx.createConstantSource = function() { return source(mk('constant')); };
                    ctx.createGain = function() { return mk('gain'); };
                    ctx.createDelay = function(max) {
                        var n = mk('delay', { maxDelayTime: max === undefined ? 1 : Number(max) });
                        return n;
                    };
                    ctx.createBiquadFilter = function() {
                        var n = mk('biquad'), type = 'lowpass';
                        Object.defineProperty(n, 'type', { enumerable: true, get: function() { return type; },
                            set: function(v) { call('prop', { node: n.__id, k: 'type', v: String(v) }); if (/^(lowpass|highpass|bandpass|lowshelf|highshelf|peaking|notch|allpass)$/.test(v)) type = v; } });
                        n.getFrequencyResponse = freqResp(n);
                        return n;
                    };
                    ctx.createIIRFilter = function(ff, fb) {
                        var n = mk('iir', { feedforward: Array.prototype.slice.call(ff || []), feedback: Array.prototype.slice.call(fb || []) });
                        n.getFrequencyResponse = freqResp(n);
                        return n;
                    };
                    ctx.createWaveShaper = function() {
                        var n = mk('waveshaper'), curve = null, over = 'none';
                        Object.defineProperty(n, 'curve', { enumerable: true, get: function() { return curve; },
                            set: function(c) {
                                var arr = c === null ? null : new Float32Array(c);
                                call('prop', { node: n.__id, k: 'curve' }, arr ? arr.buffer : null);
                                curve = c === null ? null : (c instanceof Float32Array ? c : arr);
                            } });
                        Object.defineProperty(n, 'oversample', { enumerable: true, get: function() { return over; },
                            set: function(v) { if (/^(none|2x|4x)$/.test(v)) over = v; } });
                        return n;
                    };
                    ctx.createDynamicsCompressor = function() { var n = mk('compressor'); n.reduction = 0; return n; };
                    ctx.createPanner = function() {
                        var n = mk('panner'), model = 'equalpower';
                        n.__distanceModel = 'inverse'; n.__refDistance = 1; n.__maxDistance = 10000; n.__rolloffFactor = 1;
                        n.__coneInnerAngle = 360; n.__coneOuterAngle = 360; n.__coneOuterGain = 0;
                        props(n, ['distanceModel', 'refDistance', 'maxDistance', 'rolloffFactor', 'coneInnerAngle', 'coneOuterAngle', 'coneOuterGain']);
                        Object.defineProperty(n, 'panningModel', { enumerable: true, get: function() { return model; },
                            set: function(v) { if (v === 'equalpower' || v === 'HRTF') model = v; } });
                        n.setPosition = function(x, y, z) { n.positionX.value = x; n.positionY.value = y; n.positionZ.value = z; };
                        n.setOrientation = function(x, y, z) { n.orientationX.value = x; n.orientationY.value = y; n.orientationZ.value = z; };
                        return n;
                    };
                    ctx.createChannelMerger = function(k) { return mk('merger', { numberOfInputs: k === undefined ? 6 : k | 0 }); };
                    ctx.createChannelSplitter = function(k) { return mk('splitter', { numberOfOutputs: k === undefined ? 6 : k | 0 }); };
                    ctx.createAnalyser = function() {
                        var n = mk('analyser');
                        var v = { fftSize: 2048, minDecibels: -100, maxDecibels: -30, smoothingTimeConstant: 0.8 };
                        var check = {
                            fftSize: function(x) { if (!pow2(x)) throw err('IndexSizeError', 'fftSize 必须是 32~32768 的 2 的幂'); },
                            minDecibels: function(x) { if (x >= v.maxDecibels) throw err('IndexSizeError', 'minDecibels 必须小于 maxDecibels'); },
                            maxDecibels: function(x) { if (x <= v.minDecibels) throw err('IndexSizeError', 'maxDecibels 必须大于 minDecibels'); },
                            smoothingTimeConstant: function(x) { if (!(x >= 0 && x <= 1)) throw err('IndexSizeError', 'smoothingTimeConstant 必须在 0~1'); }
                        };
                        Object.keys(v).forEach(function(k) {
                            Object.defineProperty(n, k, { enumerable: true, get: function() { return v[k]; },
                                set: function(x) { x = Number(x); check[k](x); call('prop', { node: n.__id, k: k, v: x }); v[k] = x; } });
                        });
                        Object.defineProperty(n, 'frequencyBinCount', { get: function() { return v.fftSize / 2; } });
                        n.getFloatTimeDomainData = analyserData(n, 'floatTime', Float32Array);
                        n.getByteTimeDomainData = analyserData(n, 'byteTime', Uint8Array);
                        n.getFloatFrequencyData = analyserData(n, 'floatFreq', Float32Array);
                        n.getByteFrequencyData = analyserData(n, 'byteFreq', Uint8Array);
                        return n;
                    };
                    ctx.createScriptProcessor = function(size, ins, outs) {
                        var n = mk('script');
                        n.bufferSize = size || 4096; n.onaudioprocess = null;
                        return n;
                    };
                    ctx.createPeriodicWave = function(real, imag, o) {
                        var a = { disableNormalization: !!(o && o.disableNormalization) };
                        if (real) a.real = Array.prototype.slice.call(real);
                        if (imag) a.imag = Array.prototype.slice.call(imag);
                        return { __id: call('wave', a).v.wave };
                    };
                    ctx.decodeAudioData = function(data, ok, fail) {
                        return new Promise(function(resolve, reject) {
                            var bad = function(msg) {
                                var e = { errMsg: 'decodeAudioData:fail ' + msg };
                                if (typeof fail === 'function') { try { fail(e); } catch (x) { console.error(x); } }
                                reject(e);
                            };
                            if (!(data instanceof ArrayBuffer) && !(data && data.buffer instanceof ArrayBuffer)) return bad('data 必须是 ArrayBuffer');
                            var job;
                            try { job = call('decode', { ctx: ctx.__id }, data).v.job; } catch (e) { return bad(e.message); }
                            decodes[job] = { ok: ok, fail: fail, resolve: resolve, reject: reject };
                        });
                    };
                    var to = function(op, s) {
                        return function() {
                            if (state === 'closed') return Promise.reject(err('InvalidStateError', '上下文已经关闭'));
                            if (s === state) return Promise.resolve();
                            call(op, { ctx: ctx.__id });
                            if (op === 'close') ctx.__last = call('time', { ctx: ctx.__id }).v;
                            setState(s);
                            return Promise.resolve();
                        };
                    };
                    ctx.suspend = to('suspend', 'suspended');
                    ctx.resume = to('resume', 'running');
                    ctx.close = function() {
                        if (state !== 'closed') {
                            ctx.__last = ctx.currentTime;
                            call('close', { ctx: ctx.__id });
                            setState('closed');
                        }
                        return Promise.resolve();
                    };
                    return ctx;
                };
            })();
