            // ───────── 文件系统（官方 api/file/FileSystemManager）─────────
            // 原生侧 src/wxfs：路径权限（USER_DATA_PATH 可写；临时 / 已保存 / 代码包只读）与文件操作。
            // 每个操作一次同步桥调用 __native_fs(op, json) → {ok, …} / {ok:false, err}。
            // 异步版走 success / fail / complete（下一轮事件循环回调，与微信一样不是同步回调）；
            // 同步版（xxxSync）失败时 throw Error('xxxSync:fail <原因>')。
            // 二进制：写入的 ArrayBuffer 换成 {__b64}，读出没给 encoding 时还原成 ArrayBuffer。
            function __fsCall(op, args) {
                var a = {};
                for (var k in args) {
                    if (!args.hasOwnProperty(k) || typeof args[k] === 'function') continue;
                    var v = args[k];
                    if (k === 'data' && v !== null && typeof v === 'object') v = { __b64: __b64encode(v) };
                    a[k] = v;
                }
                var r;
                try { r = JSON.parse(__native_fs(op, JSON.stringify(a))); } catch (e) { r = { ok: false, err: String(e) }; }
                return r;
            }
            function __fsStats(st) {
                return {
                    mode: st.mode, size: st.size, lastAccessedTime: st.lastAccessedTime, lastModifiedTime: st.lastModifiedTime,
                    isDirectory: function() { return !!st.isDir; }, isFile: function() { return !st.isDir; }
                };
            }
            // op 的结果 → 回给页面的 res（异步）/ 返回值（同步）
            var __fsShape = {
                readFile: function(r) { return { data: r.b64 !== undefined ? __b64decode(r.b64) : r.data }; },
                readdir: function(r) { return { files: r.files }; },
                stat: function(r) {
                    if (r.list) return { stats: r.list.map(function(x) { return { path: x.path, stats: __fsStats(x.stats) }; }) };
                    return { stats: __fsStats(r.stats) };
                },
                fstat: function(r) { return { stats: __fsStats(r.stats) }; },
                getFileInfo: function(r) { return { size: r.size, digest: r.digest }; },
                saveFile: function(r) { return { savedFilePath: r.savedFilePath }; },
                getSavedFileList: function(r) { return { fileList: r.fileList }; },
                open: function(r) { return { fd: r.fd }; },
                write: function(r) { return { bytesWritten: r.bytesWritten }; }
            };
            // 同步版返回值：只有一个字段的直接给那个值（readFileSync 给内容、readdirSync 给数组…）
            var __fsSyncPick = { readFile: 'data', readdir: 'files', stat: 'stats', fstat: 'stats', saveFile: 'savedFilePath', open: 'fd' };
            // 同步版的位置参数 → 参数对象（微信同步 API 多数是位置参数，fd 类是对象）
            var __fsPos = {
                access: ['path'], appendFile: ['filePath', 'data', 'encoding'], copyFile: ['srcPath', 'destPath'],
                mkdir: ['dirPath', 'recursive'], readFile: ['filePath', 'encoding', 'position', 'length'], readdir: ['dirPath'],
                rename: ['oldPath', 'newPath'], rmdir: ['dirPath', 'recursive'], saveFile: ['tempFilePath', 'filePath'],
                stat: ['path', 'recursive'], unlink: ['filePath'], writeFile: ['filePath', 'data', 'encoding']
            };
            function __fsAsync(op, name) {
                return function(o) {
                    o = o || {};
                    var r = op === 'read' ? __fsRead(o) : __fsCall(op, o);
                    var res = r.ok ? ((__fsShape[op] || function() { return {}; })(r)) : {};
                    res.errMsg = name + (r.ok ? ':ok' : ':fail ' + r.err);
                    setTimeout(function() {
                        if (r.ok) { o.success && o.success(res); } else { o.fail && o.fail(res); }
                        o.complete && o.complete(res);
                    }, 0);
                };
            }
            function __fsSync(op, name) {
                return function() {
                    var args = arguments, o;
                    if (__fsPos[op] && (typeof args[0] !== 'object' || args[0] === null)) {
                        o = {};
                        __fsPos[op].forEach(function(k, i) { if (args[i] !== undefined) o[k] = args[i]; });
                    } else {
                        o = args[0] || {};
                    }
                    var r = op === 'read' ? __fsRead(o) : __fsCall(op, o);
                    if (!r.ok) throw new Error(name + ':fail ' + r.err);
                    if (op === 'read') return { bytesRead: r.bytesRead, arrayBuffer: o.arrayBuffer };
                    var res = (__fsShape[op] || function() { return undefined; })(r);
                    if (res && __fsSyncPick[op]) return res[__fsSyncPick[op]];
                    return res;
                };
            }
            // read：读出的字节写进调用方给的 arrayBuffer（offset 起）
            function __fsRead(o) {
                var r = __fsCall('read', { fd: o.fd, length: o.length === undefined ? (o.arrayBuffer ? o.arrayBuffer.byteLength - (o.offset || 0) : 0) : o.length,
                                           position: o.position });
                if (r.ok && o.arrayBuffer) {
                    var src = new Uint8Array(__b64decode(r.b64)), dst = new Uint8Array(o.arrayBuffer);
                    dst.set(src, o.offset || 0);
                }
                return r;
            }
            var __fsManager = null;
            wx.getFileSystemManager = function() {
                if (__fsManager) return __fsManager;
                var m = {};
                ['access', 'appendFile', 'copyFile', 'mkdir', 'readFile', 'readdir', 'rename', 'rmdir', 'saveFile', 'stat',
                 'unlink', 'writeFile', 'truncate', 'open', 'close', 'fstat', 'ftruncate', 'read', 'write'].forEach(function(op) {
                    m[op] = __fsAsync(op, op);
                    m[op + 'Sync'] = __fsSync(op, op + 'Sync');
                });
                ['getFileInfo', 'getSavedFileList', 'removeSavedFile', 'unzip'].forEach(function(op) { m[op] = __fsAsync(op, op); });
                m.readCompressedFile = m.readZipEntry = function(o) {
                    o = o || {};
                    var res = { errMsg: 'fail not supported' };
                    setTimeout(function() { o.fail && o.fail(res); o.complete && o.complete(res); }, 0);
                };
                m.readCompressedFileSync = function() { throw new Error('readCompressedFileSync:fail not supported'); };
                __fsManager = m;
                return m;
            };
            // 旧接口（基础库 2.x 前的 wx.saveFile / getSavedFileList / getSavedFileInfo / removeSavedFile / getFileInfo）
            function __fsLegacy(op, name, shape) {
                return function(o) {
                    o = o || {};
                    var r = __fsCall(op, o);
                    var res = r.ok ? shape(r) : {};
                    res.errMsg = name + (r.ok ? ':ok' : ':fail ' + r.err);
                    var promise;
                    if (!o.success && !o.fail && !o.complete) {
                        return new Promise(function(resolve, reject) { setTimeout(function() { (r.ok ? resolve : reject)(res); }, 0); });
                    }
                    setTimeout(function() {
                        if (r.ok) { o.success && o.success(res); } else { o.fail && o.fail(res); }
                        o.complete && o.complete(res);
                    }, 0);
                    return promise;
                };
            }
            wx.saveFile = __fsLegacy('saveFile', 'saveFile', function(r) { return { savedFilePath: r.savedFilePath }; });
            wx.getSavedFileList = __fsLegacy('getSavedFileList', 'getSavedFileList', function(r) { return { fileList: r.fileList }; });
            wx.removeSavedFile = __fsLegacy('removeSavedFile', 'removeSavedFile', function() { return {}; });
            wx.getFileInfo = __fsLegacy('getFileInfo', 'getFileInfo', function(r) { return { size: r.size, digest: r.digest }; });
            wx.getSavedFileInfo = function(o) {
                o = o || {};
                var r = __fsCall('stat', { path: o.filePath });
                var res = r.ok ? { size: r.stats.size, createTime: r.stats.lastModifiedTime } : {};
                res.errMsg = 'getSavedFileInfo' + (r.ok ? ':ok' : ':fail ' + r.err);
                setTimeout(function() {
                    if (r.ok) { o.success && o.success(res); } else { o.fail && o.fail(res); }
                    o.complete && o.complete(res);
                }, 0);
            };
