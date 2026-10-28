            // 主域这边的开放数据域（game-doc/04 G9.7，原生在 game/opendata.rs）：wx.getOpenDataContext() → { canvas: sharedCanvas, postMessage }。
            // 开放数据域是另一个 QuickJS 运行时，第一次 getOpenDataContext 之后的下一帧启动（game.json 的 openDataContext 目录里的 index.js）。
            // 主域只能把 sharedCanvas 当图片源 drawImage，拿不到它的上下文、读不了它的像素（与微信一致）；消息单向（主域 → 开放数据域）。
            (function() {
                var shared = null, odc = null;
                function sharedCanvas() {
                    if (!shared) {
                        var i = __game_canvas_shared();
                        shared = new __GameCanvas([i[0], i[1], i[2], 0]);
                        Object.defineProperty(shared, '__shared', { value: true });
                        Object.defineProperty(shared, '__mainShared', { value: true });
                    }
                    return shared;
                }
                wx.getOpenDataContext = function() {
                    if (odc) return odc;
                    if (!__game_od_open()) console.warn('wx.getOpenDataContext：game.json 没有配 openDataContext，开放数据域不会运行');
                    odc = {
                        canvas: sharedCanvas(),
                        postMessage: function(m) {
                            if (m === null || typeof m !== 'object') throw new TypeError('OpenDataContext.postMessage 只接受对象');
                            __game_od_post(JSON.stringify(m));
                        }
                    };
                    return odc;
                };
                // 这两个只在开放数据域里有意义；主域调用给形状（sharedCanvas 同一块；onMessage 收不到东西）
                wx.getSharedCanvas = function() { return sharedCanvas(); };
                wx.onMessage = function() { console.warn('wx.onMessage 只在开放数据域里有效（主域收不到消息）'); };
            })();
