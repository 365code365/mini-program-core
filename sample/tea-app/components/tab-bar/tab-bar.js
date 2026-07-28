"use strict";
const common_vendor = require("../../common/vendor.js");
class TabItem extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          label: { type: String, optional: false },
          off: { type: String, optional: false },
          on: { type: String, optional: false },
          url: { type: String, optional: false }
        };
      },
      name: "TabItem"
    };
  }
  constructor(options, metadata = TabItem.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.label = this.__props__.label;
    this.off = this.__props__.off;
    this.on = this.__props__.on;
    this.url = this.__props__.url;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  props: {
    current: { type: Number, default: 0 }
  },
  data() {
    return {
      items: [
        new TabItem({ label: "首页", off: "/static/tabicons/home-off.png", on: "/static/tabicons/home-on.png", url: "/pages/index/index" }),
        new TabItem({ label: "选茶", off: "/static/tabicons/leaf-off.png", on: "/static/tabicons/leaf-on.png", url: "/pages/choose/choose" }),
        new TabItem({ label: "茶礼", off: "/static/tabicons/gift-off.png", on: "/static/tabicons/gift-on.png", url: "/pages/gift/gift" }),
        new TabItem({ label: "初心", off: "/static/tabicons/pot-off.png", on: "/static/tabicons/pot-on.png", url: "/pages/origin/origin" }),
        new TabItem({ label: "我的", off: "/static/tabicons/mine-off.png", on: "/static/tabicons/mine-on.png", url: "/pages/mine/mine" })
      ]
    };
  },
  methods: {
    go(i) {
      if (i == this.current)
        return null;
      common_vendor.index.switchTab({ url: this.items[i].url });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: common_vendor.f($data.items, (item, i, i0) => {
      return {
        a: $props.current == i ? item.on : item.off,
        b: common_vendor.t(item.label),
        c: common_vendor.n($props.current == i ? "label-on" : ""),
        d: i,
        e: common_vendor.o(($event) => $options.go(i), i)
      };
    }),
    b: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    c: `${_ctx.u_s_b_h}px`,
    d: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const Component = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createComponent(Component);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/components/tab-bar/tab-bar.js.map
