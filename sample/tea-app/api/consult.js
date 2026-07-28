"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
function submitConsult(body) {
  return utils_request.postPublic("/tea/consult/submit", body);
}
function openConsult(scene, sourcePage, productId) {
  let url = "/pages/consult/consult?scene=" + scene + "&source=" + sourcePage;
  if (productId > 0) {
    url = url + "&productId=" + productId;
  }
  common_vendor.index.navigateTo({ url });
}
exports.openConsult = openConsult;
exports.submitConsult = submitConsult;
//# sourceMappingURL=../../.sourcemap/mp-weixin/api/consult.js.map
