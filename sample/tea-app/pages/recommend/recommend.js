"use strict";
const common_vendor = require("../../common/vendor.js");
const api_product = require("../../api/product.js");
require("../../api/models.js");
const utils_assets = require("../../utils/assets.js");
class RecommendProduct extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          desc: { type: String, optional: false },
          price: { type: String, optional: false },
          unit: { type: String, optional: false },
          image: { type: String, optional: false }
        };
      },
      name: "RecommendProduct"
    };
  }
  constructor(options, metadata = RecommendProduct.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.desc = this.__props__.desc;
    this.price = this.__props__.price;
    this.unit = this.__props__.unit;
    this.image = this.__props__.image;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      curTab: 0,
      imgBannerKing: utils_assets.assetUrl("recommend_teaking", "/static/gen/rec-teaking.jpg"),
      imgBannerTasting: utils_assets.assetUrl("recommend_tasting", "/static/gen/rec-tasting.jpg"),
      tabs: ["本周推荐", "茶王系列", "品鉴装"],
      tabIcons: ["/static/icons/star-m.png", "/static/icons/trophy-m.png", "/static/icons/leaf-m.png"],
      tabIconsOn: ["/static/icons/star-w.png", "/static/icons/trophy-w.png", "/static/icons/leaf-w.png"],
      teaKings: [],
      tastings: [],
      kingLoaded: false,
      tastingLoaded: false
    };
  },
  onLoad() {
    this.imgBannerKing = utils_assets.assetUrl("recommend_teaking", "/static/gen/rec-teaking.jpg");
    this.imgBannerTasting = utils_assets.assetUrl("recommend_tasting", "/static/gen/rec-tasting.jpg");
    this.loadProducts("茶王", true);
    this.loadProducts("品鉴", false);
  },
  methods: {
    fallback(kings, i) {
      const kingImgs = [
        utils_assets.assetUrl("recommend_prod_teaking", "/static/gen/prod-teaking.jpg"),
        utils_assets.assetUrl("ai_prod_2", "/static/gen/ai-prod2.jpg"),
        utils_assets.assetUrl("ai_prod_3", "/static/gen/ai-prod3.jpg")
      ];
      const tastingImgs = [
        utils_assets.assetUrl("recommend_prod_tasting", "/static/gen/prod-tasting.jpg"),
        utils_assets.assetUrl("ai_prod_1", "/static/gen/ai-prod1.jpg"),
        utils_assets.assetUrl("recommend_prod_tasting_alt", "/static/gen/detail-2.jpg")
      ];
      return kings ? kingImgs[i % 3] : tastingImgs[i % 3];
    },
    loadProducts(keyword, kings) {
      api_product.productList(keyword, 0, 1, 3).then((page) => {
        var _a, _b, _c, _d, _f;
        const result = [];
        const size = page.list.length > 3 ? 3 : page.list.length;
        for (let i = 0; i < size; i++) {
          const item = page.list[i];
          const img = (_a = item.getString("image")) !== null && _a !== void 0 ? _a : "";
          result.push(new RecommendProduct({
            id: (_b = item.getNumber("id")) !== null && _b !== void 0 ? _b : 0,
            name: (_c = item.getString("storeName")) !== null && _c !== void 0 ? _c : "凤凰单枞",
            desc: kings ? "核心产区 · 匠心之作" : "多款好茶 · 随心品鉴",
            price: "" + ((_d = item.getNumber("price")) !== null && _d !== void 0 ? _d : 0),
            unit: (_f = item.getString("unitName")) !== null && _f !== void 0 ? _f : "",
            image: img.length > 0 ? img : this.fallback(kings, i)
          }));
        }
        if (kings) {
          this.teaKings = result;
          this.kingLoaded = true;
        } else {
          this.tastings = result;
          this.tastingLoaded = true;
        }
      }).catch((_e = null) => {
        if (kings)
          this.kingLoaded = true;
        else
          this.tastingLoaded = true;
      });
    },
    switchTab(i) {
      this.curTab = i;
    },
    open(id) {
      if (id > 0)
        common_vendor.index.navigateTo({ url: "/pages/shop/detail?id=" + id });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_vendor.f($data.tabs, (t, i, i0) => {
      return {
        a: $data.curTab == i ? $data.tabIconsOn[i] : $data.tabIcons[i],
        b: common_vendor.t(t),
        c: common_vendor.n($data.curTab == i ? "tab-text-on" : ""),
        d: common_vendor.n($data.curTab == i ? "tab-on" : ""),
        e: i,
        f: common_vendor.o(($event) => $options.switchTab(i), i)
      };
    }),
    b: $data.curTab != 2
  }, $data.curTab != 2 ? common_vendor.e({
    c: $data.imgBannerKing,
    d: $data.kingLoaded && $data.teaKings.length == 0
  }, $data.kingLoaded && $data.teaKings.length == 0 ? {} : {
    e: common_vendor.f($data.teaKings, (item, i, i0) => {
      return common_vendor.e({
        a: item.image,
        b: common_vendor.t(item.name),
        c: common_vendor.t(item.desc),
        d: common_vendor.t(item.price),
        e: item.unit.length > 0
      }, item.unit.length > 0 ? {
        f: common_vendor.t(item.unit)
      } : {}, {
        g: i,
        h: common_vendor.o(($event) => $options.open(item.id), i)
      });
    })
  }) : {}, {
    f: $data.curTab != 1
  }, $data.curTab != 1 ? common_vendor.e({
    g: $data.imgBannerTasting,
    h: $data.tastingLoaded && $data.tastings.length == 0
  }, $data.tastingLoaded && $data.tastings.length == 0 ? {} : {
    i: common_vendor.f($data.tastings, (item, i, i0) => {
      return common_vendor.e({
        a: item.image,
        b: common_vendor.t(item.name),
        c: common_vendor.t(item.desc),
        d: common_vendor.t(item.price),
        e: item.unit.length > 0
      }, item.unit.length > 0 ? {
        f: common_vendor.t(item.unit)
      } : {}, {
        g: i,
        h: common_vendor.o(($event) => $options.open(item.id), i)
      });
    })
  }) : {}, {
    j: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    k: `${_ctx.u_s_b_h}px`,
    l: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/recommend/recommend.js.map
