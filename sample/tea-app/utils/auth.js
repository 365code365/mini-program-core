"use strict";
const common_vendor = require("../common/vendor.js");
const config_env = require("../config/env.js");
function getToken() {
  const t = common_vendor.index.getStorageSync(config_env.TOKEN_KEY);
  if (typeof t == "string")
    return t;
  return "";
}
function setToken(token) {
  common_vendor.index.setStorageSync(config_env.TOKEN_KEY, token);
}
function clearToken() {
  common_vendor.index.removeStorageSync(config_env.TOKEN_KEY);
  common_vendor.index.removeStorageSync(config_env.USER_KEY);
}
function isLogin() {
  return getToken().length > 0;
}
function setUser(json) {
  common_vendor.index.setStorageSync(config_env.USER_KEY, json);
}
exports.clearToken = clearToken;
exports.getToken = getToken;
exports.isLogin = isLogin;
exports.setToken = setToken;
exports.setUser = setUser;
//# sourceMappingURL=../../.sourcemap/mp-weixin/utils/auth.js.map
