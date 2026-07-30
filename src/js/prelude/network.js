            // ───────── 网络请求 ─────────
            // 回调式、非阻塞：这里只把回调登记下来并把请求交给原生，
            // 响应由宿主每帧取回后调 __resolveRequest。
            var __requestSeq = 0;
            var __requestTasks = {};

            wx.request = function(options) {
                options = options || {};
                var id = ++__requestSeq;
                var url = options.url || '';
                var method = (options.method || 'GET').toUpperCase();
                var header = options.header || options.headers || {};
                // 微信按 dataType/method 决定怎么序列化 body：GET 的 data 拼进 query，
                // 其余情况对象转 JSON。
                var body = '';
                var data = options.data;
                if (data !== undefined && data !== null) {
                    if (method === 'GET' || method === 'HEAD') {
                        var parts = [];
                        if (typeof data === 'object') {
                            for (var k in data) {
                                if (data.hasOwnProperty(k)) {
                                    parts.push(encodeURIComponent(k) + '=' + encodeURIComponent(data[k]));
                                }
                            }
                        }
                        if (parts.length) {
                            url += (url.indexOf('?') >= 0 ? '&' : '?') + parts.join('&');
                        }
                    } else if (typeof data === 'string') {
                        body = data;
                    } else {
                        try { body = JSON.stringify(data); } catch (e) { body = String(data); }
                    }
                }

                var task = {
                    id: id,
                    aborted: false,
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
                                     String(options.timeout || 0));
                } else {
                    // 没有原生桥（纯 JS 单测环境）：立刻失败，不要让调用方一直等
                    __resolveRequest(id, 0, '{}', '', 'request:fail 原生网络桥未注册');
                }

                // 返回 RequestTask（微信语义：可 abort）
                return {
                    abort: function() { task.abort(); },
                    onHeadersReceived: function() {},
                    offHeadersReceived: function() {}
                };
            };

            // 由原生每帧回调：把一次请求的结果分发给 success / fail / complete
            function __resolveRequest(id, statusCode, headerJson, body, errMsg) {
                var task = __requestTasks[id];
                if (!task) { return false; }
                delete __requestTasks[id];
                if (task.aborted) {
                    task.complete && task.complete({ errMsg: 'request:fail abort' });
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
                if (task.dataType === 'json') {
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

            // downloadFile：本引擎没有可写的临时文件系统，直接把 URL 当
            // tempFilePath 回去 —— <image src> 支持远程 URL，绝大多数用法（下载图片
            // 后拿去显示）因此仍然成立。真正需要落盘的场景要另做，不假装支持。
            wx.downloadFile = function(options) {
                options = options || {};
                var url = options.url || '';
                return wx.request({
                    url: url,
                    method: 'GET',
                    dataType: 'text',
                    success: function(res) {
                        options.success && options.success({
                            tempFilePath: url,
                            filePath: url,
                            statusCode: res.statusCode,
                            errMsg: 'downloadFile:ok'
                        });
                    },
                    fail: options.fail,
                    complete: options.complete
                });
            };

            // 动态字体：本引擎的字体来自系统（见 text.rs 的字体发现），不支持按 URL
            // 远程加载。这里必须**存在且回调 success** —— 应用普遍在 onLaunch 里连着
            // 加载好几个字体，抛异常会把整个 onLaunch 打断（框架的响应式层就此起不来，
            // 页面只剩静态骨架）。降级为「用系统字体渲染」，比整个应用起不来好得多。
            wx.loadFontFace = function(options) {
                options = options || {};
                var family = options.family || '';
                __native_print('[loadFontFace] 忽略远程字体 ' + family + '（改用系统字体）');
                options.success && options.success({ status: 'loaded' });
                options.complete && options.complete({ status: 'loaded' });
            };
