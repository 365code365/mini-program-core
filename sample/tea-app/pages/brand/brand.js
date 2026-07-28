"use strict";
const common_vendor = require("../../common/vendor.js");
const common_assets = require("../../common/assets.js");
const _sfc_main = common_vendor.defineComponent({ methods: { nav(url) {
  common_vendor.index.navigateTo({ url });
}, goBack() {
  common_vendor.index.navigateBack();
} } });
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: common_vendor.o((...args) => $options.goBack && $options.goBack(...args), "d0"),
    b: common_assets._imports_0$14,
    c: common_assets._imports_1$2,
    d: common_assets._imports_2$1,
    e: common_assets._imports_0$2,
    f: common_vendor.o(($event) => $options.nav("/pages/adoption/list"), "b5"),
    g: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    h: `${_ctx.u_s_b_h}px`,
    i: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/brand/brand.js.map
