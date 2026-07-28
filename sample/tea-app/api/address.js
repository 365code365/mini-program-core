"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
function addressList() {
  return new Promise((resolve, reject) => {
    utils_request.get("/address/list", new common_vendor.UTSJSONObject({ page: 1, limit: 50 })).then((d) => {
      const arr = d["list"];
      if (arr != null && Array.isArray(arr)) {
        resolve(arr);
      } else {
        resolve([]);
      }
    }).catch((e = null) => {
      reject(e);
    });
  });
}
function addressDefault() {
  return utils_request.get("/address/default", null);
}
function addressEdit(form) {
  return utils_request.post("/address/edit", form);
}
function addressDel(id) {
  return utils_request.post("/address/del", new common_vendor.UTSJSONObject({ id }));
}
function addressSetDefault(id) {
  return utils_request.post("/address/default/set", new common_vendor.UTSJSONObject({ id }));
}
function cityList() {
  return utils_request.get("/city/list", null);
}
exports.addressDefault = addressDefault;
exports.addressDel = addressDel;
exports.addressEdit = addressEdit;
exports.addressList = addressList;
exports.addressSetDefault = addressSetDefault;
exports.cityList = cityList;
//# sourceMappingURL=../../.sourcemap/mp-weixin/api/address.js.map
