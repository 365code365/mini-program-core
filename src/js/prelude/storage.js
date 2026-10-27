            // ───────── 数据缓存（官方 api/storage）─────────
            // 原生侧是一张写穿到磁盘的表（bridge.rs 的 storage 那组原生函数，跨启动持久）；这里留一份读缓存。
            // 异步版（setStorage 等）**写入是同步的**（之后马上 getStorageSync 读得到），回调在下一轮事件循环，
            // 一个回调都没给时返回 Promise —— 与新版基础库一致。
            var __storage = {};

            // 一律按 JSON 存：读回来类型不变（微信语义）。从前字符串原样存、读时再 JSON.parse，
            // 存 '42' 读回数字 42、存 'true' 读回布尔。旧版存下的裸字符串 JSON.parse 不了，照旧按原文读回
            // 上限（官方「数据缓存」：单个 key 1MB，总共 10MB；按 UTF-8 字节算，与 getStorageInfo 的 currentSize 同一口径）。
            // 超了不写，抛错 / 走 fail —— 文案待取证（game-doc/06 §7）
            var __STORAGE_ITEM_MAX = 1024 * 1024, __STORAGE_TOTAL_MAX = 10 * 1024 * 1024;
            function __utf8len(str) {
                var n = 0;
                for (var i = 0; i < str.length; i++) {
                    var c = str.charCodeAt(i);
                    if (c < 0x80) n += 1; else if (c < 0x800) n += 2;
                    else if (c >= 0xd800 && c <= 0xdbff) { n += 4; i++; } else n += 3;
                }
                return n;
            }
            function __storageSet(key, data) {
                var value = data === undefined ? '' : JSON.stringify(data);
                if (value === undefined) value = '';
                var size = __utf8len(key) + __utf8len(value);
                if (size > __STORAGE_ITEM_MAX) throw new Error('exceed storage item max length 1MB');
                if (typeof __native_storage_size === 'function') {
                    var old = __storage[key] !== undefined ? __storage[key] : (typeof __native_storage_get === 'function' ? __native_storage_get(key) : '');
                    var oldSize = old ? __utf8len(key) + __utf8len(old) : 0;
                    if ((Number(__native_storage_size()) || 0) - oldSize + size > __STORAGE_TOTAL_MAX) throw new Error('exceed storage max size 10MB');
                }
                __storage[key] = value;
                if (typeof __native_storage_set === 'function') { __native_storage_set(key, value); }
            }
            wx.setStorageSync = function(key, data) {
                try { __storageSet(key, data); } catch (e) { throw new Error('setStorageSync:fail ' + e.message); }
            };

            wx.getStorageSync = function(key) {
                var value = __storage[key];
                if (value === undefined && typeof __native_storage_get === 'function') {
                    value = __native_storage_get(key);
                    if (value) { __storage[key] = value; }
                }
                if (value === undefined || value === '') { return ''; }
                try { return JSON.parse(value); } catch(e) { return value; }
            };

            wx.removeStorageSync = function(key) {
                delete __storage[key];
                if (typeof __native_storage_remove === 'function') { __native_storage_remove(key); }
            };

            wx.clearStorageSync = function() {
                __storage = {};
                if (typeof __native_storage_clear === 'function') { __native_storage_clear(); }
            };

            // 键取原生那张表（含上次会话存下、本次还没读过的）；占用按「键 + 值」的 UTF-8 字节，单位 KB
            wx.getStorageInfoSync = function() {
                var keys = [];
                try { keys = JSON.parse(__native_storage_keys()); } catch (e) { keys = Object.keys(__storage); }
                var bytes = Number(__native_storage_size()) || 0;
                return { keys: keys, currentSize: Math.ceil(bytes / 1024), limitSize: 10240 };
            };

            function __storageCb(o, name, run) {
                o = o || {};
                var res;
                try { res = run() || {}; res.errMsg = name + ':ok'; }
                catch (e) { res = { errMsg: name + ':fail ' + (e && e.message ? e.message : e) }; }
                var ok = /:ok$/.test(res.errMsg);
                if (!o.success && !o.fail && !o.complete) {
                    return new Promise(function(resolve, reject) { setTimeout(function() { (ok ? resolve : reject)(res); }, 0); });
                }
                setTimeout(function() {
                    if (ok) { o.success && o.success(res); } else { o.fail && o.fail(res); }
                    o.complete && o.complete(res);
                }, 0);
            }
            function __storageKey(k) {
                if (typeof k !== 'string' || !k) throw new Error('parameter error: parameter.key should be String instead of ' + (k === undefined ? 'Undefined' : typeof k) + ';');
                return k;
            }

            wx.setStorage = function(o) { o = o || {}; return __storageCb(o, 'setStorage', function() { __storageSet(__storageKey(o.key), o.data); }); };
            // 键不存在：微信是 fail（data not found），不是 success('')
            wx.getStorage = function(o) {
                o = o || {};
                return __storageCb(o, 'getStorage', function() {
                    var k = __storageKey(o.key);
                    var raw = __storage[k] !== undefined ? __storage[k] : __native_storage_get(k);
                    if (raw === undefined || raw === '') throw new Error('data not found');
                    return { data: wx.getStorageSync(k) };
                });
            };
            wx.removeStorage = function(o) { o = o || {}; return __storageCb(o, 'removeStorage', function() { wx.removeStorageSync(__storageKey(o.key)); }); };
            wx.clearStorage = function(o) { return __storageCb(o, 'clearStorage', function() { wx.clearStorageSync(); }); };
            wx.getStorageInfo = function(o) { return __storageCb(o, 'getStorageInfo', function() { return wx.getStorageInfoSync(); }); };

            // 批量（基础库 2.25.0）：kvList = [{key, value}]，读回按 keyList 的顺序给 dataList（没有的给 ''）
            wx.batchSetStorageSync = function(kvList) {
                (kvList || []).forEach(function(kv) { if (kv && typeof kv.key === 'string') wx.setStorageSync(kv.key, kv.value); });
            };
            wx.batchGetStorageSync = function(keyList) {
                return (keyList || []).map(function(k) { return wx.getStorageSync(k); });
            };
            wx.batchSetStorage = function(o) { o = o || {}; return __storageCb(o, 'batchSetStorage', function() { wx.batchSetStorageSync(o.kvList); }); };
            wx.batchGetStorage = function(o) { o = o || {}; return __storageCb(o, 'batchGetStorage', function() { return { dataList: wx.batchGetStorageSync(o.keyList) }; }); };

            // ---------- Buffer URL（基础库 2.32.0）：ArrayBuffer → 可给 <image> / <video> / 音频用的路径 ----------
            // 落成 wxfile://tmp 下的文件（扩展名按内容认）；revoke 删掉它
            wx.createBufferURL = function(buffer) { return __native_temp_write(__b64encode(buffer)); };
            wx.revokeBufferURL = function(url) { __native_temp_remove(String(url || '')); };

            // ---------- 周期性 / 预拉取（api/storage/background-fetch）：拉取由微信后台做，数据由宿主给 ----------
            var __bgFetchToken = '';
            wx.setBackgroundFetchToken = function(o) { o = o || {}; __bgFetchToken = String(o.token || ''); return __storageCb(o, 'setBackgroundFetchToken', function() {}); };
            wx.getBackgroundFetchToken = function(o) { return __storageCb(o, 'getBackgroundFetchToken', function() { return { token: __bgFetchToken }; }); };
            wx.getBackgroundFetchData = function(o) {
                return __storageCb(o, 'getBackgroundFetchData', function() {
                    var d = (typeof __hostSystemInfo === 'object' && __hostSystemInfo) ? __hostSystemInfo.backgroundFetchData : null;
                    if (!d) throw new Error('no background fetch data');
                    return { fetchedData: String(d.fetchedData || ''), timeStamp: Number(d.timeStamp) || Date.now(), path: String(d.path || ''),
                             query: String(d.query || ''), scene: Number(d.scene) || 1001 };
                });
            };

            // ---------- 弱网缓存（api/storage/cachemanager）：规则照收，引擎的网络层不走它（命中永远是 null） ----------
            wx.createCacheManager = function(opt) {
                var rules = [], ls = {};
                var m = {
                    mode: (opt && opt.mode) || 'weakNetwork', state: 0,
                    addRule: function(r) { rules.push(r); return 'rule_' + rules.length; },
                    addRules: function(rs) { return (rs || []).map(function(r) { return m.addRule(r); }); },
                    deleteRule: function() {}, deleteRules: function() {}, clearRules: function() { rules = []; },
                    deleteCache: function() {}, deleteCaches: function() {}, clearCaches: function() {},
                    match: function() { return Promise.resolve(null); },
                    start: function() { m.state = 1; return Promise.resolve(); },
                    stop: function() { m.state = 0; return Promise.resolve(); },
                    on: function(ev, cb) { (ls[ev] = ls[ev] || []).push(cb); },
                    off: function(ev, cb) { if (ls[ev]) ls[ev] = cb ? ls[ev].filter(function(f) { return f !== cb; }) : []; }
                };
                return m;
            };
