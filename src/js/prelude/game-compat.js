            // ==================== 小游戏：编解码（二进制桥，game-doc/02 §6、03 §6） ====================
            //
            // wx.encode / wx.decode：官方「工具」类 API（10 矩阵里「本轮补入」）。编码在原生做（js/bridge_bin.rs），
            // 合法 format：utf8 / utf-8 / ucs2 / ucs-2 / utf16le / utf-16le / latin1 / gbk，缺省 utf8。
            //
            // TextEncoder / TextDecoder 这两个**全局**先不提供：微信小游戏里有没有它们以取证为准。
            // （atob / btoa / performance / queueMicrotask 是 QuickJS 自带的全局，Context::full 就有，不归我们管。）
            // （game-doc/06 §7）。游戏用 typeof 探测，没有就走自己的 polyfill —— 多给一个微信没有的全局，反而可能
            // 让它走进只在别的平台上测过的分支。
            wx.encode = function(o) {
                o = o || {};
                if (typeof o.data !== 'string') throw new TypeError('wx.encode: data 应当是字符串');
                return __native_text_encode(o.data, o.format === undefined ? 'utf8' : String(o.format));
            };
            wx.decode = function(o) {
                o = o || {};
                return __native_text_decode(o.data, o.format === undefined ? 'utf8' : String(o.format));
            };
            // 原生 base64：codec.js 的纯 JS 逐字节实现，1MB 要几十毫秒；游戏的资源与网络二进制都过它
            wx.arrayBufferToBase64 = function(buf) { return __native_b64_encode(buf); };
            wx.base64ToArrayBuffer = function(s) { return __native_b64_decode(String(s)); };
            // ---------- Intl（最小实现）----------
            // QuickJS 没有 Intl；微信的 V8 / JSC 有。引擎常写 `typeof (Intl == null ? void 0 : Intl.Segmenter)`（Babel 编出来的），
            // 全局没有 Intl 就直接 ReferenceError（PixiJS 7 的 Text 模块在加载时就这么做，G9.1 实测）。
            // 这里只给形状对、结果按 zh-CN / en 的常见写法：Segmenter 按码点切（合并组合符与 ZWJ 序列），
            // NumberFormat / DateTimeFormat 走 toLocaleString 的简单格式，Collator 用 localeCompare。不做完整的 ICU。
            if (typeof globalThis.Intl === 'undefined') (function() {
                function joinable(cp) { return (cp >= 0x300 && cp <= 0x36f) || cp === 0x200d || (cp >= 0xfe00 && cp <= 0xfe0f) || (cp >= 0x1f3fb && cp <= 0x1f3ff) || (cp >= 0xe0020 && cp <= 0xe007f); }
                function Segmenter(locale, opt) { this.__g = (opt && opt.granularity) || 'grapheme'; }
                Segmenter.prototype.segment = function(s) {
                    s = String(s); var out = [], cps = Array.from(s), idx = 0, gran = this.__g;
                    if (gran === 'word') {
                        var re = /[\u4e00-\u9fff]|[A-Za-z0-9_']+|\s+|./gu, m;
                        while ((m = re.exec(s))) out.push({ segment: m[0], index: m.index, input: s, isWordLike: /[\w\u4e00-\u9fff]/u.test(m[0]) });
                    } else {
                        for (var i = 0; i < cps.length; i++) {
                            var seg = cps[i];
                            while (i + 1 < cps.length && (joinable(cps[i + 1].codePointAt(0)) || cps[i].codePointAt(0) === 0x200d)) seg += cps[++i];
                            out.push({ segment: seg, index: idx, input: s }); idx += seg.length;
                        }
                    }
                    out[Symbol.iterator] = Array.prototype[Symbol.iterator];
                    out.containing = function(k) { for (var j = 0; j < out.length; j++) if (k >= out[j].index && k < out[j].index + out[j].segment.length) return out[j]; };
                    return out;
                };
                function NumberFormat(locale, o) { this.__o = o || {}; }
                NumberFormat.prototype.format = function(n) {
                    var o = this.__o, d = o.maximumFractionDigits, v = Number(n);
                    var s = d !== undefined ? v.toFixed(d).replace(/\.?0+$/, o.minimumFractionDigits ? '' : '') : String(v);
                    if (o.minimumFractionDigits) s = v.toFixed(o.minimumFractionDigits);
                    if (o.useGrouping !== false) { var p = s.split('.'); p[0] = p[0].replace(/\B(?=(\d{3})+(?!\d))/g, ','); s = p.join('.'); }
                    if (o.style === 'percent') s = (v * 100).toFixed(o.maximumFractionDigits || 0) + '%';
                    return s;
                };
                NumberFormat.prototype.resolvedOptions = function() { return { locale: 'zh-CN', numberingSystem: 'latn' }; };
                function DateTimeFormat(locale, o) { this.__o = o || {}; }
                DateTimeFormat.prototype.format = function(d) { d = d === undefined ? new Date() : new Date(d); function p(x) { return (x < 10 ? '0' : '') + x; }
                    return d.getFullYear() + '/' + (d.getMonth() + 1) + '/' + d.getDate() + (this.__o.hour !== undefined ? ' ' + p(d.getHours()) + ':' + p(d.getMinutes()) : ''); };
                DateTimeFormat.prototype.resolvedOptions = function() { return { locale: 'zh-CN', calendar: 'gregory', timeZone: 'Asia/Shanghai' }; };
                function Collator() {}
                Collator.prototype.compare = function(a, b) { return String(a).localeCompare(String(b)); };
                function PluralRules(locale) { this.__l = String(locale || 'zh'); }
                PluralRules.prototype.select = function(n) { return /^en/.test(this.__l) && Number(n) === 1 ? 'one' : 'other'; };
                globalThis.Intl = { Segmenter: Segmenter, NumberFormat: NumberFormat, DateTimeFormat: DateTimeFormat, Collator: Collator, PluralRules: PluralRules,
                    getCanonicalLocales: function(l) { return [].concat(l || []); } };
            })();
