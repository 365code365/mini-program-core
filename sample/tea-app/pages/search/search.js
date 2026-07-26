"use strict";
const common_vendor = require("../../common/vendor.js");
const api_product = require("../../api/product.js");
const api_adopt = require("../../api/adopt.js");
require("../../api/models.js");
const api_mappers = require("../../api/mappers.js");
const common_assets = require("../../common/assets.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      q: "",
      searched: false,
      loading: false,
      hots: ["鸭屎香", "古树认养", "蜜兰香", "礼盒", "坪坑口", "口粮茶"],
      gardenHits: [],
      productHits: []
    };
  },
  onLoad() {
    api_product.searchKeywords().then((arr) => {
      var _a, _b;
      const ks = [];
      for (let i = 0; i < arr.length; i++) {
        const w = (_a = arr[i].getString("keyword")) !== null && _a !== void 0 ? _a : (_b = arr[i].getString("title")) !== null && _b !== void 0 ? _b : "";
        if (w.length > 0)
          ks.push(w);
      }
      if (ks.length > 0)
        this.hots = ks;
    }).catch((_e = null) => {
    });
  },
  methods: {
    tapHot(h) {
      this.q = h;
      this.doSearch();
    },
    clear() {
      this.q = "";
      this.searched = false;
      this.gardenHits = [];
      this.productHits = [];
    },
    doSearch() {
      if (this.q.length == 0)
        return null;
      this.searched = true;
      this.loading = true;
      api_product.productList(this.q, 0, 1, 20).then((res) => {
        this.productHits = api_mappers.toProductList(res.list);
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
      api_adopt.gardenList("").then((res) => {
        const all = api_mappers.toGardenList(res.list);
        const hits = [];
        for (let i = 0; i < all.length; i++) {
          const g = all[i];
          if (g.name.indexOf(this.q) >= 0 || g.location.indexOf(this.q) >= 0 || g.variety.indexOf(this.q) >= 0)
            hits.push(g);
        }
        this.gardenHits = hits;
      }).catch((_e = null) => {
      });
    },
    toDetail(id) {
      common_vendor.index.navigateTo({ url: "/pages/adoption/detail?id=" + id });
    },
    toProduct(id) {
      common_vendor.index.navigateTo({ url: "/pages/shop/detail?id=" + id });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_assets._imports_0$4,
    b: common_vendor.o((...args) => $options.doSearch && $options.doSearch(...args), "fb"),
    c: $data.q,
    d: common_vendor.o(($event) => $data.q = $event.detail.value, "6a"),
    e: $data.q.length > 0
  }, $data.q.length > 0 ? {
    f: common_assets._imports_1$3,
    g: common_vendor.o((...args) => $options.clear && $options.clear(...args), "de")
  } : {}, {
    h: !$data.searched
  }, !$data.searched ? {
    i: common_vendor.f($data.hots, (h, i, i0) => {
      return {
        a: common_vendor.t(h),
        b: i,
        c: common_vendor.o(($event) => $options.tapHot(h), i)
      };
    })
  } : common_vendor.e({
    j: $data.gardenHits.length == 0 && $data.productHits.length == 0 && !$data.loading
  }, $data.gardenHits.length == 0 && $data.productHits.length == 0 && !$data.loading ? {
    k: common_vendor.t($data.q)
  } : {}, {
    l: $data.gardenHits.length > 0
  }, $data.gardenHits.length > 0 ? {
    m: common_vendor.t($data.gardenHits.length),
    n: common_vendor.f($data.gardenHits, (g, i, i0) => {
      return {
        a: g.cover,
        b: common_vendor.t(g.name),
        c: common_vendor.t(g.pricePerShare),
        d: i,
        e: common_vendor.o(($event) => $options.toDetail(g.id), i)
      };
    })
  } : {}, {
    o: $data.productHits.length > 0
  }, $data.productHits.length > 0 ? {
    p: common_vendor.t($data.productHits.length),
    q: common_vendor.f($data.productHits, (p, i, i0) => {
      return common_vendor.e({
        a: p.cover,
        b: common_vendor.t(p.name),
        c: p.price > 0
      }, p.price > 0 ? {
        d: common_vendor.t(p.price)
      } : {}, {
        e: i,
        f: common_vendor.o(($event) => $options.toProduct(p.id), i)
      });
    })
  } : {}), {
    r: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    s: `${_ctx.u_s_b_h}px`,
    t: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
