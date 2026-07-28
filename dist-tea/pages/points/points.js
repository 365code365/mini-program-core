"use strict";
const common_vendor = require("../../common/vendor.js");
const api_user = require("../../api/user.js");
const utils_auth = require("../../utils/auth.js");
class Item extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          name: { type: String, optional: false },
          cost: { type: Number, optional: false },
          icon: { type: String, optional: false }
        };
      },
      name: "Item"
    };
  }
  constructor(options, metadata = Item.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.name = this.__props__.name;
    this.cost = this.__props__.cost;
    this.icon = this.__props__.icon;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      points: 0,
      items: [
        new Item({ name: "20 元无门槛券", cost: 200, icon: "/static/icons/gift-d.png" }),
        new Item({ name: "品鉴装兑换券", cost: 500, icon: "/static/icons/leaf-d.png" }),
        new Item({ name: "云岫定制茶则", cost: 1200, icon: "/static/icons/doc-d.png" }),
        new Item({ name: "50 元认养券", cost: 800, icon: "/static/icons/gift-d.png" }),
        new Item({ name: "盖碗品鉴套装", cost: 2e3, icon: "/static/icons/cart-d.png" }),
        new Item({ name: "古树认养抵扣券", cost: 3e3, icon: "/static/icons/tree-d.png" })
      ]
    };
  },
  onLoad() {
    if (utils_auth.isLogin()) {
      api_user.integralUser().then((d) => {
        var _a, _b;
        this.points = (_a = d.getNumber("integral")) !== null && _a !== void 0 ? _a : (_b = d.getNumber("integralCount")) !== null && _b !== void 0 ? _b : 0;
      }).catch((_e = null) => {
      });
    }
  },
  methods: {
    redeem(cost) {
      if (this.points < cost)
        return null;
      common_vendor.index.showToast({ title: "兑换功能即将开放", icon: "none" });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: common_vendor.t($data.points),
    b: common_vendor.f($data.items, (it, i, i0) => {
      return {
        a: it.icon,
        b: common_vendor.t(it.name),
        c: common_vendor.t(it.cost),
        d: common_vendor.t($data.points >= it.cost ? "立即兑换" : "积分不足"),
        e: common_vendor.n($data.points >= it.cost ? "" : "cell-btn-off"),
        f: common_vendor.o(($event) => $options.redeem(it.cost), i),
        g: i
      };
    }),
    c: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    d: `${_ctx.u_s_b_h}px`,
    e: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
