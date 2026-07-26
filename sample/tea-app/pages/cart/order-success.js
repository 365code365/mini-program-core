"use strict";
const common_vendor = require("../../common/vendor.js");
const common_assets = require("../../common/assets.js");
const _sfc_main = common_vendor.defineComponent({
  methods: {
    toOrders() {
      common_vendor.index.redirectTo({ url: "/pages/orders/orders" });
    },
    toShop() {
      common_vendor.index.redirectTo({ url: "/pages/shop/list" });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: common_assets._imports_0$5,
    b: common_vendor.o((...args) => $options.toOrders && $options.toOrders(...args), "94"),
    c: common_vendor.o((...args) => $options.toShop && $options.toShop(...args), "4a"),
    d: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    e: `${_ctx.u_s_b_h}px`,
    f: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
