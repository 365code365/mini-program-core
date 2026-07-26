"use strict";
const common_vendor = require("../../common/vendor.js");
const api_adopt = require("../../api/adopt.js");
require("../../api/models.js");
const api_mappers = require("../../api/mappers.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      variety: "all",
      list: [],
      loading: false
    };
  },
  onLoad(opt) {
    const v = opt["variety"];
    if (v != null)
      this.variety = v;
    this.load();
  },
  methods: {
    load() {
      this.loading = true;
      const v = this.variety == "all" ? "" : this.variety;
      api_adopt.gardenList(v).then((res) => {
        this.list = api_mappers.toGardenList(res.list);
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    setVariety(v) {
      this.variety = v;
      this.load();
    },
    open(g) {
      if (g.sharesLeft == 0)
        return null;
      common_vendor.index.navigateTo({ url: "/pages/adoption/detail?id=" + g.id });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: common_vendor.n($data.variety == "all" ? "fil-on" : ""),
    b: common_vendor.o(($event) => $options.setVariety("all"), "5f"),
    c: common_vendor.n($data.variety == "古树" ? "fil-on" : ""),
    d: common_vendor.o(($event) => $options.setVariety("古树"), "b0"),
    e: common_vendor.n($data.variety == "台地" ? "fil-on" : ""),
    f: common_vendor.o(($event) => $options.setVariety("台地"), "4e"),
    g: common_vendor.f($data.list, (g, i, i0) => {
      return common_vendor.e({
        a: g.cover,
        b: g.sharesLeft == 0
      }, g.sharesLeft == 0 ? {} : {}, {
        c: common_vendor.t(g.location),
        d: common_vendor.t(g.altitude),
        e: common_vendor.t(g.treeAge),
        f: common_vendor.t(g.area),
        g: common_vendor.t(g.pricePerShare),
        h: common_vendor.t(g.sharesLeft == 0 ? "已售罄" : "剩余 " + g.sharesLeft + " 份 · 已认养 " + g.sales),
        i,
        j: common_vendor.o(($event) => $options.open(g), i)
      });
    }),
    h: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    i: `${_ctx.u_s_b_h}px`,
    j: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
