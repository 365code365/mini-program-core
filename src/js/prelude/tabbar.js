            // 原生 tabBar 的运行时 API（官方 api/ui/tab-bar/*）。改动交给宿主（host::tabbar_api）：
            // 它知道 app.json 里有没有原生 tabBar、index 越不越界，下一帧回 success / fail。
            // 自定义 tabBar（custom: true）与没有 tabBar 的小程序上一律 fail（微信同样不生效）。
            var __tabBarSeq = 0, __tabBarTasks = {};
            function __tabBarCall(op, options, extra) {
                options = options || {};
                var id = ++__tabBarSeq;
                var task = { op: op, o: options };
                var promise;
                if (!options.success && !options.fail && !options.complete) {
                    promise = new Promise(function(resolve, reject) { task.resolve = resolve; task.reject = reject; });
                }
                __tabBarTasks[id] = task;
                var req = { id: id, op: op };
                for (var k in extra) req[k] = extra[k];
                if (typeof __native_host_request === 'function') __native_host_request('tabBar', JSON.stringify(req));
                return promise;
            }
            globalThis.__resolveTabBar = function(id, ok, err) {
                var t = __tabBarTasks[id];
                if (!t) return;
                delete __tabBarTasks[id];
                var res = { errMsg: t.op + (ok ? ':ok' : ':fail ' + err) };
                if (ok) { t.o.success && t.o.success(res); t.resolve && t.resolve(res); }
                else { t.o.fail && t.o.fail(res); t.reject && t.reject(res); }
                t.o.complete && t.o.complete(res);
            };
            function __idx(o) { return { index: Number(o && o.index) }; }
            wx.setTabBarBadge = function(o) { return __tabBarCall('setTabBarBadge', o, { index: Number(o && o.index), text: String((o && o.text) || '') }); };
            wx.removeTabBarBadge = function(o) { return __tabBarCall('removeTabBarBadge', o, __idx(o)); };
            wx.showTabBarRedDot = function(o) { return __tabBarCall('showTabBarRedDot', o, __idx(o)); };
            wx.hideTabBarRedDot = function(o) { return __tabBarCall('hideTabBarRedDot', o, __idx(o)); };
            wx.setTabBarItem = function(o) {
                o = o || {};
                var e = { index: Number(o.index) };
                if (o.text !== undefined) e.text = String(o.text);
                if (o.iconPath !== undefined) e.iconPath = String(o.iconPath);
                if (o.selectedIconPath !== undefined) e.selectedIconPath = String(o.selectedIconPath);
                return __tabBarCall('setTabBarItem', o, e);
            };
            wx.setTabBarStyle = function(o) {
                o = o || {};
                var e = {};
                ['color', 'selectedColor', 'backgroundColor', 'borderStyle'].forEach(function(k) { if (o[k] !== undefined) e[k] = String(o[k]); });
                return __tabBarCall('setTabBarStyle', o, e);
            };
            wx.showTabBar = function(o) { return __tabBarCall('showTabBar', o, {}); };
            wx.hideTabBar = function(o) { return __tabBarCall('hideTabBar', o, {}); };
