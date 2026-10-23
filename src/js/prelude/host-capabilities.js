            // ───────── 要宿主才能做的能力：默认实现 + 宿主推事件的统一通道 ─────────
            //
            // 1. 登录、支付、扫码、定位、蓝牙、NFC、传感器……引擎自己做不了。宿主在创建选项 `hostApis` 里
            //    点名的，__installHostApis 会把这里的默认实现换成真的（发 hostCall 给宿主）；没点名的
            //    **立刻 fail**（下一轮事件循环），errMsg 写明原因。
            //    不是 undefined：微信里这些 API 总是存在，框架包装层（uni.login → wx.login）拿到 undefined
            //    会直接抛 TypeError；而一个马上失败的函数，页面自己的 fail 分支就能接住。
            //    这些默认实现带 `__default` 标记：`wx.canIUse` 据此报 false（宿主没接 = 当前环境不可用）。
            // 2. 监听型 `wx.onXxx / offXxx`（传感器、蓝牙发现、定位变化、Wi-Fi、按键……）统一登记在这里，
            //    宿主用 `mr_app_dispatch_event(app, "accelerometerChange", json)`（或 `onAccelerometerChange`）推，
            //    引擎调 __hostDispatch 分发给监听者。没有宿主推，监听者就一直不触发 —— 与微信里没开始监听一样。
            (function() {
                function hostApi(name) {
                    if (typeof wx[name] === 'function') return;
                    wx[name] = function(o) { return __hostInvoke(name, o); };
                    wx[name].__default = true;
                }
                [
                    // 账号与开放数据
                    'login', 'pluginLogin', 'checkSession', 'getUserProfile', 'getUserInfo', 'authorize', 'authorizeForMiniProgram',
                    'openSetting', 'getShareInfo', 'getGroupEnterInfo', 'getWeRunData', 'shareToWeRun',
                    'chooseAddress', 'chooseInvoice', 'chooseInvoiceTitle', 'chooseLicensePlate', 'openCard', 'addCard',
                    'requestSubscribeMessage', 'requestSubscribeDeviceMessage', 'requestSubscribeEmployeeMessage',
                    'checkEmployeeRelation', 'bindEmployeeRelation', 'checkIsAddedToMyMiniProgram', 'showRedPackage',
                    'addVideoToFavorites', 'addFileToFavorites', 'openCustomerServiceChat',
                    'startSoterAuthentication', 'checkIsSupportSoterAuthentication', 'checkIsSoterEnrolledInDevice',
                    'requestFacialVerify', 'checkIsSupportFacialRecognition', 'openStoreOrderDetail', 'openStoreCouponDetail',
                    'requestDeviceVoIP', 'getDeviceVoIPList', 'openStickerSetView', 'openStickerIPView', 'openSingleStickerView',
                    'openDesignerProfile', 'getStoreTraceId',
                    // 视频号
                    'reserveChannelsLive', 'openChannelsUserProfile', 'openChannelsLiveNoticeInfo', 'openChannelsLive',
                    'openChannelsEvent', 'openChannelsActivity', 'getChannelsShareKey', 'getChannelsLiveNoticeInfo', 'getChannelsLiveInfo',
                    // 支付
                    'requestPayment', 'requestOrderPayment', 'requestPluginPayment', 'requestVirtualPayment', 'requestCommonPayment',
                    'requestMerchantTransfer', 'requestAppleSubscribeSign', 'requestAutomaticRepayment', 'verifyHKPaymentPassword',
                    'openHKOfflinePayView', 'jumpToOfflinePay',
                    // 跳转与转发
                    'navigateToMiniProgram', 'navigateBackMiniProgram', 'openEmbeddedMiniProgram', 'openOfficialAccountProfile',
                    'openOfficialAccountChat', 'openOfficialAccountArticle', 'openInquiriesTopic', 'openPhotoTopicView', 'copyRealname',
                    'postMessageToReferrerPage', 'postMessageToReferrerMiniProgram', 'updateWeChatApp',
                    'shareFileMessage', 'shareVideoMessage', 'shareToOfficialAccount', 'authPrivateMessage',
                    'shareAppMessageToGroup', 'shareImageToGroup', 'shareVideoToGroup', 'shareFileToGroup', 'shareEmojiToGroup',
                    'selectGroupMembers', 'openChatTool', 'notifyGroupMembers', 'getChatToolInfo', 'enterChatToolMode',
                    // 位置
                    'getLocation', 'getFuzzyLocation', 'chooseLocation', 'choosePoi', 'openLocation',
                    'startLocationUpdate', 'startLocationUpdateBackground', 'stopLocationUpdate',
                    // 媒体（选图 / 存相册 / 打开文档的参数归一在 host-media.js）
                    'chooseImage', 'chooseMedia', 'chooseVideo', 'chooseMessageFile', 'saveImageToPhotosAlbum',
                    'saveVideoToPhotosAlbum', 'saveMediaToPhotosAlbum', 'openDocument', 'saveFileToDisk',
                    'editImage', 'cropImage', 'compressVideo', 'openVideoEditor', 'checkDeviceSupportHevc',
                    'joinVoIPChat', 'join1v1Chat', 'exitVoIPChat', 'updateVoIPChatMuteConfig', 'subscribeVoIPVideoMembers', 'setEnable1v1Chat',
                    // 设备
                    'scanCode', 'makePhoneCall', 'sendSms', 'chooseContact', 'addPhoneContact', 'addPhoneCalendar', 'addPhoneRepeatCalendar',
                    'setVisualEffectOnCapture',
                    'startAccelerometer', 'stopAccelerometer', 'startCompass', 'stopCompass', 'startGyroscope', 'stopGyroscope',
                    'startDeviceMotionListening', 'stopDeviceMotionListening',
                    'openBluetoothAdapter', 'closeBluetoothAdapter', 'getBluetoothAdapterState', 'startBluetoothDevicesDiscovery',
                    'stopBluetoothDevicesDiscovery', 'getBluetoothDevices', 'getConnectedBluetoothDevices', 'makeBluetoothPair',
                    'isBluetoothDevicePaired', 'createBLEConnection', 'closeBLEConnection', 'getBLEDeviceServices',
                    'getBLEDeviceCharacteristics', 'getBLEDeviceRSSI', 'getBLEMTU', 'setBLEMTU', 'readBLECharacteristicValue',
                    'writeBLECharacteristicValue', 'notifyBLECharacteristicValueChange', 'createBLEPeripheralServer',
                    'startBeaconDiscovery', 'stopBeaconDiscovery', 'getBeacons',
                    'startWifi', 'stopWifi', 'getWifiList', 'setWifiList', 'connectWifi', 'getConnectedWifi',
                    'startHCE', 'stopHCE', 'sendHCEMessage', 'getHCEState',
                    'getSecureElementPasses', 'canAddSecureElementPass', 'removeSecureElementPass', 'addPaymentPassGetCertificateData',
                    'addPaymentPassFinish', 'checkTransitCardSupport', 'getTransitCardList', 'getTransitCardInfo', 'getTransitCardCPLC',
                    'issueTransitCard', 'rechargeTransitCard', 'deleteTransitCard',
                    'startLocalServiceDiscovery', 'stopLocalServiceDiscovery',
                    // AI
                    'initFaceDetect', 'faceDetect', 'stopFaceDetect', 'getCommonConfig', 'openPrivacyContract'
                ].forEach(hostApi);

                // ---------- 宿主推事件 ----------
                var listeners = {};
                // `onAccelerometerChange` / `accelerometerChange` → `accelerometerChange`
                function eventKey(name) {
                    var n = String(name || '');
                    if (n.indexOf('on') === 0 && n.length > 2 && n.charAt(2) === n.charAt(2).toUpperCase()) n = n.slice(2);
                    return n.charAt(0).toLowerCase() + n.slice(1);
                }
                function makeEvent(api) {
                    var key = eventKey(api);
                    if (typeof wx['on' + api] !== 'function') {
                        wx['on' + api] = function(cb) { if (typeof cb === 'function') (listeners[key] = listeners[key] || []).push(cb); };
                    }
                    if (typeof wx['off' + api] !== 'function') {
                        wx['off' + api] = function(cb) {
                            if (!listeners[key]) return;
                            listeners[key] = cb ? listeners[key].filter(function(f) { return f !== cb; }) : [];
                        };
                    }
                }
                ['AccelerometerChange', 'CompassChange', 'GyroscopeChange', 'DeviceMotionChange',
                 'LocationChange', 'LocationChangeError',
                 'BluetoothDeviceFound', 'BluetoothAdapterStateChange', 'BLEConnectionStateChange', 'BLECharacteristicValueChange',
                 'BLEMTUChange', 'BLEPeripheralConnectionStateChanged', 'BeaconUpdate', 'BeaconServiceChange',
                 'WifiConnected', 'WifiConnectedWithPartialInfo', 'GetWifiList', 'HCEMessage', 'BatteryInfoChange',
                 'ScreenRecordingStateChanged', 'GeneratePoster', 'CopyUrl',
                 'LocalServiceFound', 'LocalServiceLost', 'LocalServiceResolveFail', 'LocalServiceDiscoveryStop',
                 'VoIPChatInterrupted', 'VoIPChatMembersChanged', 'VoIPChatSpeakersChanged', 'VoIPChatStateChanged',
                 'VoIPVideoMembersChanged', 'AudioInterruptionBegin', 'AudioInterruptionEnd', 'LazyLoadError', 'ApiCategoryChange',
                 'UserTriggerTranslation', 'UserOffTranslation', 'MenuButtonBoundingClientRectWeightChange', 'WindowStateChange',
                 'ParallelStateChange', 'EmbeddedMiniProgramHeightChange', 'KeyDown', 'KeyUp'].forEach(makeEvent);
                // 只有 on、没有 off 的（官方就是这样）
                // （旧版 onBackgroundAudioPlay / Pause / Stop 在 media-ext.js，挂在 BackgroundAudioManager 上）
                ['BackgroundFetchData', 'NeedPrivacyAuthorization']
                    .forEach(function(api) {
                        var key = eventKey(api);
                        if (typeof wx['on' + api] !== 'function') {
                            wx['on' + api] = function(cb) { if (typeof cb === 'function') listeners[key] = [cb]; };
                        }
                    });

                // 已有专门派发函数的那几个（app-info.js / device.js / lifecycle.js），同一个通道也认
                var special = {
                    networkStatusChange: function(p) { __dispatchNetworkStatus(p.networkType || (p.isConnected === false ? 'none' : 'wifi')); },
                    keyboardHeightChange: function(p) { __dispatchKeyboardHeight(p.height); },
                    memoryWarning: function(p) { __dispatchMemoryWarning(p.level); },
                    userCaptureScreen: function() { __dispatchUserCaptureScreen(); },
                    themeChange: function(p) { __dispatchApp('onThemeChange', { theme: p.theme === 'dark' ? 'dark' : 'light' }); },
                    error: function(p) { __dispatchApp('onError', p.message || p.error || String(p)); },
                    // 宿主承载的对象自己的事件（host-objects.js）
                    contextEvent: function(p) { __dispatchContextEvent(p); },
                    nfcDiscovered: function(p) { __dispatchContextEvent({ context: 'NFCAdapter', event: 'discovered', data: p }); },
                    pageNotFound: function(p) { __dispatchApp('onPageNotFound', p); },
                    unhandledRejection: function(p) { __dispatchApp('onUnhandledRejection', p); }
                };

                // 宿主推一个事件：返回收到它的监听者个数（0 = 没人听）
                globalThis.__hostDispatch = function(name, payloadJson) {
                    var key = eventKey(name);
                    var p = payloadJson;
                    if (typeof p === 'string') { try { p = JSON.parse(p || '{}'); } catch (e) { p = { data: payloadJson }; } }
                    if (p === null || p === undefined) p = {};
                    // 电量变了：getBatteryInfoSync 也要读到新值
                    if (key === 'batteryInfoChange' && typeof __hostSystemInfo === 'object' && __hostSystemInfo) {
                        __hostSystemInfo.battery = { level: p.level, isCharging: !!p.isCharging };
                    }
                    if (key === 'screenRecordingStateChanged') __screenRecording = p.state === 'start' ? 'on' : 'off';
                    if (special[key]) { try { special[key](p); } catch (e) { console.error(e); } return 1; }
                    // 录音的分片 / 打断 / 出错（audio.js 的 RecorderManager）
                    var rec = /^recorder(.+)$/.exec(key);
                    if (rec && typeof wx['__onRecorder' + rec[1]] === 'function') {
                        if (__recorder) { wx['__onRecorder' + rec[1]](p); return 1; }
                        return 0;
                    }
                    var list = (listeners[key] || []).slice();
                    list.forEach(function(cb) { try { cb(p); } catch (e) { console.error(e); } });
                    return list.length;
                };
                // 引擎自己也要推（按键、录屏状态……）
                globalThis.__hostListenerCount = function(name) { return (listeners[eventKey(name)] || []).length; };
            })();
            var __screenRecording = 'off';
