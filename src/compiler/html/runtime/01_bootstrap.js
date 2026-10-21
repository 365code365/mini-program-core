/* mini-render 响应式运行时（导出 HTML 工程） */
(function () {
  "use strict";
  var appConfig = null, appInst = null, pageConfig = null, pageInst = null;
  var AST = window.__WXML__ || [];
  var composing = false, delegated = false;

  // ───────── 框架 API ─────────
  window.App = function (cfg) { appConfig = cfg; };
  window.Page = function (cfg) { pageConfig = cfg; };
  window.Component = function (cfg) {
    var c = {};
    if (cfg.data) c.data = cfg.data;
    var m = cfg.methods || {};
    for (var k in m) c[k] = m[k];
    if (cfg.properties) { c.data = c.data || {}; for (var p in cfg.properties) { var pd = cfg.properties[p]; c.data[p] = (pd && typeof pd === "object" && "value" in pd) ? pd.value : undefined; } }
    if (cfg.attached) c.onLoad = cfg.attached;
    if (cfg.ready) c.onReady = cfg.ready;
    pageConfig = c;
  };
  window.getApp = function () { return appInst; };
  window.getCurrentPages = function () { return pageInst ? [pageInst] : []; };
  window.Behavior = function (b) { return b; };

  // ───────── 表达式 & 插值 ─────────
  var exprCache = {};
  // 当前 WXML 文件里的 wxs 模块（模块名 → 导出对象），由 02_vnode.js 进出作用域时切换。
  // 套在数据作用域**里面**：模块名压过同名数据字段（与原生 parser::expr::wxs 一致）
  var curWxs = null, NO_WXS = {};
  function evalExpr(code, scope) {
    try {
      var fn = exprCache[code];
      if (!fn) { fn = new Function("$s", "$w", "with($s){with($w){return (" + code + ");}}"); exprCache[code] = fn; }
      return fn(scope || {}, curWxs || NO_WXS);
    } catch (e) { return undefined; }
  }
  function interp(str, scope) {
    if (str == null) return "";
    if (("" + str).indexOf("{{") < 0) return str;
    return ("" + str).replace(/\{\{([\s\S]+?)\}\}/g, function (_, e) { var v = evalExpr(e.trim(), scope); return v == null ? "" : ("" + v); });
  }
  function evalWhole(str, scope) {
    if (str == null) return undefined;
    var m = /^\s*\{\{([\s\S]+)\}\}\s*$/.exec("" + str);
    if (m) return evalExpr(m[1].trim(), scope);
    return str;
  }
  function truthy(v) { return v === true || v === "true" || v === "1" || v === 1; }
  function esc(s) { return ("" + s).replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;"); }
  function escText(s) { return ("" + s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;"); }
  function convertRpx(s) { return ("" + s).replace(/(-?[0-9.]+)rpx/g, function (_, n) { return (parseFloat(n) * 0.5) + "px"; }); }
  function camel(s) { return s.replace(/-([a-z])/g, function (_, c) { return c.toUpperCase(); }); }

