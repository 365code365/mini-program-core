            // ───────── 宿主外壳与宿主能力（移动端 SDK / Flutter 插件） ─────────
            //
            // 两类：
            // 1. 导航栏。引擎不画导航栏 —— 微信里它是原生外壳，标题、颜色、返回键都由宿主画。
            //    这几个 API 只把意图交给宿主（事件 `navigationBar`），回调同步成功。
            //    从前它们根本不存在，页面在 onLoad 里调一句 setNavigationBarTitle 就抛
            //    TypeError，后面的初始化全部没跑。
            // 2. 宿主实现的 `wx.*`。创建选项 `hostApis` 里点名的（如 chooseImage、getLocation）
            //    由 __installHostApis 装成真函数：调用发事件 `hostCall`，宿主处理完调
            //    mr_app_host_reply，引擎再调 __resolveHostCall 分发 success / fail / complete。
            //    没点名的：微信里**总是存在**的那几个能力（login / chooseImage / getLocation …）由
            //    system.js 给一个**立刻 fail** 的默认实现（框架包装层拿到 undefined 会直接 TypeError，
            //    马上失败的函数页面自己的 fail 分支接得住）；其余仍是 undefined —— 给一个永远不回的
            //    假函数比 undefined 更糟。
            function __emitHostEvent(ev) {
                if (typeof __native_host_event === 'function') {
                    try { __native_host_event(JSON.stringify(ev)); } catch (e) { console.error(e); }
                }
            }

            // 同步成功的 API：有回调走回调；一个回调都没给时按新版基础库返回 Promise
            function __syncOk(options, name, res) {
                res = res || {};
                res.errMsg = name + ':ok';
                var o = options || {};
                o.success && o.success(res);
                o.complete && o.complete(res);
                if (!o.success && !o.fail && !o.complete) return Promise.resolve(res);
            }

            wx.setNavigationBarTitle = function(o) {
                o = o || {};
                __emitHostEvent({ type: 'navigationBar', title: String(o.title == null ? '' : o.title) });
                return __syncOk(o, 'setNavigationBarTitle');
            };
            wx.setNavigationBarColor = function(o) {
                o = o || {};
                var ev = { type: 'navigationBar' };
                if (o.frontColor) ev.frontColor = String(o.frontColor);
                if (o.backgroundColor) ev.backgroundColor = String(o.backgroundColor);
                __emitHostEvent(ev);
                return __syncOk(o, 'setNavigationBarColor');
            };
            wx.showNavigationBarLoading = function(o) {
                __emitHostEvent({ type: 'navigationBar', loading: true });
                return __syncOk(o, 'showNavigationBarLoading');
            };
            wx.hideNavigationBarLoading = function(o) {
                __emitHostEvent({ type: 'navigationBar', loading: false });
                return __syncOk(o, 'hideNavigationBarLoading');
            };

            var __hostCallSeq = 0;
            var __hostCallTasks = {};
            // 某个宿主 API 可选的两步（host-media.js 填）：
            // - prepare(args, go, fail)：发 hostCall **之前** —— 按官方默认值补参数、先弹操作菜单问来源；
            //   准备好了调 go(args)，不发了调 fail(errMsg)（走 fail 回调，宿主那边什么都收不到）
            // - finish(res, args)：success **之前** —— 回包按官方形状补齐，返回新的 res
            var __hostCallHooks = {};

            // 宿主点名过的 API 名（含 `MapContext.getCenterLocation` 这种「对象方法」，见 __hostInvoke）
            var __hostApiSet = {};

            // 发一次 hostCall：options 里的回调留在这边，其余字段作为 args 交给宿主
            function __hostCall(name, options) {
                options = options || {};
                var id = ++__hostCallSeq;
                // 回调不进 args（宿主那边是 JSON，函数传不过去）
                var args = {};
                for (var k in options) {
                    if (options.hasOwnProperty(k) && typeof options[k] !== 'function') args[k] = options[k];
                }
                var task = { name: name, args: args, success: options.success, fail: options.fail, complete: options.complete };
                var promise;
                if (!options.success && !options.fail && !options.complete) {
                    promise = new Promise(function(resolve, reject) { task.resolve = resolve; task.reject = reject; });
                }
                __hostCallTasks[id] = task;
                var send = function(a) {
                    task.args = a;
                    __emitHostEvent({ type: 'hostCall', id: id, api: name, args: a });
                };
                var hook = __hostCallHooks[name];
                if (hook && hook.prepare) {
                    // 失败也走下一轮事件循环：与微信一样，回调不在调用栈里同步发生
                    var fail = function(errMsg) {
                        setTimeout(function() { __resolveHostCall(id, false, JSON.stringify({ errMsg: errMsg })); }, 0);
                    };
                    try { hook.prepare(args, send, fail); } catch (e) { console.error(e); send(args); }
                } else {
                    send(args);
                }
                return promise;
            }

            // 「要宿主才能做」的那一类：宿主点名了就交给宿主，没点名就**立刻 fail**（下一轮事件循环），
            // errMsg 写明原因。对象方法（`MapContext.getCenterLocation`、`NFCAdapter.startDiscovery`）也走这里，
            // 宿主在 hostApis 里点的就是这个带点的名字
            function __hostInvoke(name, options) {
                if (__hostApiSet[name]) return __hostCall(name, options);
                options = options || {};
                var shown = name.indexOf('.') > 0 ? name.slice(name.indexOf('.') + 1) : name;
                var bad = { errMsg: shown + ':fail 当前宿主未实现（宿主可在创建选项 hostApis 里接管 ' + name + '）' };
                if (!options.success && !options.fail && !options.complete) {
                    return new Promise(function(resolve, reject) { setTimeout(function() { reject(bad); }, 0); });
                }
                setTimeout(function() { options.fail && options.fail(bad); options.complete && options.complete(bad); }, 0);
            }

            function __installHostApis(names) {
                (names || []).forEach(function(name) {
                    if (typeof name !== 'string' || !name || name.indexOf('__') === 0) return;
                    __hostApiSet[name] = true;
                    // 带点的是对象方法，由对应的对象自己经 __hostInvoke 调，不往 wx 上挂
                    if (name.indexOf('.') >= 0) return;
                    wx[name] = function(options) { return __hostCall(name, options); };
                });
            }

            // 宿主回结果：resultJson 是对象就原样作为 res，其它值包成 { data }
            function __resolveHostCall(id, ok, resultJson) {
                var task = __hostCallTasks[id];
                if (!task) return false;
                delete __hostCallTasks[id];
                var res;
                try { res = JSON.parse(resultJson || '{}'); } catch (e) { res = { data: resultJson }; }
                if (res === null || typeof res !== 'object' || Array.isArray(res)) res = { data: res };
                if (!res.errMsg) res.errMsg = task.name + (ok ? ':ok' : ':fail');
                var hook = __hostCallHooks[task.name];
                if (ok && hook && hook.finish) {
                    try { res = hook.finish(res, task.args || {}) || res; } catch (e) { console.error(e); }
                }
                if (ok) {
                    task.success && task.success(res);
                    task.resolve && task.resolve(res);
                } else {
                    task.fail && task.fail(res);
                    task.reject && task.reject(res);
                }
                task.complete && task.complete(res);
                return true;
            }
