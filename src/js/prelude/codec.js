            // ───────── 二进制与文本的编解码 ─────────
            // 桥上只有字符串，二进制（WebSocket 帧、文件内容）一律按 base64 过桥。
            // QuickJS 没有 btoa / atob / TextEncoder，这里自己实现；同时给出官方的
            // wx.arrayBufferToBase64 / wx.base64ToArrayBuffer（基础库 2.4.0 起废弃，但仍可用）。
            var __B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
            var __B64REV = (function() {
                var t = {};
                for (var i = 0; i < __B64.length; i++) t[__B64.charAt(i)] = i;
                t['-'] = 62; t['_'] = 63; // URL 安全字母表也认
                return t;
            })();
            function __toBytes(buf) {
                if (buf instanceof Uint8Array) return buf;
                if (buf instanceof ArrayBuffer) return new Uint8Array(buf);
                if (buf && buf.buffer instanceof ArrayBuffer) return new Uint8Array(buf.buffer, buf.byteOffset || 0, buf.byteLength);
                return new Uint8Array(0);
            }
            function __b64encode(buf) {
                var b = __toBytes(buf), out = '', i;
                for (i = 0; i + 2 < b.length; i += 3) {
                    var n = (b[i] << 16) | (b[i + 1] << 8) | b[i + 2];
                    out += __B64.charAt(n >> 18) + __B64.charAt((n >> 12) & 63) + __B64.charAt((n >> 6) & 63) + __B64.charAt(n & 63);
                }
                var rest = b.length - i;
                if (rest === 1) {
                    var n1 = b[i] << 16;
                    out += __B64.charAt(n1 >> 18) + __B64.charAt((n1 >> 12) & 63) + '==';
                } else if (rest === 2) {
                    var n2 = (b[i] << 16) | (b[i + 1] << 8);
                    out += __B64.charAt(n2 >> 18) + __B64.charAt((n2 >> 12) & 63) + __B64.charAt((n2 >> 6) & 63) + '=';
                }
                return out;
            }
            function __b64decode(s) {
                s = String(s || '').replace(/[\s=]/g, '');
                var len = Math.floor(s.length * 3 / 4), out = new Uint8Array(len), j = 0, acc = 0, bits = 0;
                for (var i = 0; i < s.length; i++) {
                    var v = __B64REV[s.charAt(i)];
                    if (v === undefined) continue;
                    acc = (acc << 6) | v; bits += 6;
                    if (bits >= 8) { bits -= 8; out[j++] = (acc >> bits) & 255; }
                }
                return out.buffer.slice(0, j);
            }
            // UTF-8：字符串 ↔ 字节（含代理对）
            function __utf8encode(str) {
                str = String(str);
                var out = [];
                for (var i = 0; i < str.length; i++) {
                    var c = str.charCodeAt(i);
                    if (c >= 0xd800 && c < 0xdc00 && i + 1 < str.length) {
                        var d = str.charCodeAt(i + 1);
                        if (d >= 0xdc00 && d < 0xe000) { c = 0x10000 + ((c - 0xd800) << 10) + (d - 0xdc00); i++; }
                    }
                    if (c < 0x80) out.push(c);
                    else if (c < 0x800) out.push(0xc0 | (c >> 6), 0x80 | (c & 63));
                    else if (c < 0x10000) out.push(0xe0 | (c >> 12), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
                    else out.push(0xf0 | (c >> 18), 0x80 | ((c >> 12) & 63), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
                }
                return new Uint8Array(out);
            }
            function __utf8decode(buf) {
                var b = __toBytes(buf), out = '', i = 0;
                while (i < b.length) {
                    var c = b[i++], cp;
                    if (c < 0x80) cp = c;
                    else if (c < 0xe0) cp = ((c & 31) << 6) | (b[i++] & 63);
                    else if (c < 0xf0) cp = ((c & 15) << 12) | ((b[i++] & 63) << 6) | (b[i++] & 63);
                    else cp = ((c & 7) << 18) | ((b[i++] & 63) << 12) | ((b[i++] & 63) << 6) | (b[i++] & 63);
                    if (cp >= 0x10000) { cp -= 0x10000; out += String.fromCharCode(0xd800 + (cp >> 10), 0xdc00 + (cp & 1023)); }
                    else out += String.fromCharCode(cp);
                }
                return out;
            }
            wx.arrayBufferToBase64 = function(buf) { return __b64encode(buf); };
            wx.base64ToArrayBuffer = function(s) { return __b64decode(s); };
