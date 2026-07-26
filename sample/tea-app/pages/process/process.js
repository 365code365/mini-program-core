"use strict";
const common_vendor = require("../../common/vendor.js");
const store_data = require("../../store/data.js");
const common_assets = require("../../common/assets.js");
const _sfc_main = common_vendor.defineComponent({ data() {
  return { steps: store_data.craftSteps };
} });
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: common_assets._imports_0$11,
    b: common_vendor.f($data.steps, (s, i, i0) => {
      return {
        a: common_vendor.t(s.date),
        b: common_vendor.t(s.title),
        c: common_vendor.t(s.desc),
        d: i
      };
    }),
    c: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    d: `${_ctx.u_s_b_h}px`,
    e: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
