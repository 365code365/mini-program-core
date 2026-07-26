"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
require("./models.js");
function gardenList(variety) {
  const q = new common_vendor.UTSJSONObject({ page: 1, limit: 50 });
  if (variety.length > 0)
    q["variety"] = variety;
  return utils_request.getPublic("/adopt/garden/list", q);
}
function gardenDetail(id) {
  return utils_request.getPublic("/adopt/garden/detail/" + id, null);
}
function createAdopt(gardenId, shareNum) {
  return utils_request.post("/adopt/create", new common_vendor.UTSJSONObject({ gardenId, shareNum }));
}
function payAdopt(orderNo, payType, payChannel, balanceAmount) {
  return utils_request.post("/adopt/pay", new common_vendor.UTSJSONObject({ orderNo, payType, payChannel, balanceAmount }));
}
function submitPlate(recordId, nickname, message) {
  return utils_request.post("/adopt/plate/submit", new common_vendor.UTSJSONObject({ recordId, nickname, message }));
}
function plateInfo(recordId) {
  return utils_request.get("/adopt/plate/info/" + recordId, null);
}
function verifyCert(orderNo) {
  return utils_request.get("/adopt/cert/verify/" + orderNo, null);
}
function myList(page, limit) {
  return utils_request.get("/adopt/my/list", new common_vendor.UTSJSONObject({ page, limit }));
}
exports.createAdopt = createAdopt;
exports.gardenDetail = gardenDetail;
exports.gardenList = gardenList;
exports.myList = myList;
exports.payAdopt = payAdopt;
exports.plateInfo = plateInfo;
exports.submitPlate = submitPlate;
exports.verifyCert = verifyCert;
