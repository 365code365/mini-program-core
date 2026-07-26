"use strict";
const common_vendor = require("../common/vendor.js");
const config_env = require("../config/env.js");
const utils_auth = require("./auth.js");
class RequestOptions extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          auth: { type: Boolean, optional: false },
          toast: { type: Boolean, optional: false }
        };
      },
      name: "RequestOptions"
    };
  }
  constructor(options, metadata = RequestOptions.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.auth = this.__props__.auth;
    this.toast = this.__props__.toast;
    delete this.__props__;
  }
}
const DEFAULT_OPT = new RequestOptions({ auth: true, toast: true });
function buildHeader(auth) {
  const header = new common_vendor.UTSJSONObject({});
  header["Content-Type"] = "application/json";
  if (auth) {
    const tk = utils_auth.getToken();
    if (tk.length > 0) {
      header[config_env.TOKEN_HEADER] = tk;
    }
  }
  return header;
}
function goLogin() {
  utils_auth.clearToken();
  common_vendor.index.navigateTo({ url: "/pages/login/login" });
}
function request(method, url, data = null, opt = DEFAULT_OPT) {
  return new Promise((resolve, reject) => {
    common_vendor.index.request({
      url: config_env.BASE_URL + url,
      method,
      data,
      header: buildHeader(opt.auth),
      timeout: config_env.TIMEOUT,
      success: (res) => {
        var _a, _b;
        const body = res.data;
        const code = (_a = body.getNumber("code")) !== null && _a !== void 0 ? _a : res.statusCode;
        if (code == 200) {
          resolve(body["data"]);
        } else if (code == 401 || code == 402) {
          if (opt.toast)
            common_vendor.index.showToast({ title: "登录已失效，请重新登录", icon: "none" });
          goLogin();
          reject(new Error("" + code));
        } else {
          const msg = (_b = body.getString("message")) !== null && _b !== void 0 ? _b : "请求失败";
          if (opt.toast)
            common_vendor.index.showToast({ title: msg, icon: "none" });
          reject(new Error(msg));
        }
      },
      fail: (_err) => {
        if (opt.toast)
          common_vendor.index.showToast({ title: "网络异常，请稍后重试", icon: "none" });
        reject(new Error("network error"));
      }
    });
  });
}
function get(url, data = null, opt = DEFAULT_OPT) {
  return request("GET", url, data, opt);
}
function post(url, data = null, opt = DEFAULT_OPT) {
  return request("POST", url, data, opt);
}
function getPublic(url, data = null) {
  return request("GET", url, data, new RequestOptions({ auth: false, toast: true }));
}
function postPublic(url, data = null) {
  return request("POST", url, data, new RequestOptions({ auth: false, toast: true }));
}
exports.RequestOptions = RequestOptions;
exports.get = get;
exports.getPublic = getPublic;
exports.post = post;
exports.postPublic = postPublic;
