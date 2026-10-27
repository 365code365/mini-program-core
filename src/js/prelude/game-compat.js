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
