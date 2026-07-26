"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
const utils_auth = require("../utils/auth.js");
function loginAccount(account, password) {
  return utils_request.post("/login", new common_vendor.UTSJSONObject({ account, password }), { auth: false, toast: true }).then((data) => {
    const token = data.getString("token");
    if (token != null)
      utils_auth.setToken(token);
    utils_auth.setUser(common_vendor.UTS.JSON.stringify(data));
    return data;
  });
}
function sendCode(phone) {
  return utils_request.post("/sendCode?phone=" + phone, null, { auth: false, toast: true });
}
function loginMobile(phone, captcha) {
  return utils_request.post("/login/mobile", new common_vendor.UTSJSONObject({ phone, captcha }), { auth: false, toast: true }).then((data) => {
    const token = data.getString("token");
    if (token != null)
      utils_auth.setToken(token);
    utils_auth.setUser(common_vendor.UTS.JSON.stringify(data));
    return data;
  });
}
function programLogin(code, info) {
  return utils_request.post("/wechat/authorize/program/login?code=" + code, info, { auth: false, toast: true }).then((data) => {
    const type = data.getString("type");
    const token = data.getString("token");
    if (type == "login" && token != null) {
      utils_auth.setToken(token);
      utils_auth.setUser(common_vendor.UTS.JSON.stringify(data));
    }
    return data;
  });
}
function bindWxPhone(body) {
  return utils_request.post("/wechat/register/binding/phone", body, { auth: false, toast: true }).then((data) => {
    const token = data.getString("token");
    if (token != null)
      utils_auth.setToken(token);
    utils_auth.setUser(common_vendor.UTS.JSON.stringify(data));
    return data;
  });
}
function logout() {
  return utils_request.get("/logout", null).then((d) => {
    utils_auth.clearToken();
    return d;
  });
}
exports.bindWxPhone = bindWxPhone;
exports.loginAccount = loginAccount;
exports.loginMobile = loginMobile;
exports.logout = logout;
exports.programLogin = programLogin;
exports.sendCode = sendCode;
