"use strict";
const common_vendor = require("../common/vendor.js");
class PickedAddress extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          phone: { type: String, optional: false },
          region: { type: String, optional: false },
          detail: { type: String, optional: false }
        };
      },
      name: "PickedAddress"
    };
  }
  constructor(options, metadata = PickedAddress.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.phone = this.__props__.phone;
    this.region = this.__props__.region;
    this.detail = this.__props__.detail;
    delete this.__props__;
  }
}
let picked = null;
function setPickedAddress(a) {
  picked = a;
}
function takePickedAddress() {
  const p = picked;
  picked = null;
  return p;
}
exports.PickedAddress = PickedAddress;
exports.setPickedAddress = setPickedAddress;
exports.takePickedAddress = takePickedAddress;
//# sourceMappingURL=../../.sourcemap/mp-weixin/store/selection.js.map
