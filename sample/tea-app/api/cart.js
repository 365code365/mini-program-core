"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
require("./models.js");
function cartList(page, limit) {
  return utils_request.get("/cart/list", new common_vendor.UTSJSONObject({ page, limit }));
}
function cartSave(productId, num, sku) {
  return utils_request.post("/cart/save", new common_vendor.UTSJSONObject({ productId, cartNum: num, productAttrUnique: sku }));
}
function cartNum(id, num) {
  return utils_request.post("/cart/num?id=" + id + "&number=" + num, null);
}
function cartDelete(ids) {
  const idStr = ids.join(",");
  return utils_request.post("/cart/delete?ids=" + idStr, null);
}
exports.cartDelete = cartDelete;
exports.cartList = cartList;
exports.cartNum = cartNum;
exports.cartSave = cartSave;
//# sourceMappingURL=../../.sourcemap/mp-weixin/api/cart.js.map
