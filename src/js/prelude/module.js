            // 打包器/框架运行时（uni-app 的 vendor、Vue、各类 polyfill）普遍会摸
            // `global` / `self` / `window` 之类的全局别名来做环境探测。
            // 小程序环境里 `globalThis` 是有的，把常见别名指过去即可 ——
            // 缺了 `global` 时 Vue 运行时会在第一次特征检测就 ReferenceError。
            if (typeof globalThis !== 'undefined') {
                if (typeof global === 'undefined') { globalThis.global = globalThis; }
                if (typeof self === 'undefined') { globalThis.self = globalThis; }
            }

            var __modules = {};       // key -> factory(module, exports, require)
            var __moduleCache = {};   // key -> module 实例
            
            function __normalizePath(path) {
                var parts = String(path).split('/');
                var stack = [];
                for (var i = 0; i < parts.length; i++) {
                    var p = parts[i];
                    if (p === '' || p === '.') continue;
                    if (p === '..') { stack.pop(); continue; }
                    stack.push(p);
                }
                return stack.join('/');
            }
            
            function __stripJs(key) {
                if (key.length > 3 && key.substring(key.length - 3) === '.js') {
                    return key.substring(0, key.length - 3);
                }
                return key;
            }
            
            function __dirname(path) {
                var idx = path.lastIndexOf('/');
                return idx >= 0 ? path.substring(0, idx) : '';
            }
            
            function __resolveModulePath(currentDir, request) {
                var full;
                if (request.charAt(0) === '/') {
                    full = request.substring(1);
                } else {
                    full = (currentDir ? currentDir + '/' : '') + request;
                }
                return __stripJs(__normalizePath(full));
            }
            
            function __defineModule(path, factory) {
                __modules[__stripJs(__normalizePath(path))] = factory;
            }
            
            function __require(currentDir, request) {
                var key = __resolveModulePath(currentDir, request);
                if (__moduleCache[key]) { return __moduleCache[key].exports; }
                var factory = __modules[key];
                if (!factory) {
                    throw new Error('Cannot find module: ' + request + ' (resolved: ' + key + ')');
                }
                var module = { exports: {} };
                __moduleCache[key] = module;
                var dir = __dirname(key);
                var localRequire = function(req) { return __require(dir, req); };
                factory.call(module.exports, module, module.exports, localRequire);
                return module.exports;
            }
            
            // 全局 require（相对项目根目录解析）
            function require(request) { return __require('', request); }

            // 以「模块作用域」立即执行一段源码（app.js / 页面 js 走这条）。
            //
            // 小程序里每个 .js 文件都是 CommonJS 模块，`module` / `exports` / `require`
            // 天然在作用域里。从前 app.js 是当**裸脚本**求值的，于是 TS/uni-app 这类
            // 编译产物一上来就 `Object.defineProperty(exports, ...)`，直接
            // ReferenceError: 'exports' is not defined。
            //
            // 与 __require 的区别是**不缓存**：页面 js 每次进入都要重新执行，
            // 这样 `Page({...})` 才会重新注册（返回上一页再进来不能是空白页）。
            function __runAsModule(path, factory) {
                var key = __stripJs(__normalizePath(path));
                var module = { exports: {} };
                var dir = __dirname(key);
                var localRequire = function(req) { return __require(dir, req); };
                factory.call(module.exports, module, module.exports, localRequire);
                return module.exports;
            }
