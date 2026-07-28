"use strict";
const common_vendor = require("../../common/vendor.js");
const api_marketing = require("../../api/marketing.js");
const utils_auth = require("../../utils/auth.js");
const common_assets = require("../../common/assets.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return { signed: 0, today: false };
  },
  onLoad() {
    if (!utils_auth.isLogin()) {
      common_vendor.index.redirectTo({ url: "/pages/login/login?redirect=/pages/checkin/checkin" });
      return null;
    }
    this.load();
  },
  methods: {
    load() {
      api_marketing.signToday().then((d) => {
        var _a, _b, _c;
        this.signed = (_a = d.getNumber("sumSignDay")) !== null && _a !== void 0 ? _a : (_b = d.getNumber("signNum")) !== null && _b !== void 0 ? _b : 0;
        this.today = (_c = d.getBoolean("isSign")) !== null && _c !== void 0 ? _c : d.getNumber("isSign") == 1;
      }).catch((_e = null) => {
      });
    },
    sign() {
      if (this.today)
        return null;
      api_marketing.doSign().then((_d) => {
        this.signed = Math.min(7, this.signed + 1);
        this.today = true;
        common_vendor.index.showToast({ title: "签到成功", icon: "none" });
      }).catch((_e = null) => {
      });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: common_vendor.t($data.signed),
    b: common_vendor.f(7, (d, i, i0) => {
      return common_vendor.e({
        a: i < $data.signed
      }, i < $data.signed ? {
        b: common_assets._imports_0$6
      } : {
        c: common_vendor.t(i == 6 ? "+50" : "+10")
      }, {
        d: common_vendor.n(i < $data.signed ? "day-ic-on" : ""),
        e: common_vendor.t(i + 1),
        f: i,
        g: common_vendor.n(i < $data.signed ? "day-on" : "")
      });
    }),
    c: common_vendor.t($data.today ? "今日已签到" : "立即签到 +10 积分"),
    d: common_vendor.n($data.today ? "btn-off" : ""),
    e: common_vendor.o((...args) => $options.sign && $options.sign(...args), "6b"),
    f: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    g: `${_ctx.u_s_b_h}px`,
    h: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/checkin/checkin.js.map
