            // ───────── 网络请求 ─────────
            // 回调式、非阻塞：这里只把回调登记下来并把请求交给原生，
            // 响应由宿主每帧取回后调 __resolveRequest。
            var __requestSeq = 0;
            var __requestTasks = {};
            // 任务对象上的 on/off（RequestTask / DownloadTask / UploadTask 共用）
            function __on(task, k) { return function(cb) { if (typeof cb === 'function') task.listeners[k].push(cb); }; }
            function __off(task, k) { return function(cb) { task.listeners[k] = typeof cb === 'function' ? task.listeners[k].filter(function(f) { return f !== cb; }) : []; }; }
            function __emit(task, k, arg) {
                var l = task.listeners && task.listeners[k];
                if (!l) return;
                l = l.slice();
                for (var i = 0; i < l.length; i++) { try { l[i](arg); } catch (e) { console.error(e); } }
            }
            // 原生的过程事件（MiniApp::update 在处理完成结果之前调）：headers / chunk / progress
            globalThis.__netEvent = function(id, kind, json) {
                var task = __requestTasks[id];
                if (!task || task.aborted) {
                    // 没人要的分块也得取走，否则一直占着原生的字节仓
                    if (kind === 'chunk' && typeof __net_take_chunk === 'function') __net_take_chunk(id);
                    return;
                }
                if (kind === 'chunk') {
                    var buf = typeof __net_take_chunk === 'function' ? __net_take_chunk(id) : null;
                    if (buf) __emit(task, 'chunk', { data: buf });
                    return;
                }
                var p = JSON.parse(json);
                if (kind === 'headers') {
                    __emit(task, 'headers', { header: p.header, statusCode: p.statusCode, cookies: [] });
                } else if (kind === 'progress') {
                    var exp = p.expected || 0;
                    __emit(task, 'progress', { progress: exp > 0 ? Math.min(100, Math.floor(p.written * 100 / exp)) : 0,
                        totalBytesWritten: p.written, totalBytesExpectedToWrite: exp });
                }
            };

            wx.request = function(options) {
                options = options || {};
                var id = ++__requestSeq;
                var url = options.url || '';
                var method = (options.method || 'GET').toUpperCase();
                var header = options.header || options.headers || {};
                var ctype = '';
                for (var hk in header) { if (header.hasOwnProperty(hk) && hk.toLowerCase() === 'content-type') ctype = String(header[hk]).toLowerCase(); }
                function __formEncode(d) {
                    var ps = [];
                    for (var k in d) {
                        if (d.hasOwnProperty(k) && d[k] !== undefined) {
                            var v = d[k];
                            ps.push(encodeURIComponent(k) + '=' + encodeURIComponent(v !== null && typeof v === 'object' ? JSON.stringify(v) : v));
                        }
                    }
                    return ps.join('&');
                }
                // 微信按 dataType/method 决定怎么序列化 body：GET 的 data 拼进 query，
                // 其余情况对象转 JSON。
                var body = '';
                var data = options.data;
                if (data !== undefined && data !== null) {
                    if (method === 'GET' || method === 'HEAD') {
                        // 对象按键值拼 query；字符串原样拼上（从前字符串 data 被丢掉）
                        var q = typeof data === 'object' ? __formEncode(data) : String(data);
                        if (q) {
                            url += (url.indexOf('?') >= 0 ? '&' : '?') + q;
                        }
                    } else if (typeof data === 'string') {
                        body = data;
                    } else if (ctype.indexOf('application/x-www-form-urlencoded') >= 0 && typeof data === 'object') {
                        // 表单编码（官方：content-type 为 x-www-form-urlencoded 时对象转成 query string）
                        body = __formEncode(data);
                    } else {
                        try { body = JSON.stringify(data); } catch (e) { body = String(data); }
                    }
                }

                var task = {
                    id: id,
                    kind: 'request',
                    aborted: false,
                    // responseType:'arraybuffer'：正文不进 JSON，回调时按 id 直接取 ArrayBuffer（src/net_stream.rs）
                    binary: options.responseType === 'arraybuffer',
                    listeners: { headers: [], chunk: [] },
                    success: options.success,
                    fail: options.fail,
                    complete: options.complete,
                    // 微信默认 dataType='json'：2xx 且能解析就把 data 解析成对象
                    dataType: options.dataType === undefined ? 'json' : options.dataType,
                    abort: function() { this.aborted = true; }
                };
                __requestTasks[id] = task;

                if (typeof __native_request === 'function') {
                    __native_request(String(id), method, url, JSON.stringify(header), body,
                                     String(options.timeout || 0), String((task.binary ? 1 : 0) | (options.enableChunked ? 2 : 0)));
                } else {
                    // 没有原生桥（纯 JS 单测环境）：立刻失败，不要让调用方一直等
                    __resolveRequest(id, 0, '{}', '', 'request:fail 原生网络桥未注册');
                }

                // 返回 RequestTask（微信语义：可 abort；onHeadersReceived 在 success 之前，onChunkReceived 要 enableChunked）
                return {
                    abort: function() { task.abort(); },
                    onHeadersReceived: __on(task, 'headers'), offHeadersReceived: __off(task, 'headers'),
                    onChunkReceived: __on(task, 'chunk'), offChunkReceived: __off(task, 'chunk')
                };
            };

            // 由原生每帧回调：把一次请求的结果分发给 success / fail / complete
            function __resolveRequest(id, statusCode, headerJson, body, errMsg) {
                var task = __requestTasks[id];
                if (!task) { return false; }
                delete __requestTasks[id];
                if (task.aborted) {
                    // 微信：abort 之后走 fail（`request:fail abort`）再 complete。从前只调 complete，
                    // 在 fail 里收尾（关 loading、复位标记）的页面就卡住了；下载 / 上传任务的 complete 是空的，什么都不回
                    var abortRes = { errMsg: (task.kind || 'request') + ':fail abort' };
                    task.fail && task.fail(abortRes);
                    task.complete && task.complete(abortRes);
                    return true;
                }
                var header = {};
                try { header = JSON.parse(headerJson || '{}'); } catch (e) {}
                if (errMsg) {
                    var failRes = { errMsg: errMsg };
                    task.fail && task.fail(failRes);
                    task.complete && task.complete(failRes);
                    return true;
                }
                var data = body;
                if (task.binary) {
                    data = typeof __net_take_bytes === 'function' ? (__net_take_bytes(id) || new ArrayBuffer(0)) : new ArrayBuffer(0);
                } else if (task.dataType === 'json') {
                    try { data = JSON.parse(body); } catch (e) { /* 不是 JSON 就原样给字符串 */ }
                }
                var res = {
                    data: data,
                    statusCode: statusCode,
                    header: header,
                    cookies: [],
                    errMsg: 'request:ok'
                };
                // 注意：4xx/5xx 在微信里**仍然走 success**，只是 statusCode 不是 2xx
                task.success && task.success(res);
                task.complete && task.complete(res);
                return true;
            }

            // downloadFile / uploadFile（官方 api/network/download、upload）：原生线程收发，
            // 结果与 wx.request 走同一条回填路径（__resolveRequest），只是 success 的形状不同。
            // 下载的文件落在 `wxfile://tmp/…`（真实位置见 src/wxfile.rs），能直接给 <image src>、uploadFile 用；
            // 给了 `filePath`（`wxfile://usr/…`）就存到那里。上传按 multipart/form-data 发 `filePath` 指向的文件
            // （`wxfile://` 或包内路径）与 `formData`，回包 `data` 是字符串（微信语义）。
            wx.env = wx.env || { USER_DATA_PATH: 'wxfile://usr' };
            function __fileTask(kind, options, shape) {
                var id = ++__requestSeq;
                var task = {
                    id: id, kind: kind, aborted: false, dataType: kind === 'downloadFile' ? 'json' : 'text',
                    success: function(r) {
                        var res = shape(r);
                        res.errMsg = kind + ':ok';
                        options.success && options.success(res);
                        options.complete && options.complete(res);
                    },
                    fail: function(r) { options.fail && options.fail(r); options.complete && options.complete(r); },
                    complete: null,
                    listeners: { headers: [], progress: [] },
                    abort: function() { this.aborted = true; }
                };
                __requestTasks[id] = task;
                return { id: id, api: {
                    abort: function() { task.abort(); },
                    onProgressUpdate: __on(task, 'progress'), offProgressUpdate: __off(task, 'progress'),
                    onHeadersReceived: __on(task, 'headers'), offHeadersReceived: __off(task, 'headers')
                } };
            }
            wx.downloadFile = function(options) {
                options = options || {};
                var t = __fileTask('downloadFile', options, function(r) {
                    var d = (r.data && typeof r.data === 'object') ? r.data : {};
                    var out = { statusCode: r.statusCode, header: r.header };
                    if (d.filePath) out.filePath = d.filePath; else out.tempFilePath = d.tempFilePath;
                    return out;
                });
                if (typeof __native_download === 'function') {
                    __native_download(String(t.id), String(options.url || ''), JSON.stringify(options.header || {}),
                                      String(options.filePath || ''), String(options.timeout || 0));
                } else {
                    __resolveRequest(t.id, 0, '{}', '', 'downloadFile:fail 原生网络桥未注册');
                }
                return t.api;
            };
            wx.uploadFile = function(options) {
                options = options || {};
                var t = __fileTask('uploadFile', options, function(r) {
                    return { data: typeof r.data === 'string' ? r.data : JSON.stringify(r.data), statusCode: r.statusCode, header: r.header };
                });
                if (typeof __native_upload === 'function') {
                    __native_upload(String(t.id), String(options.url || ''), String(options.filePath || ''), String(options.name || 'file'),
                                    JSON.stringify(options.header || {}), JSON.stringify(options.formData || {}), String(options.timeout || 0));
                } else {
                    __resolveRequest(t.id, 0, '{}', '', 'uploadFile:fail 原生网络桥未注册');
                }
                return t.api;
            };

            // 动态字体（官方 api/ui/font/wx.loadFontFace）：`source` 是 `url("https://…")`、包内路径或
            // `wxfile://`。原生登记字族（src/text_faces.rs：只认 TTF / OTF，WOFF 明确 fail），远程的先下载；
            // 有结论后宿主回 __resolveFontFace，页面随之按新字体重排。
            // 回调形状：success / fail 都是 `{status, errMsg}`（status 为 'loaded' / 'error'）
            var __fontSeq = 0, __fontTasks = {};
            wx.loadFontFace = function(options) {
                options = options || {};
                var id = ++__fontSeq, task = { o: options }, promise;
                if (!options.success && !options.fail && !options.complete) {
                    promise = new Promise(function(resolve, reject) { task.resolve = resolve; task.reject = reject; });
                }
                __fontTasks[id] = task;
                var src = String(options.source || '');
                var m = /url\(\s*['"]?([^'")]+)['"]?\s*\)/.exec(src);
                if (m) src = m[1];
                __native_load_font(String(id), String(options.family || ''), src.trim());
                return promise;
            };
            globalThis.__resolveFontFace = function(id, ok, family, err) {
                var t = __fontTasks[id];
                if (!t) return;
                delete __fontTasks[id];
                var res = ok ? { status: 'loaded', errMsg: 'loadFontFace:ok' } : { status: 'error', errMsg: 'loadFontFace:fail ' + err };
                if (ok) { t.o.success && t.o.success(res); t.resolve && t.resolve(res); }
                else { t.o.fail && t.o.fail(res); t.reject && t.reject(res); }
                t.o.complete && t.o.complete(res);
            };
