"use strict";
const common_vendor = require("../common/vendor.js");
const utils_request = require("../utils/request.js");
function agreementTemplate(type) {
  return utils_request.getPublic("/agreement/template/" + type, null);
}
function signAgreement(templateId, bizType, bizId) {
  return utils_request.post("/agreement/sign", new common_vendor.UTSJSONObject({ templateId, bizType, bizId }), new utils_request.RequestOptions({ auth: true, toast: false }));
}
exports.agreementTemplate = agreementTemplate;
exports.signAgreement = signAgreement;
