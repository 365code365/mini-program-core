            // wx 的中立命名空间：nb / gb / mb 与 wx 是同一个对象，另按函数名认出 `xx.函数名` 的 xx
            // （为什么、怎么认，见 src/js/api_alias.rs 的文件头）。
            // 三处共用这一份：QuickJS 前置代码（api-probe 之后，指向探针包好的 wx）、小游戏高性能模式的页面、H5 导出的运行时。
            // 所以只用 ES5、不调任何原生入口。
            (function (g) {
                if (typeof wx !== 'object' || wx === null) return;
                // 复核候选用没包探针的那份：摸一个不存在的名字不该刷「未实现的 API」
                var impl = typeof __wxImpl === 'object' && __wxImpl ? __wxImpl : wx;
                var NAME = /^[A-Za-z_$][\w$]*$/;
                // 别的平台的命名空间：按函数名认的时候绝不占（框架靠 typeof my / tt … 探测平台）
                var PLATFORM = ['my', 'tt', 'qq', 'swan', 'ks', 'jd', 'dd', 'xhs', 'qh', 'has', 'uni', 'Taro', 'plus'];
                var MIN_HITS = 2;
                var installed = { wx: true };
                // 装一个别名。已有同名全局（开发者自己的变量、别的 SDK）一律不覆盖，返回 false
                function alias(n) {
                    if (typeof n !== 'string' || !NAME.test(n)) return false;
                    if (installed[n] === true || g[n] === wx) { installed[n] = true; return true; }
                    if (typeof g[n] !== 'undefined') return false;
                    g[n] = wx;
                    installed[n] = true;
                    return true;
                }
                // canIUse('nb.request') 这类以别名开头的写法要认
                function isAlias(n) { return installed[n] === true; }
                // cands = { 前缀: [成员名…] }（Rust 扫描出来的，见 api_alias.rs）
                function guess(cands) {
                    if (!cands || typeof cands !== 'object') return;
                    for (var n in cands) {
                        if (!Object.prototype.hasOwnProperty.call(cands, n)) continue;
                        if (installed[n] === true || PLATFORM.indexOf(n) >= 0 || typeof g[n] !== 'undefined') continue;
                        var ms = cands[n] || [], hit = [];
                        for (var i = 0; i < ms.length; i++) {
                            var m = ms[i];
                            if (typeof m === 'string' && m.indexOf('__') !== 0 && typeof impl[m] === 'function' && hit.indexOf(m) < 0) hit.push(m);
                        }
                        if (hit.length >= MIN_HITS && alias(n) && typeof console !== 'undefined' && console.log) {
                            console.log('[API 别名] ' + n + '.* → wx.*（按函数名认出：' + hit.slice(0, 4).join(', ') + (hit.length > 4 ? ' …' : '') + '）');
                        }
                    }
                }
                g.__wxAlias = alias;
                g.__wxIsAlias = isAlias;
                g.__wxAliasGuess = guess;
                ['nb', 'gb', 'mb'].forEach(alias);
            })(typeof globalThis !== 'undefined' ? globalThis : this);
