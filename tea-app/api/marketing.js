"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
require("./models.js");
function signToday() {
  return utils_request.get("/user/sign/get", null);
}
function doSign() {
  return utils_request.get("/user/sign/integral", null);
}
function myCoupons(type, page, limit) {
  return utils_request.get("/coupon/list", new common_vendor.UTSJSONObject({ type, page, limit }));
}
function voucherList(page, limit) {
  return utils_request.get("/tasting/voucher/list", new common_vendor.UTSJSONObject({ page, limit }));
}
function shareCode() {
  return utils_request.get("/tasting/share/code", null);
}
function shareReceive(shareCode2) {
  return utils_request.get("/tasting/share/receive", new common_vendor.UTSJSONObject({ shareCode: shareCode2 }));
}
function voucherUse(voucherId) {
  return utils_request.get("/tasting/voucher/use?voucherId=" + voucherId, null);
}
function shareRecords(page, limit) {
  return utils_request.get("/tasting/share/list", new common_vendor.UTSJSONObject({ page, limit }));
}
exports.doSign = doSign;
exports.myCoupons = myCoupons;
exports.shareCode = shareCode;
exports.shareReceive = shareReceive;
exports.shareRecords = shareRecords;
exports.signToday = signToday;
exports.voucherList = voucherList;
exports.voucherUse = voucherUse;
