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
                    exitPictureInPicture: noop
                };
            };
