"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
require("./models.js");
function userInfo() {
  return utils_request.get("/user", null);
}
function userBalance() {
  return utils_request.get("/user/balance", null);
}
function workKefu() {
  return utils_request.get("/wechat/work/kefu", null);
}
function integralUser() {
  return utils_request.get("/integral/user", null);
}
function rechargeIndex() {
  return utils_request.get("/recharge/index", null);
}
function rechargeRoutine(price, rechargeId) {
  return utils_request.post("/recharge/routine", new common_vendor.UTSJSONObject({ price, rechar_id: rechargeId, from: "routine" }));
}
exports.integralUser = integralUser;
exports.rechargeIndex = rechargeIndex;
exports.rechargeRoutine = rechargeRoutine;
exports.userBalance = userBalance;
exports.userInfo = userInfo;
exports.workKefu = workKefu;
