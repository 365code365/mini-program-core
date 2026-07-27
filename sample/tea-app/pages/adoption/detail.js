"use strict";
const common_vendor = require("../../common/vendor.js");
const api_adopt = require("../../api/adopt.js");
const api_mappers = require("../../api/mappers.js");
const common_assets = require("../../common/assets.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      g: new api_mappers.GardenVM({
        id: 0,
        name: "",
        variety: "",
        altitude: 0,
        area: "",
        treeAge: 0,
        pricePerShare: 0,
        sharesLeft: 0,
        cover: "",
        location: "",
        sales: 0,
        benefits: [],
        weatherEui: "",
        cameraEui: "",
        traceCode: ""
      }),
      shares: 1
    };
  },
  computed: {
    total() {
      return this.shares * this.g.pricePerShare;
    }
  },
  onLoad(opt) {
    const id = opt["id"];
    if (id != null) {
      api_adopt.gardenDetail(parseInt(id)).then((res) => {
        this.g = api_mappers.toGarden(res);
      }).catch((_e = null) => {
      });
    }
  },
  methods: {
    nav(url) {
      common_vendor.index.navigateTo({ url });
    },
    inc() {
      const max = Math.min(5, this.g.sharesLeft);
      if (this.shares < max)
        this.shares += 1;
    },
    dec() {
      if (this.shares > 1)
        this.shares -= 1;
    },
    toCheckout() {
      common_vendor.index.navigateTo({ url: "/pages/checkout/checkout?id=" + this.g.id + "&shares=" + this.shares });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: $data.g.cover,
    b: common_assets._imports_0$7,
    c: common_vendor.o(($event) => $options.nav("/pages/garden/garden"), "7d"),
    d: common_vendor.t($data.g.variety),
    e: common_vendor.t($data.g.location),
    f: common_vendor.t($data.g.name),
    g: common_vendor.t($data.g.altitude),
    h: common_vendor.t($data.g.treeAge),
    i: common_vendor.t($data.g.area),
    j: common_vendor.f($data.g.benefits, (b, i, i0) => {
      return {
        a: common_vendor.t(b),
        b: i
      };
    }),
    k: common_assets._imports_1$3,
    l: common_vendor.t($data.g.sharesLeft),
    m: common_vendor.o((...args) => $options.dec && $options.dec(...args), "fd"),
    n: common_vendor.t($data.shares),
    o: common_vendor.o((...args) => $options.inc && $options.inc(...args), "24"),
    p: common_vendor.t($options.total),
    q: $data.g.sharesLeft == 0
  }, $data.g.sharesLeft == 0 ? {} : {
    r: common_vendor.o((...args) => $options.toCheckout && $options.toCheckout(...args), "46")
  }, {
    s: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    t: `${_ctx.u_s_b_h}px`,
    v: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
