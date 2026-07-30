            // 导航 API
            wx.navigateTo = function(options) {
                __pendingNavigation = {
                    type: 'navigateTo',
                    url: options.url
                };
                __native_print('[Navigate] navigateTo: ' + options.url);
                options.success && options.success();
            };
            
            wx.navigateBack = function(options) {
                options = options || {};
                var delta = options.delta || 1;
                __pendingNavigation = {
                    type: 'navigateBack',
                    delta: delta
                };
                // 逻辑层页面栈出栈（native 侧应配合使用实例复用而非重新加载页面 JS）
                __popPage(delta);
                __native_print('[Navigate] navigateBack');
                options.success && options.success();
            };
            
            wx.switchTab = function(options) {
                __pendingNavigation = {
                    type: 'switchTab',
                    url: options.url
                };
                __native_print('[Navigate] switchTab: ' + options.url);
                options.success && options.success();
            };
            
            // `redirectTo` 是**替换**当前页（栈深不变、退不回来），
            // `reLaunch` 是**清空整个栈**再开。宿主两条分支都实现了
            // （window.rs::process_navigation），这里以前却分别发成
            // 'navigateTo' / 'switchTab'，结果 redirectTo 变成入栈（还能退回本该关掉的页），
            // reLaunch 走 switchTab（目标不是 tab 页时直接失败、返回栈也没清）。
            wx.redirectTo = function(options) {
                __pendingNavigation = {
                    type: 'redirectTo',
                    url: options.url
                };
                __native_print('[Navigate] redirectTo: ' + options.url);
                options.success && options.success();
            };
            
            wx.reLaunch = function(options) {
                __pendingNavigation = {
                    type: 'reLaunch',
                    url: options.url
                };
                __native_print('[Navigate] reLaunch: ' + options.url);
                options.success && options.success();
            };
