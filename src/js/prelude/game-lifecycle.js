            // ==================== 小游戏：生命周期与分包（game-doc/04 G6.1 / G6.2，GT-LC / GT-P） ====================
            //
            // 生命周期：wx.onShow / offShow / onHide / offHide。原生在前后台切换时调 __gameLifecycle('show' | 'hide')
            // （game/frame.rs 的 set_background：音频先停、再派发 onHide；回前台先恢复、再派发 onShow）。
            // onShow 的参数 = 当时的 getEnterOptionsSync()；冷启动不派发 onShow（取证待定，按官方「小游戏回到前台」的说法）。
            // exitMiniProgram / restartMiniProgram：交给宿主（宿主事件 {type:'exit'} / {type:'restart'}），success 照常回调。
            //
            // 分包：原生把 game.json 的 subpackages 整理成 __gameSubpackages（入口模块键、占用字节，game/package.rs）。
            // - 没加载的分包里的模块 require 不到（与微信一致：分包代码下载之前不在运行环境里）；
            // - loadSubpackage({name}) 的 name 可以是分包名或 root；进度回调一次 100%（本地包），然后执行入口（只执行一次，
            //   重复加载直接 success），执行完才 success（GT-P1 / P3 / P4）；入口抛错 → fail 并进 wx.onError；
            // - 从独立分包启动（创建选项 entrySubpackage）时主包锁着，loadSubpackage({name:'__GAME__'}) 才加载并执行 game.js（GT-P5）。
            // fail 的文案待取证（GT-P2）。
            (function() {
                var shows = [], hides = [];
                function on(list) { return function(cb) { if (typeof cb === 'function' && list.indexOf(cb) < 0) list.push(cb); }; }
                function off(get, set) { return function(cb) { set(typeof cb === 'function' ? get().filter(function(f) { return f !== cb; }) : []); }; }
                wx.onShow = on(shows); wx.onHide = on(hides);
                wx.offShow = off(function() { return shows; }, function(v) { shows.length = 0; Array.prototype.push.apply(shows, v); });
                wx.offHide = off(function() { return hides; }, function(v) { hides.length = 0; Array.prototype.push.apply(hides, v); });
                globalThis.__gameLifecycle = function(kind) {
                    var list = (kind === 'show' ? shows : hides).slice();
                    for (var i = 0; i < list.length; i++) {
                        var arg = kind === 'show' ? JSON.parse(JSON.stringify(wx.getEnterOptionsSync())) : {};
                        __gameCall(list[i], null, [arg]);
                    }
                    return list.length;
                };
                function hostAction(type, api) {
                    return function(o) {
                        __native_host_event(JSON.stringify({ type: type }));
                        return __asyncOk(o, api);
                    };
                }
                wx.exitMiniProgram = hostAction('exit', 'exitMiniProgram');
                wx.restartMiniProgram = hostAction('restart', 'restartMiniProgram');

                // ── 分包 ──
                // 分包表由原生在前置代码之后、game.js 之前设（__gameSubpackages / __gameEntrySub / __gameMainBytes），第一次用时读
                var subs = null, loaded = {}, mainLoaded = true;
                function init() {
                    if (subs) return;
                    subs = globalThis.__gameSubpackages || [];
                    var entrySub = globalThis.__gameEntrySub || null;
                    mainLoaded = !entrySub;
                    if (entrySub) loaded[entrySub] = true;
                }
                function under(key, s) { return s.prefix ? key.indexOf(s.prefix) === 0 : key === s.entry; }
                function ownerOf(key) {
                    for (var i = 0; i < subs.length; i++) if (under(key, subs[i])) return subs[i];
                    return null;
                }
                function locked(key) {
                    init();
                    var s = ownerOf(key);
                    if (s) return loaded[s.name] ? null : s.name;
                    return mainLoaded ? null : '__GAME__';
                }
                var origRequire = __require;
                globalThis.__require = function(dir, req) {
                    var key = __resolveModulePath(dir, req);
                    var by = locked(key);
                    if (by) throw new Error('module "' + key + '" is not defined（所在分包 ' + by + ' 还没有 loadSubpackage）');
                    return origRequire(dir, req);
                };
                function find(name) {
                    init();
                    var n = String(name || '').replace(/^\/+/, '');
                    for (var i = 0; i < subs.length; i++) {
                        var s = subs[i];
                        if (s.name === name || s.root === n || s.root.replace(/\/+$/, '') === n.replace(/\/+$/, '')) return s;
                    }
                    return null;
                }
                wx.loadSubpackage = function(o) {
                    o = o || {};
                    var progress = [], task = { onProgressUpdate: function(cb) { if (typeof cb === 'function') progress.push(cb); } };
                    var name = o.name;
                    var done = function(ok, errMsg) {
                        var r = { errMsg: 'loadSubpackage:' + (ok ? 'ok' : 'fail ' + errMsg) };
                        if (ok) { if (typeof o.success === 'function') __gameCall(o.success, null, [r]); }
                        else if (typeof o.fail === 'function') __gameCall(o.fail, null, [r]);
                        if (typeof o.complete === 'function') __gameCall(o.complete, null, [r]);
                    };
                    setTimeout(function() {
                        init();
                        var main = name === '__GAME__';
                        var s = main ? null : find(name);
                        if (!main && !s) return done(false, 'subpackage not found: ' + name);
                        var bytes = main ? (globalThis.__gameMainBytes || 0) : s.bytes;
                        progress.forEach(function(cb) { __gameCall(cb, null, [{ progress: 100, totalBytesWritten: bytes, totalBytesExpectedToWrite: bytes }]); });
                        var already = main ? mainLoaded : loaded[s.name];
                        if (main) mainLoaded = true; else loaded[s.name] = true;
                        if (!already) {
                            try { __require('', main ? 'game' : s.entry); }
                            catch (e) {
                                // 入口没执行成功：下次还能再试（不留半个加载状态）
                                if (main) mainLoaded = false; else loaded[s.name] = false;
                                __gameCall(function() { throw e; }, null, []);
                                return done(false, e && e.message !== undefined ? e.message : String(e));
                            }
                        }
                        done(true);
                    }, 0);
                    return task;
                };
            })();
