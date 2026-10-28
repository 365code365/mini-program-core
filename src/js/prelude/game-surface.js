            // ==================== 小游戏：官方 API 表面（game-doc/10、11 §4 GT-API） ====================
            //
            // 官方小游戏 API 总览里有、通用段里没有的那批，按等级补形状：
            // - H / W（宿主能力、依赖微信后台）：函数存在，宿主在 hostApis 点名就交给宿主（__hostInvoke），
            //   没点名就下一轮 fail（与微信「不支持 / 无权限」同形），带 __default 标记 → canIUse 报 false；
            // - 监听型：on / off 都有，宿主经 mr_app_dispatch_event 推（与 host-capabilities.js 同一个通道）；
            // - 引擎自己做、但在后面几期才实现的（createCanvas、触摸、showToast…）**不在这里放空壳**：
            //   空壳会让游戏以为能力存在而走错分支。它们登记在 game-doc/取证/api-surface-deferred.json，
            //   门禁（tests/game_e2e.rs 的 api1）要求「缺的 ⊆ 延期清单」，每一期做完从清单里删。
            (function() {
                function proxy(name) {
                    if (typeof wx[name] === 'function') return;
                    wx[name] = function(o) { return __hostInvoke(name, o); };
                    wx[name].__default = true;
                }
                [
                    // 关系链 / 开放数据（W：数据只有微信后台有，宿主后端可选提供）
                    'getFriendCloudStorage', 'getGroupCloudStorage', 'getUserCloudStorage', 'getUserCloudStorageKeys',
                    'setUserCloudStorage', 'removeUserCloudStorage', 'getPotentialFriendList', 'getRelationFriendList',
                    'getUserInteractiveStorage', 'modifyFriendInteractiveStorage', 'getFriendSendGiftStatus', 'sendGiftToFriend',
                    'getGroupInfo', 'getGroupMembersInfo', 'getGameClubData', 'getGameExptInfo', 'getPhoneNumber',
                    // 转发 / 分享（H）
                    'shareAppMessage', 'shareMessageToFriend', 'shareTextToGroup',
                    // 虚拟支付（W）
                    'requestMidasPayment', 'requestMidasFriendPayment', 'requestMidasPaymentGameItem', 'checkIsSupportMidasPayment',
                    // 其它宿主能力
                    'openCustomerServiceConversation', 'operateGameRecorderVideo', 'requestFacialRecognition',
                    'requestSubscribeSystemMessage', 'getOfficialComponentsInfo',
                    'checkHandoffEnabled', 'startHandoff', 'setHandoffQuery', 'exitChatTool'
                ].forEach(proxy);

                // 同步形态的宿主能力：没接宿主时给「不可用」的同步结果，不抛（游戏在启动路径上直接调它们）
                if (typeof wx.isChatTool !== 'function') { wx.isChatTool = function() { return false; }; wx.isChatTool.__default = true; }
                if (typeof wx.setMessageToFriendQuery !== 'function') {
                    // 官方返回 boolean（是否设置成功）；没有宿主就是没设上
                    wx.setMessageToFriendQuery = function() { return false; };
                    wx.setMessageToFriendQuery.__default = true;
                }

                // 数据分析：引擎能「做」的部分就是收下（写宿主日志事件），与微信一样不回结果
                ['reportScene', 'reportUserBehaviorBranchAnalytics'].forEach(function(name) {
                    if (typeof wx[name] === 'function') return;
                    wx[name] = function(o) {
                        try { __native_host_event(JSON.stringify({ type: 'report', api: name, data: o || {} })); } catch (e) {}
                        if (o && (o.success || o.complete)) __asyncOk(o, name, {});
                    };
                });

                // 离线模式（E）：引擎不区分离线模式，状态恒为「在线」，监听者不会被触发；调试开关只收下
                if (typeof wx.enableOfflineModeDebug !== 'function') {
                    wx.enableOfflineModeDebug = function(o) { __asyncOk(o || {}, 'enableOfflineModeDebug', {}); };
                }

                // ---------- 监听型（宿主推） ----------
                var listeners = {};
                function key(api) { return api.charAt(0).toLowerCase() + api.slice(1); }
                [
                    'ShareAppMessage', 'ShareTimeline', 'AddToFavorites', 'ShareMessageToFriend', 'Handoff',
                    'DirectAdStatusChange', 'OfficialComponentsInfoChange', 'InteractiveStorageModified', 'OfflineModeStateChange'
                ].forEach(function(api) {
                    var k = key(api);
                    if (typeof wx['on' + api] !== 'function') {
                        wx['on' + api] = function(cb) { if (typeof cb === 'function') (listeners[k] = listeners[k] || []).push(cb); };
                    }
                    if (typeof wx['off' + api] !== 'function') {
                        wx['off' + api] = function(cb) {
                            if (!listeners[k]) return;
                            listeners[k] = typeof cb === 'function' ? listeners[k].filter(function(f) { return f !== cb; }) : [];
                        };
                    }
                });
                // 接到 host-capabilities.js 的宿主推事件通道上：先给这里的监听者，再交给原来的分发
                var prev = globalThis.__hostDispatch;
                globalThis.__hostDispatch = function(name, payloadJson) {
                    var n = String(name || '');
                    if (n.indexOf('on') === 0 && n.length > 2 && n.charAt(2) === n.charAt(2).toUpperCase()) n = n.slice(2);
                    var k = key(n);
                    if (listeners[k]) {
                        var p = payloadJson;
                        if (typeof p === 'string') { try { p = JSON.parse(p || '{}'); } catch (e) { p = { data: payloadJson }; } }
                        var list = listeners[k].slice();
                        list.forEach(function(cb) { __gameCall(cb, null, [p || {}]); });
                        return list.length;
                    }
                    return typeof prev === 'function' ? prev(name, payloadJson) : 0;
                };
                // 广告状态（同步）：没有宿主时报「未接入」，形状待取证（game-doc/06 §7）
                if (typeof wx.getDirectAdStatusSync !== 'function') {
                    wx.getDirectAdStatusSync = function() { return { status: 0 }; };
                    wx.getDirectAdStatusSync.__default = true;
                }
            })();

            // ---------- wx.canIUse（小游戏版，04 G1.8）----------
            // 小程序那份把 view / scroll-view 等组件名也报 true；小游戏没有 WXML 组件，那些一律 false。
            // API 判断沿用同一规则：真实现 = true，「立刻 fail」的默认实现（__default）= false，不存在 = false
            wx.canIUse = function(schema) {
                var parts = String(schema || '').split('.');
                var head = parts[0];
                if (head === 'wx' && parts.length > 1) head = parts[1];
                if (!head) return false;
                var f = (typeof __wxImpl === 'object' ? __wxImpl : wx)[head];
                if (typeof f === 'function') return !f.__default;
                return f !== undefined && f !== null;
            };
