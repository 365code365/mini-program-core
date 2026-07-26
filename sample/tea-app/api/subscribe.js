"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
require("./models.js");
function planList() {
  return utils_request.getPublic("/subscribe/plan/list", null);
}
function createSubscribe(planId, addressId, payType, payChannel, useBalance, couponId, remark) {
  return utils_request.post("/subscribe/create", new common_vendor.UTSJSONObject({
    planId,
    addressId,
    payType,
    payChannel,
    useBalance,
    couponId,
    remark
  }));
}
function myList(status, page, limit) {
  return utils_request.get("/subscribe/my/list", new common_vendor.UTSJSONObject({ status, page, limit }));
}
function cancel(id) {
  return utils_request.post("/subscribe/cancel", new common_vendor.UTSJSONObject({ id }));
}
exports.cancel = cancel;
exports.createSubscribe = createSubscribe;
exports.myList = myList;
exports.planList = planList;
