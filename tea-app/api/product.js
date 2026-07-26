"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
require("./models.js");
function productList(keyword, cid, page, limit) {
  const q = new common_vendor.UTSJSONObject({ page, limit });
  if (keyword.length > 0)
    q["keyword"] = keyword;
  if (cid > 0)
    q["cid"] = cid;
  return utils_request.getPublic("/products", q);
}
function productDetail(id) {
  return utils_request.getPublic("/product/detail/" + id, null);
}
function searchKeywords() {
  return utils_request.getPublic("/search/keyword", null);
}
exports.productDetail = productDetail;
exports.productList = productList;
exports.searchKeywords = searchKeywords;
