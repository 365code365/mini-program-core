"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("./request.js");
const config_env = require("../config/env.js");
const STORAGE_KEY = "yunxiu_ui_assets";
let cache = null;
function loadCache() {
  var e_1, _a;
  const map = /* @__PURE__ */ new Map();
  const raw = common_vendor.index.getStorageSync(STORAGE_KEY);
  if (raw != null) {
    const obj = raw;
    try {
      for (var _b = common_vendor.__values(obj.toMap().keys()), _c = _b.next(); !_c.done; _c = _b.next()) {
        var key = _c.value;
        const value = obj.getString(key);
        if (value != null && value.length > 0) {
          map.set(key, value);
        }
      }
    } catch (e_1_1) {
      e_1 = { error: e_1_1 };
    } finally {
      try {
        if (_c && !_c.done && (_a = _b.return))
          _a.call(_b);
      } finally {
        if (e_1)
          throw e_1.error;
      }
    }
  }
  return map;
}
function remote(path) {
  if (path.startsWith("/static/gen/")) {
    return config_env.ASSET_BASE + path.substring("/static/gen".length);
  }
  return path;
}
function siteOrigin() {
  const i = config_env.ASSET_BASE.indexOf("/", "https://".length);
  return i > 0 ? config_env.ASSET_BASE.substring(0, i) : config_env.ASSET_BASE;
}
function normalize(url) {
  let s = url.trim();
  while (s.startsWith("/http")) {
    s = s.substring(1);
  }
  if (s.startsWith("//")) {
    return "https:" + s;
  }
  if (s.startsWith("http://") || s.startsWith("https://")) {
    return s;
  }
  return siteOrigin() + (s.startsWith("/") ? s : "/" + s);
}
function assetUrl(key, fallback) {
  if (cache == null) {
    cache = loadCache();
  }
  const hit = common_vendor.UTS.mapGet(cache, key);
  if (hit != null && hit.length > 0) {
    return normalize(hit);
  }
  return remote(fallback);
}
function refreshAssets() {
  utils_request.getPublic("/tea/ui/assets").then((data) => {
    common_vendor.index.setStorageSync(STORAGE_KEY, data);
    cache = null;
  }).catch((_e = null) => {
  });
}
exports.assetUrl = assetUrl;
exports.refreshAssets = refreshAssets;
