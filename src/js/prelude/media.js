            // VideoContext（官方 api/media/video/VideoContext.html）：按 `<video id>` 找组件，
            // 命令交给宿主（它有布局树与播放器），事件由宿主回发到 video 组件自己的绑定上。
            // 第二个参数（组件实例）只决定在哪个组件模板里找 id；本引擎的 id 是页面级的，忽略。
            // sendDanmu 在当前位置加一条弹幕（components/video/extras.rs）；showStatusBar 这类没有状态栏的方法照常存在、不做事。
            function __videoCmd(id, type, extra) {
                var req = { id: String(id), type: type };
                if (extra) { for (var k in extra) req[k] = extra[k]; }
                if (typeof __native_host_request === 'function') {
                    __native_host_request('video', JSON.stringify(req));
                }
            }
            wx.createVideoContext = function(id, component) {
                var noop = function() {};
                return {
                    play: function() { __videoCmd(id, 'play'); },
                    pause: function() { __videoCmd(id, 'pause'); },
                    stop: function() { __videoCmd(id, 'stop'); },
                    seek: function(position) { __videoCmd(id, 'seek', { position: Number(position) || 0 }); },
                    playbackRate: function(rate) { __videoCmd(id, 'playbackRate', { rate: Number(rate) || 1 }); },
                    requestFullScreen: function(opts) {
                        var d = opts && opts.direction;
                        __videoCmd(id, 'requestFullScreen', (d === 0 || d === 90 || d === -90) ? { direction: d } : null);
                    },
                    exitFullScreen: function() { __videoCmd(id, 'exitFullScreen'); },
                    sendDanmu: function(d) { d = d || {}; __videoCmd(id, 'sendDanmu', { text: String(d.text || ''), color: String(d.color || '#ffffff') }); },
                    showStatusBar: noop,
                    hideStatusBar: noop,
                    requestBackgroundPlayback: noop,
                    exitBackgroundPlayback: noop,
                    exitPictureInPicture: noop,
                    // 投屏（DLNA）：引擎没有投屏协议，交给宿主（宿主事件 videoCasting，command 是方法名）
                    startCasting: function() { __emitHostEvent({ type: 'videoCasting', id: String(id), command: 'startCasting' }); },
                    switchCasting: function() { __emitHostEvent({ type: 'videoCasting', id: String(id), command: 'switchCasting' }); },
                    exitCasting: function() { __emitHostEvent({ type: 'videoCasting', id: String(id), command: 'exitCasting' }); },
                    reconnectCasting: function() { __emitHostEvent({ type: 'videoCasting', id: String(id), command: 'reconnectCasting' }); }
                };
            };
            // <image show-menu-by-longpress>（官方 component/image：长按显示发送给朋友、收藏、保存图片、搜一搜、识别码的菜单）。
            // 长按绑定由渲染层挂（components/image/attrs.rs，处理函数名 `__imgmenu:<src>`，__dispatchHandler 转到这里）。
            // 宿主在 hostApis 里接了 `image.showMenu`：交宿主出原生菜单（识别二维码 / 小程序码只有宿主做得了），参数 {src}；
            // 没接：引擎的 actionSheet 给三项 —— 发送给朋友 / 收藏发宿主事件 {type:'imageMenu', action:'share'|'favorite', src}，
            // 保存图片走 wx.saveImageToPhotosAlbum（远程图先下载）。返回 true = 长按被处理，这次触摸不再发 tap
            globalThis.__imageMenu = function(src, event) {
                if (__hostApiSet['image.showMenu']) {
                    __hostCall('image.showMenu', { src: src });
                    return true;
                }
                wx.showActionSheet({
                    itemList: ['发送给朋友', '收藏', '保存图片'],
                    success: function(res) {
                        if (res.tapIndex === 2) {
                            var save = function(path) { wx.saveImageToPhotosAlbum({ filePath: path, fail: function(e) { console.warn('[image] 保存图片失败', e && e.errMsg); } }); };
                            if (/^https?:\/\//.test(src)) wx.downloadFile({ url: src, success: function(d) { save(d.tempFilePath); } });
                            else save(src);
                        } else {
                            __emitHostEvent({ type: 'imageMenu', action: res.tapIndex === 0 ? 'share' : 'favorite', src: src });
                        }
                    }
                });
                return true;
            };
