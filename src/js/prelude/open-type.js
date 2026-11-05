            // ───────── <button open-type> 的行为与回调（官方 component/button）─────────
            // 宿主在点击之后调（host::builtin_tap，按钮自己的 bindtap 已经先发了；disabled 的按钮不会到这里）：
            // attrs 是 open-type 与配套属性（lang / session-from / send-message-* / app-parameter / show-message-card …，
            // renderer event_attr `open_type_bind`），binds 是按钮自己身上的回调绑定 [{type, handler, owner, id, dataset}]
            // —— 回调只派给按钮自己（组件事件，不冒泡），可能在宿主回包之后异步发生，所以由这里自己 __dispatchEvent。
            //
            // 要宿主做的那几种走 hostCall，api 名是 `button.<open-type>`（宿主在 hostApis 里点名接管；errMsg 按惯例只写
            // 点后面那段：`getPhoneNumber:ok`）：getPhoneNumber / getRealtimePhoneNumber（回 {code}）、chooseAvatar（{avatarUrl}）、
            // contact（{path, query}）、launchApp、liveActivity（{code}）。没接的宿主：下一轮事件循环按 fail 回
            // （`<名>:fail 当前宿主未实现…`，与其余宿主能力同一句；官方真机上的失败原因随环境而定，待验）。
            // getUserInfo / openSetting 复用 wx.getUserInfo / wx.openSetting（同样是宿主能力，没接就 fail）。
            // share：调页面 onShareAppMessage({from:'button', target})，再发宿主事件 {type:'share', from, title, path, imageUrl}
            //   （返回值带 promise 的，等它 resolve 再发，官方给 3 秒）。feedback：发宿主事件 {type:'openFeedback'}（与小游戏的反馈按钮同一个）。
            // agreePrivacyAuthorization：引擎不收集用户信息，直接同意，发 bind:agreeprivacyauthorization {buttonId}。
            // target：按钮自己的 {id, dataset}（share 的 onShareAppMessage({target}) 要它）
            globalThis.__buttonOpenType = function(attrs, binds, target) {
                attrs = attrs || {};
                binds = binds || [];
                var type = attrs['open-type'];
                var ds = __coerceDataset((target && target.dataset) || {});
                var emit = function(name, detail) {
                    binds.forEach(function(b) {
                        if (b.type !== name) return;
                        var t = { id: b.id || '', dataset: b.dataset || {}, offsetLeft: 0, offsetTop: 0 };
                        __dispatchEvent(b.handler, { type: name, target: t, currentTarget: t, detail: detail || {} }, b.owner);
                    });
                };
                var viaHost = function(api, args, ok, bad) {
                    var both = function(name) { return function(res) { emit(name, res); }; };
                    args = args || {};
                    args.success = both(ok);
                    args.fail = both(bad || ok);
                    __hostInvoke('button.' + api, args);
                };
                switch (type) {
                    case 'share': return __buttonShare((target && target.id) || attrs.id || '', ds);
                    case 'getPhoneNumber':
                        return viaHost('getPhoneNumber', { phoneNumberNoQuotaToast: attrs['phone-number-no-quota-toast'] !== 'false' }, 'getphonenumber');
                    case 'getRealtimePhoneNumber':
                        return viaHost('getRealtimePhoneNumber', { phoneNumberNoQuotaToast: attrs['phone-number-no-quota-toast'] !== 'false' }, 'getrealtimephonenumber');
                    case 'chooseAvatar': return viaHost('chooseAvatar', {}, 'chooseavatar');
                    case 'contact':
                        return viaHost('contact', {
                            sessionFrom: attrs['session-from'] || '', sendMessageTitle: attrs['send-message-title'] || '',
                            sendMessagePath: attrs['send-message-path'] || '', sendMessageImg: attrs['send-message-img'] || '',
                            showMessageCard: attrs['show-message-card'] === 'true'
                        }, 'contact');
                    case 'launchApp': return viaHost('launchApp', { appParameter: attrs['app-parameter'] || '' }, 'launchapp', 'error');
                    case 'liveActivity': return viaHost('liveActivity', {}, 'createliveactivity');
                    case 'getUserInfo': {
                        var lang = attrs.lang || 'en';
                        var r = function(res) { emit('getuserinfo', res); };
                        return wx.getUserInfo({ lang: lang, withCredentials: true, success: r, fail: r });
                    }
                    case 'openSetting': {
                        var s = function(res) { emit('opensetting', res); };
                        return wx.openSetting({ success: s, fail: s });
                    }
                    case 'feedback': return __emitHostEvent({ type: 'openFeedback' });
                    case 'agreePrivacyAuthorization':
                        globalThis.__privacyAgreed = true;
                        return setTimeout(function() { emit('agreeprivacyauthorization', { buttonId: attrs.id || '' }); }, 0);
                    default: return;
                }
            };
            // open-type="share"：转发内容 = 页面 onShareAppMessage 的返回值（缺的字段用默认：当前页路径、空标题由宿主补）
            function __buttonShare(id, dataset) {
                var page = __currentPage;
                var q = (page && page.options) || {}, qs = [];
                for (var k in q) if (q.hasOwnProperty(k)) qs.push(encodeURIComponent(k) + '=' + encodeURIComponent(q[k]));
                var path = '/' + ((page && page.route) || '') + (qs.length ? '?' + qs.join('&') : '');
                var send = function(c) {
                    c = (c && typeof c === 'object') ? c : {};
                    __emitHostEvent({ type: 'share', from: 'button', title: c.title || '', path: c.path || path, imageUrl: c.imageUrl || '' });
                };
                var content = null;
                if (page && typeof page.onShareAppMessage === 'function') {
                    try { content = page.onShareAppMessage({ from: 'button', target: { id: id, dataset: dataset } }); } catch (e) { console.error(e); }
                }
                if (content && content.promise && typeof content.promise.then === 'function') {
                    var done = false, base = content;
                    var finish = function(c) { if (done) return; done = true; send(c || base); };
                    content.promise.then(function(c) { finish(Object.assign({}, base, c || {})); }, function() { finish(base); });
                    setTimeout(function() { finish(base); }, 3000);
                    return;
                }
                send(content);
            }
