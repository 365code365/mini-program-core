            // 剪贴板、振动、操作菜单、图片预览，以及「要宿主才能做」的能力的默认实现。
            // 依赖 host.js 的 __emitHostEvent / __syncOk，必须排在它后面。

            // ---------- 剪贴板（官方 api/device/clipboard）----------
            // 写：引擎存一份 + 发宿主事件 `clipboard`（移动端宿主据此写系统剪贴板；桌面窗体直接走系统剪贴板）。
            // 微信写成功后会自己弹「内容已复制」的 Toast，这里同样弹
            wx.setClipboardData = function(o) {
                o = o || {};
                var data = String(o.data == null ? '' : o.data);
                if (typeof __native_clipboard_set === 'function') __native_clipboard_set(data);
                __emitHostEvent({ type: 'clipboard', data: data });
                wx.showToast({ title: '内容已复制', icon: 'none', duration: 1500 });
                return __syncOk(o, 'setClipboardData');
            };
            wx.getClipboardData = function(o) {
                var data = typeof __native_clipboard_get === 'function' ? String(__native_clipboard_get()) : '';
                return __syncOk(o, 'getClipboardData', { data: data });
            };

            // ---------- 振动（官方 api/device/vibrate）：交给宿主（宿主事件 `vibrate`）----------
            wx.vibrateShort = function(o) {
                o = o || {};
                __emitHostEvent({ type: 'vibrate', duration: 'short', style: String(o.type || 'medium') });
                return __syncOk(o, 'vibrateShort');
            };
            wx.vibrateLong = function(o) {
                __emitHostEvent({ type: 'vibrate', duration: 'long' });
                return __syncOk(o, 'vibrateLong');
            };

            // ---------- 操作菜单（官方 api/ui/interaction/wx.showActionSheet）----------
            // 面板由宿主画（host::ui_overlay::action_sheet），点完回 __resolveActionSheet
            var __asSeq = 0, __asTasks = {};
            wx.showActionSheet = function(o) {
                o = o || {};
                var id = ++__asSeq, task = { o: o }, promise;
                if (!o.success && !o.fail && !o.complete) {
                    promise = new Promise(function(resolve, reject) { task.resolve = resolve; task.reject = reject; });
                }
                __asTasks[id] = task;
                __native_host_request('actionSheet', JSON.stringify({
                    id: id,
                    alertText: o.alertText ? String(o.alertText) : '',
                    itemList: (o.itemList || []).map(function(s) { return String(s); }),
                    itemColor: o.itemColor ? String(o.itemColor) : '#000000'
                }));
                return promise;
            };
            globalThis.__resolveActionSheet = function(id, tapIndex, err) {
                var t = __asTasks[id];
                if (!t) return;
                delete __asTasks[id];
                if (tapIndex >= 0) {
                    var res = { tapIndex: tapIndex, errMsg: 'showActionSheet:ok' };
                    t.o.success && t.o.success(res); t.resolve && t.resolve(res);
                    t.o.complete && t.o.complete(res);
                } else {
                    var bad = { errMsg: 'showActionSheet:fail ' + (err || 'cancel') };
                    t.o.fail && t.o.fail(bad); t.reject && t.reject(bad);
                    t.o.complete && t.o.complete(bad);
                }
            };

            // ---------- 图片预览（官方 api/media/image/wx.previewImage）----------
            // 全屏浮层由宿主画（host::ui_overlay::image_preview）；打开即 success
            wx.previewImage = function(o) {
                o = o || {};
                var urls = (o.urls || []).map(function(s) { return String(s); });
                if (!urls.length) {
                    var bad = { errMsg: 'previewImage:fail parameter error: urls should not be empty' };
                    o.fail && o.fail(bad); o.complete && o.complete(bad);
                    if (!o.success && !o.fail && !o.complete) return Promise.reject(bad);
                    return;
                }
                __native_host_request('previewImage', JSON.stringify({ urls: urls, current: o.current === undefined ? urls[0] : o.current }));
                return __syncOk(o, 'previewImage');
            };

            // ---------- 要宿主才能做的能力：默认实现 ----------
            // 登录、支付、扫码、定位、选图……引擎自己做不了。宿主在创建选项 `hostApis` 里点名的，
            // __installHostApis 会把这里的默认实现换成真的（发 hostCall 给宿主）；没点名的就是下面这个：
            // **立刻**走 fail（下一轮事件循环），errMsg 写明原因。
            // 不再是 undefined：微信里这些 API 总是存在，框架包装层（uni.login → wx.login）拿到 undefined
            // 会直接抛 TypeError；而一个马上失败的函数，页面自己的 fail 分支就能接住。
            ['login', 'getUserProfile', 'getUserInfo', 'requestPayment', 'scanCode', 'getLocation', 'chooseLocation',
             'chooseImage', 'chooseMedia', 'chooseVideo', 'chooseMessageFile', 'makePhoneCall', 'authorize',
             // 相册写入、订阅消息、地址 / 位置 / 联系人、跳转其它小程序、设置页、文档预览：同样只有宿主能做
             'saveImageToPhotosAlbum', 'saveVideoToPhotosAlbum', 'requestSubscribeMessage', 'chooseAddress', 'openLocation',
             'navigateToMiniProgram', 'navigateBackMiniProgram', 'openSetting', 'openDocument', 'getShareInfo', 'chooseContact',
             'addPhoneContact', 'checkSession', 'getWeRunData', 'chooseInvoice', 'openCustomerServiceChat', 'startSoterAuthentication']
                .forEach(function(name) {
                    if (typeof wx[name] === 'function') return;
                    wx[name] = function(o) {
                        o = o || {};
                        var bad = { errMsg: name + ':fail 当前宿主未实现（宿主可在创建选项 hostApis 里接管）' };
                        var promise;
                        if (!o.success && !o.fail && !o.complete) {
                            promise = new Promise(function(resolve, reject) { setTimeout(function() { reject(bad); }, 0); });
                            return promise;
                        }
                        setTimeout(function() { o.fail && o.fail(bad); o.complete && o.complete(bad); }, 0);
                    };
                    wx[name].__default = true;
                });
