            var __console_buffer = [];
            // Error 参数要连栈一起打。框架（Vue 等）的错误处理器普遍是
            // `console.error(err)`，而 `String(err)` 只有 "TypeError: not a function" ——
            // 没有名字也没有位置，等于逼人去通读几十万行 vendor 包。
            function __fmtArg(a) {
                if (a instanceof Error) {
                    var s = (a.name || 'Error') + ': ' + (a.message || '');
                    if (a.stack) { s += '\n' + a.stack; }
                    return s;
                }
                if (a && typeof a === 'object') {
                    try { return JSON.stringify(a); } catch (e) { return String(a); }
                }
                return String(a);
            }
            function __fmtArgs(args) {
                return Array.prototype.map.call(args, __fmtArg).join(' ');
            }
            var console = {
                log: function() {
                    var msg = __fmtArgs(arguments);
                    __console_buffer.push('[LOG] ' + msg);
                    if (typeof __native_print === 'function') { __native_print(msg); }
                },
                error: function() {
                    var msg = __fmtArgs(arguments);
                    __console_buffer.push('[ERROR] ' + msg);
                    if (typeof __native_print === 'function') { __native_print('[ERROR] ' + msg); }
                },
                warn: function() {
                    var msg = Array.prototype.slice.call(arguments).join(' ');
                    __console_buffer.push('[WARN] ' + msg);
                    if (typeof __native_print === 'function') { __native_print('[WARN] ' + msg); }
                },
                info: function() {
                    var msg = __fmtArgs(arguments);
                    __console_buffer.push('[INFO] ' + msg);
                    if (typeof __native_print === 'function') { __native_print(msg); }
                }
            };
            // `console` 必须把常见方法**全部**给齐，缺一个就可能让整个小程序起不来：
            // uni-app 的运行时开头就有
            //   ["log","warn","error","info","debug"].reduce((m,t)=>{ m[t]=console[t].bind(console) })
            // 少了 `debug` 时这句直接 `TypeError: cannot read property 'bind' of undefined`，
            // app 脚本执行中断、一个页面都加载不出来（tea-app 换新版编译产物后就是这样，
            // 47 个模块注册完就崩，报错停在 vendor.js 的 CONSOLE_TYPES.reduce 上）。
            //
            // `debug` 归到 [LOG]：微信开发者工具里 debug 也是普通日志级别。
            // 其余几个按 Console API 的语义补齐，行为保守（不真的开销）：
            // group/groupEnd/table/dir/trace/assert/count/time/timeEnd。
            console.debug = console.log;
            console.dir = console.log;
            console.trace = console.log;
            console.table = console.log;
            console.group = console.log;
            console.groupCollapsed = console.log;
            console.groupEnd = function() {};
            console.assert = function(cond) {
                if (!cond) {
                    console.error('Assertion failed: ' + __fmtArgs(Array.prototype.slice.call(arguments, 1)));
                }
            };
            var __console_counts = {};
            console.count = function(label) {
                label = label === undefined ? 'default' : String(label);
                __console_counts[label] = (__console_counts[label] || 0) + 1;
                console.log(label + ': ' + __console_counts[label]);
            };
            var __console_timers = {};
            console.time = function(label) {
                __console_timers[label === undefined ? 'default' : String(label)] = Date.now();
            };
            console.timeEnd = function(label) {
                label = label === undefined ? 'default' : String(label);
                var t0 = __console_timers[label];
                if (t0 !== undefined) {
                    console.log(label + ': ' + (Date.now() - t0) + 'ms');
                    delete __console_timers[label];
                }
            };
