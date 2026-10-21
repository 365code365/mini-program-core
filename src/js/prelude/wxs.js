            // ==================== wxs 模块 ====================
            // 模板表达式 `{{m.f(x)}}` 由渲染层（Rust 的表达式引擎）发起，经 `parser::expr::wxs` 的宿主
            // 调到这里。模块代码在 WXML 加载期就打好了包（`{entry, files:{路径: 代码}}`，见
            // `parser::wxml_file::wxs_bundle`），第一次用到时由宿主交给 `__wxsDefine`。
            //
            // 语义按官方 wxs/01wxs-module.html：
            //  · 每个 .wxs 文件是一个模块，`module.exports` 对外；`require` 只能相对引 .wxs；
            //  · 模块只在第一次被 require 时执行一次，之后拿缓存（单例）；
            //  · wxs 没有 `new Date()` / 正则字面量，用 `getDate()` / `getRegExp()`。
            // 模块之间不共享作用域（每个文件包在自己的函数里），也碰不到页面实例。

            var __wxsDefs = {};

            function __wxsResolve(from, rel) {
                var base = String(from).split('#')[0].split('/');
                base.pop();
                var parts = String(rel).split('/');
                for (var i = 0; i < parts.length; i++) {
                    var p = parts[i];
                    if (p === '' || p === '.') { continue; }
                    if (p === '..') { if (base.length > 1) { base.pop(); } continue; }
                    base.push(p);
                }
                var out = base.join('/');
                return out.charAt(0) === '/' ? out : '/' + out;
            }

            function __wxsGetDate() {
                var a = arguments;
                if (a.length === 0) { return new Date(); }
                if (a.length === 1) { return new Date(a[0]); }
                var d = new Date(a[0], a[1], a.length > 2 ? a[2] : 1, a[3] || 0, a[4] || 0, a[5] || 0, a[6] || 0);
                return d;
            }
            function __wxsGetRegExp(pattern, flags) { return new RegExp(pattern, flags || ''); }

            /// 定义一个模块包（重复定义同一个 id 无害：id 是代码的哈希）
            function __wxsDefine(id, bundle) {
                if (__wxsDefs[id]) { return true; }
                var files = (bundle && bundle.files) || {};
                var cache = {};
                function load(path) {
                    if (cache.hasOwnProperty(path)) { return cache[path].exports; }
                    var code = files[path];
                    if (code === undefined) { throw new Error('找不到 wxs 模块 ' + path); }
                    var mod = { exports: {} };
                    cache[path] = mod;
                    var fn = new Function('module', 'exports', 'require', 'getDate', 'getRegExp', code);
                    fn(mod, mod.exports, function (rel) { return load(__wxsResolve(path, rel)); }, __wxsGetDate, __wxsGetRegExp);
                    return mod.exports;
                }
                var exp;
                try { exp = load(bundle.entry); }
                catch (e) { __native_print('[wxs] ' + bundle.entry + ': ' + e.message); exp = {}; }
                __wxsDefs[id] = { exports: exp === undefined || exp === null ? {} : exp };
                return true;
            }

            /// 结果以 JSON 交回（undefined、抛错都给 null）
            function __wxsCall(id, path, args) {
                var d = __wxsDefs[id];
                if (!d) { return 'null'; }
                var owner = d.exports, fn = d.exports;
                for (var i = 0; i < path.length; i++) {
                    if (fn === null || fn === undefined) { return 'null'; }
                    owner = fn;
                    fn = fn[path[i]];
                }
                if (typeof fn !== 'function') { return 'null'; }
                var r;
                try { r = fn.apply(owner, args || []); }
                catch (e) { __native_print('[wxs] ' + path.join('.') + ': ' + e.message); return 'null'; }
                if (r === undefined) { return 'null'; }
                try { return JSON.stringify(r); } catch (e2) { return 'null'; }
            }

            /// 导出对象里的非函数部分（`{{m.PI}}`）。函数在 JSON 里自然被丢掉
            function __wxsExports(id) {
                var d = __wxsDefs[id];
                if (!d) { return 'null'; }
                try { return JSON.stringify(d.exports); } catch (e) { return '{}'; }
            }
