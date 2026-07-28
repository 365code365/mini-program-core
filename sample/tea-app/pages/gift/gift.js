"use strict";
const common_vendor = require("../../common/vendor.js");
const utils_assets = require("../../utils/assets.js");
const api_consult = require("../../api/consult.js");
const common_assets = require("../../common/assets.js");
class GiftType extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          name: { type: String, optional: false },
          desc: { type: String, optional: false },
          keyword: { type: String, optional: false },
          icon: { type: String, optional: false }
        };
      },
      name: "GiftType"
    };
  }
  constructor(options, metadata = GiftType.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.name = this.__props__.name;
    this.desc = this.__props__.desc;
    this.keyword = this.__props__.keyword;
    this.icon = this.__props__.icon;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      statusBarH: 20,
      imgHero: utils_assets.assetUrl("gift_hero", "/static/gen/gift-hero.jpg"),
      imgKing: utils_assets.assetUrl("gift_teaking", "/static/gen/gift-teaking.jpg"),
      imgTasting: utils_assets.assetUrl("gift_tasting", "/static/gen/gift-tasting.jpg"),
      types: [
        new GiftType({ name: "个人送礼", desc: "心意传情\n暖心相伴", keyword: "品鉴礼盒", icon: "/static/icons/user-d.png" }),
        new GiftType({ name: "商务送礼", desc: "得体大方\n合作更顺", keyword: "商务礼盒", icon: "/static/icons/gift-d.png" }),
        new GiftType({ name: "企业茶礼", desc: "品牌传递\n礼遇客户", keyword: "企业茶礼", icon: "/static/icons/shield-d.png" })
      ]
    };
  },
  onLoad() {
    this.statusBarH = common_vendor.index.getWindowInfo().statusBarHeight;
    this.imgHero = utils_assets.assetUrl("gift_hero", "/static/gen/gift-hero.jpg");
    this.imgKing = utils_assets.assetUrl("gift_teaking", "/static/gen/gift-teaking.jpg");
    this.imgTasting = utils_assets.assetUrl("gift_tasting", "/static/gen/gift-tasting.jpg");
  },
  methods: {
    openGift(keyword) {
      common_vendor.index.navigateTo({ url: "/pages/shop/list?keyword=" + encodeURIComponent(keyword) });
    },
    consultGift() {
      api_consult.openConsult("gift", "gift", 0);
    },
    consultEnterprise() {
      api_consult.openConsult("enterprise", "gift", 0);
    },
    reserve() {
      api_consult.openConsult("appointment", "gift", 0);
    }
  }
});
if (!Array) {
  const _easycom_tab_bar2 = common_vendor.resolveComponent("tab-bar");
  _easycom_tab_bar2();
}
const _easycom_tab_bar = () => "../../components/tab-bar/tab-bar.js";
if (!Math) {
  _easycom_tab_bar();
}
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: $data.imgHero,
    b: common_vendor.s("padding-top:" + ($data.statusBarH + 40) + "px"),
    c: common_vendor.f($data.types, (item, i, i0) => {
      return {
        a: item.icon,
        b: common_vendor.t(item.name),
        c: common_vendor.t(item.desc),
        d: i,
        e: common_vendor.o(($event) => $options.openGift(item.keyword), i)
      };
    }),
    d: common_assets._imports_5,
    e: common_vendor.o((...args) => $options.consultGift && $options.consultGift(...args), "b7"),
    f: common_vendor.o(($event) => $options.openGift("礼盒"), "ae"),
    g: $data.imgKing,
    h: common_vendor.o(($event) => $options.openGift("茶王"), "2c"),
    i: $data.imgTasting,
    j: common_vendor.o(($event) => $options.openGift("品鉴"), "4f"),
    k: common_assets._imports_5$1,
    l: common_assets._imports_2$2,
    m: common_assets._imports_2$1,
    n: common_vendor.o((...args) => $options.consultEnterprise && $options.consultEnterprise(...args), "03"),
    o: common_vendor.o((...args) => $options.reserve && $options.reserve(...args), "29"),
    p: common_vendor.p({
      current: 2
    }),
    q: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    r: `${_ctx.u_s_b_h}px`,
    s: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/gift/gift.js.map
