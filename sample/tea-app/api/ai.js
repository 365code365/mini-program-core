"use strict";
const utils_request = require("../utils/request.js");
function recommendTea(body) {
  return utils_request.postPublic("/ai/tea/recommend", body);
}
exports.recommendTea = recommendTea;
//# sourceMappingURL=../../.sourcemap/mp-weixin/api/ai.js.map
