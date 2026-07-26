"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
function weatherLatest(gardenId) {
  return utils_request.getPublic("/iot/weather/latest", new common_vendor.UTSJSONObject({ gardenId }));
}
function cameraPlay(gardenId) {
  return utils_request.getPublic("/iot/camera/play", new common_vendor.UTSJSONObject({ gardenId }));
}
function traceArchive(batchNo) {
  return utils_request.getPublic("/trace/archive/" + batchNo, null);
}
exports.cameraPlay = cameraPlay;
exports.traceArchive = traceArchive;
exports.weatherLatest = weatherLatest;
