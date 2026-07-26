"use strict";
const common_vendor = require("../../common/vendor.js");
const utils_assets = require("../../utils/assets.js");
const common_assets = require("../../common/assets.js");
class Reason extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          name: { type: String, optional: false },
          desc: { type: String, optional: false },
          icon: { type: String, optional: false }
        };
      },
      name: "Reason"
    };
  }
  constructor(options, metadata = Reason.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.name = this.__props__.name;
    this.desc = this.__props__.desc;
    this.icon = this.__props__.icon;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      statusBarH: 20,
      imgHero: utils_assets.assetUrl("home_hero", "/static/gen/home-hero.jpg"),
      imgRecKing: utils_assets.assetUrl("home_rec_teaking", "/static/gen/rec-teaking.jpg"),
      imgRecTasting: utils_assets.assetUrl("home_rec_tasting", "/static/gen/rec-tasting.jpg"),
      imgFeatOrigin: utils_assets.assetUrl("home_feat_origin", "/static/gen/feat-origin.jpg"),
      imgFeatBiz: utils_assets.assetUrl("home_feat_biz", "/static/gen/feat-biz.jpg"),
      reasons: [
        new Reason({ name: "真实产区", desc: "凤凰核心产区\n源头可溯", icon: "/static/icons/mountain-d.png" }),
        new Reason({ name: "可信师傅", desc: "当地制茶师傅\n世代传承", icon: "/static/icons/user-d.png" }),
        new Reason({ name: "认真选茶", desc: "层层筛选把关\n只为好茶", icon: "/static/icons/leaf-d.png" }),
        new Reason({ name: "价格相符", desc: "去除品牌溢价\n价值透明", icon: "/static/icons/doc-d.png" })
      ]
    };
  },
  onLoad() {
    this.statusBarH = common_vendor.index.getWindowInfo().statusBarHeight;
    this.imgHero = utils_assets.assetUrl("home_hero", "/static/gen/home-hero.jpg");
    this.imgRecKing = utils_assets.assetUrl("home_rec_teaking", "/static/gen/rec-teaking.jpg");
    this.imgRecTasting = utils_assets.assetUrl("home_rec_tasting", "/static/gen/rec-tasting.jpg");
    this.imgFeatOrigin = utils_assets.assetUrl("home_feat_origin", "/static/gen/feat-origin.jpg");
    this.imgFeatBiz = utils_assets.assetUrl("home_feat_biz", "/static/gen/feat-biz.jpg");
  },
  methods: {
    // from=home 让品牌片可以「返回」首页；冷启动直接进 splash 时没有上一页，只能 switchTab
    openSplash() {
      common_vendor.index.navigateTo({ url: "/pages/splash/splash?from=home" });
    },
    goAi() {
      common_vendor.index.switchTab({ url: "/pages/choose/choose" });
    },
    goGift() {
      common_vendor.index.switchTab({ url: "/pages/gift/gift" });
    },
    goRecommend() {
      common_vendor.index.navigateTo({ url: "/pages/recommend/recommend" });
    },
    goOrigin() {
      common_vendor.index.switchTab({ url: "/pages/origin/origin" });
    },
    goBiz() {
      common_vendor.index.switchTab({ url: "/pages/gift/gift" });
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
    a: common_vendor.s("padding-top:" + ($data.statusBarH + 8) + "px"),
    b: $data.imgHero,
    c: common_vendor.o((...args) => $options.openSplash && $options.openSplash(...args), "9c"),
    d: common_assets._imports_0,
    e: common_vendor.o((...args) => $options.goAi && $options.goAi(...args), "e8"),
    f: common_assets._imports_1,
    g: common_vendor.o((...args) => $options.goGift && $options.goGift(...args), "de"),
    h: common_vendor.o((...args) => $options.goRecommend && $options.goRecommend(...args), "34"),
    i: $data.imgRecKing,
    j: common_vendor.o((...args) => $options.goRecommend && $options.goRecommend(...args), "28"),
    k: $data.imgRecTasting,
    l: common_vendor.o((...args) => $options.goRecommend && $options.goRecommend(...args), "43"),
    m: common_vendor.f($data.reasons, (item, i, i0) => {
      return {
        a: item.icon,
        b: common_vendor.t(item.name),
        c: common_vendor.t(item.desc),
        d: i
      };
    }),
    n: $data.imgFeatOrigin,
    o: common_vendor.o((...args) => $options.goOrigin && $options.goOrigin(...args), "2b"),
    p: $data.imgFeatBiz,
    q: common_vendor.o((...args) => $options.goBiz && $options.goBiz(...args), "3b"),
    r: common_vendor.p({
      current: 0
    }),
    s: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    t: `${_ctx.u_s_b_h}px`,
    v: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
