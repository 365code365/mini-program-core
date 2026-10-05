"use strict";
const common_vendor = require("../../common/vendor.js");
const common_assets = require("../../common/assets.js");
class Champ extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          pid: { type: String, optional: false },
          name: { type: String, optional: false },
          award: { type: String, optional: false },
          desc: { type: String, optional: false },
          price: { type: Number, optional: false },
          cover: { type: String, optional: false }
        };
      },
      name: "Champ"
    };
  }
  constructor(options, metadata = Champ.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.pid = this.__props__.pid;
    this.name = this.__props__.name;
    this.award = this.__props__.award;
    this.desc = this.__props__.desc;
    this.price = this.__props__.price;
    this.cover = this.__props__.cover;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      champions: [
        new Champ({ pid: "p4", name: "宋种水仙 · 茶王珍藏", award: "国家级斗茶赛冠军", desc: "老枞水仙 · 限量 88 罐", price: 1280, cover: "/static/slices/category-aroma/card-shuixian.jpg" }),
        new Champ({ pid: "p1", name: "鸭屎香 · 茶王甄选", award: "省赛金奖", desc: "核心产区古树 · 礼盒装", price: 880, cover: "/static/slices/category-aroma/card-yashixiang.jpg" }),
        new Champ({ pid: "p2", name: "蜜兰香 · 名丛臻品", award: "市赛特等奖", desc: "甜蜜幽兰 · 韵味悠长", price: 680, cover: "/static/slices/category-aroma/card-milanxiang.jpg" })
      ]
    };
  },
  methods: { open(id) {
    common_vendor.index.navigateTo({ url: "/pages/shop/detail?id=" + id });
  } }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: common_vendor.f($data.champions, (c, i, i0) => {
      return {
        a: c.cover,
        b: common_vendor.t(c.award),
        c: common_vendor.t(c.name),
        d: common_vendor.t(c.desc),
        e: common_vendor.t(c.price),
        f: i,
        g: common_vendor.o(($event) => $options.open(c.pid), i)
      };
    }),
    b: common_assets._imports_0$13,
    c: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    d: `${_ctx.u_s_b_h}px`,
    e: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/tea-king/tea-king.js.map
