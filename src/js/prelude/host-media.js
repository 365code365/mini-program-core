            // ───────── 选图 / 选视频 / 选文件 / 存相册 / 打开文档：宿主点名之后的那一半 ─────────
            //
            // 宿主只管「打开系统的选择器、把文件交回来」。这里替它做三件与平台无关的事
            // （填 host.js 的 __hostCallHooks，没点名的 API 不经过这里）：
            // 1. 参数按官方文档的默认值补齐、夹到合法范围，宿主拿到的 args 总是完整的；
            // 2. 移动端 sourceType 同时有 album 与 camera 时，先弹微信那个「拍照 / 从手机相册选择」的
            //    操作菜单（引擎画的 showActionSheet，所有宿主长得一样），选完把 sourceType 收窄成一项再交给宿主。
            //    桌面（platform 不是 android / ios / ohos）没有相机，不问，直接交给宿主开文件对话框；
            // 3. 回包按官方形状补齐：chooseImage 的 tempFilePaths ↔ tempFiles 互推、chooseMedia 的 type /
            //    fileType、chooseMessageFile 的 name / type / time……宿主少给几个字段页面也不会读到 undefined。
            //
            // 宿主交回的文件写成 `file://<绝对路径>`，引擎在 __resolveHostCall 之前就把它们导入了
            // `wxfile://tmp/`（runtime/app_host.rs），所以 finish 里看到的已经是虚拟路径。
            (function() {
                var IMAGE_EXT = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'heic', 'heif'];
                var VIDEO_EXT = ['mp4', 'mov', 'm4v', '3gp', 'avi', 'mkv', 'webm'];

                function list(v, allowed, dflt) {
                    var a = Array.isArray(v) ? v : (typeof v === 'string' ? [v] : []);
                    a = a.filter(function(x) { return allowed.indexOf(x) >= 0; });
                    return a.length ? a : dflt.slice();
                }
                function clampInt(v, lo, hi, dflt) {
                    var n = Math.floor(Number(v));
                    if (!isFinite(n)) n = dflt;
                    return Math.max(lo, Math.min(hi, n));
                }
                function typeName(v) {
                    if (v === undefined) return 'Undefined';
                    if (v === null) return 'Null';
                    if (Array.isArray(v)) return 'Array';
                    var t = typeof v;
                    return t.charAt(0).toUpperCase() + t.slice(1);
                }
                function extOf(p) {
                    var m = /\.([A-Za-z0-9]{1,5})$/.exec(String(p || '').split(/[?#]/)[0]);
                    return m ? m[1].toLowerCase() : '';
                }
                function baseName(p) {
                    var s = String(p || '');
                    return s.slice(s.lastIndexOf('/') + 1);
                }
                function kindOf(p) {
                    var e = extOf(p);
                    return VIDEO_EXT.indexOf(e) >= 0 ? 'video' : IMAGE_EXT.indexOf(e) >= 0 ? 'image' : 'file';
                }
                function num(v) { var n = Number(v); return isFinite(n) ? n : 0; }
                function mobile() {
                    try {
                        var p = String(wx.getSystemInfoSync().platform || '').toLowerCase();
                        return p === 'android' || p === 'ios' || p === 'ohos' || p === 'harmony';
                    } catch (e) { return false; }
                }
                // sourceType 两项都有：先问用哪个（微信就是这个菜单），选完收窄成一项。取消 = `<api>:fail cancel`
                function askSource(name, a, cameraText, go, fail) {
                    if (a.sourceType.length < 2 || !mobile()) { go(a); return; }
                    wx.showActionSheet({
                        itemList: [cameraText, '从手机相册选择'],
                        success: function(r) { a.sourceType = [r.tapIndex === 0 ? 'camera' : 'album']; go(a); },
                        fail: function() { fail(name + ':fail cancel'); }
                    });
                }
                function requirePath(name) {
                    return function(a, go, fail) {
                        if (typeof a.filePath !== 'string' || !a.filePath) {
                            fail(name + ':fail parameter error: parameter.filePath should be String instead of ' + typeName(a.filePath) + ';');
                            return;
                        }
                        go(a);
                    };
                }

                // 官方 api/media/image/wx.chooseImage：count 默认 9（最多 9）
                __hostCallHooks.chooseImage = {
                    prepare: function(a, go, fail) {
                        a.count = clampInt(a.count, 1, 9, 9);
                        a.sizeType = list(a.sizeType, ['original', 'compressed'], ['original', 'compressed']);
                        a.sourceType = list(a.sourceType, ['album', 'camera'], ['album', 'camera']);
                        askSource('chooseImage', a, '拍照', go, fail);
                    },
                    finish: function(res) {
                        var files = Array.isArray(res.tempFiles) ? res.tempFiles : null;
                        var paths = Array.isArray(res.tempFilePaths) ? res.tempFilePaths : null;
                        if (!files) files = (paths || []).map(function(p) { return { path: p, size: 0 }; });
                        files = files.map(function(f) {
                            var p = f.path || f.tempFilePath || '';
                            return { path: p, size: num(f.size) };
                        });
                        res.tempFiles = files;
                        res.tempFilePaths = paths || files.map(function(f) { return f.path; });
                        return res;
                    }
                };

                // 官方 api/media/video/wx.chooseMedia：count 默认 9（基础库 2.25 起最多 20），
                // maxDuration 3~60 秒、默认 10；回包 { tempFiles, type }，type 按选到的文件推
                __hostCallHooks.chooseMedia = {
                    prepare: function(a, go, fail) {
                        a.count = clampInt(a.count, 1, 20, 9);
                        a.mediaType = list(a.mediaType, ['image', 'video', 'mix'], ['image', 'video']);
                        a.sourceType = list(a.sourceType, ['album', 'camera'], ['album', 'camera']);
                        a.maxDuration = clampInt(a.maxDuration, 3, 60, 10);
                        a.sizeType = list(a.sizeType, ['original', 'compressed'], ['original', 'compressed']);
                        a.camera = a.camera === 'front' ? 'front' : 'back';
                        askSource('chooseMedia', a, '拍摄', go, fail);
                    },
                    finish: function(res) {
                        var files = (Array.isArray(res.tempFiles) ? res.tempFiles : []).map(function(f) {
                            var p = f.tempFilePath || f.path || '';
                            var kind = f.fileType === 'video' || f.fileType === 'image' ? f.fileType : (kindOf(p) === 'video' ? 'video' : 'image');
                            var out = { tempFilePath: p, size: num(f.size), fileType: kind, width: num(f.width), height: num(f.height) };
                            if (kind === 'video') {
                                out.duration = num(f.duration);
                                out.thumbTempFilePath = f.thumbTempFilePath || '';
                            }
                            return out;
                        });
                        var kinds = files.map(function(f) { return f.fileType; });
                        res.tempFiles = files;
                        res.type = kinds.length && kinds.every(function(k) { return k === 'video'; }) ? 'video'
                            : kinds.some(function(k) { return k === 'video'; }) ? 'mix' : 'image';
                        return res;
                    }
                };

                // 官方 api/media/video/wx.chooseVideo：compressed 默认 true，maxDuration 默认 60
                __hostCallHooks.chooseVideo = {
                    prepare: function(a, go, fail) {
                        a.sourceType = list(a.sourceType, ['album', 'camera'], ['album', 'camera']);
                        a.compressed = a.compressed !== false;
                        a.maxDuration = clampInt(a.maxDuration, 1, 60, 60);
                        a.camera = a.camera === 'front' ? 'front' : 'back';
                        askSource('chooseVideo', a, '拍摄', go, fail);
                    },
                    finish: function(res) {
                        var f = res.tempFilePath ? res : (Array.isArray(res.tempFiles) && res.tempFiles[0]) || {};
                        var out = {
                            errMsg: res.errMsg,
                            tempFilePath: f.tempFilePath || f.path || '',
                            duration: num(f.duration),
                            size: num(f.size),
                            width: num(f.width),
                            height: num(f.height)
                        };
                        if (f.thumbTempFilePath) out.thumbTempFilePath = f.thumbTempFilePath;
                        return out;
                    }
                };

                // 官方 api/media/image/wx.chooseMessageFile：count 必填（1~100）；type all / video / image / file；
                // extension 只在 type 为 file 时有效。宿主那边是系统的文件选择器
                __hostCallHooks.chooseMessageFile = {
                    prepare: function(a, go, fail) {
                        if (typeof a.count !== 'number' || !isFinite(a.count)) {
                            fail('chooseMessageFile:fail parameter error: parameter.count should be Number instead of ' + typeName(a.count) + ';');
                            return;
                        }
                        a.count = clampInt(a.count, 1, 100, 1);
                        a.type = ['all', 'video', 'image', 'file'].indexOf(a.type) >= 0 ? a.type : 'all';
                        a.extension = (Array.isArray(a.extension) ? a.extension : [])
                            .map(function(e) { return String(e).replace(/^\.+/, ''); })
                            .filter(function(e) { return e.length > 0; });
                        go(a);
                    },
                    finish: function(res) {
                        var now = Math.floor(Date.now() / 1000);
                        res.tempFiles = (Array.isArray(res.tempFiles) ? res.tempFiles : []).map(function(f) {
                            var p = f.path || f.tempFilePath || '';
                            return {
                                path: p,
                                size: num(f.size),
                                name: f.name ? String(f.name) : baseName(p),
                                type: f.type === 'image' || f.type === 'video' || f.type === 'file' ? f.type : kindOf(f.name || p),
                                time: f.time ? num(f.time) : now
                            };
                        });
                        return res;
                    }
                };

                // 存相册 / 打开文档：filePath 必填。宿主从 hostCall 事件的 `files` 里拿到它的真实路径
                __hostCallHooks.saveImageToPhotosAlbum = { prepare: requirePath('saveImageToPhotosAlbum') };
                __hostCallHooks.saveVideoToPhotosAlbum = { prepare: requirePath('saveVideoToPhotosAlbum') };
                __hostCallHooks.openDocument = { prepare: requirePath('openDocument') };
            })();
