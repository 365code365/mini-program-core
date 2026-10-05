"use strict";
const common_vendor = require("../../common/vendor.js");
const api_cart = require("../../api/cart.js");
require("../../api/models.js");
const api_mappers = require("../../api/mappers.js");
const utils_auth = require("../../utils/auth.js");
const common_assets = require("../../common/assets.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return { items: [], loading: false };
  },
  computed: {
    allChecked() {
      if (this.items.length == 0)
        return false;
      for (let i = 0; i < this.items.length; i++) {
        if (!this.items[i].checked)
          return false;
      }
      return true;
    },
    checkedCount() {
      let n = 0;
      for (let i = 0; i < this.items.length; i++) {
        if (this.items[i].checked)
          n += 1;
      }
      return n;
    },
    total() {
      let t = 0;
      for (let i = 0; i < this.items.length; i++) {
        if (this.items[i].checked)
          t += this.items[i].price * this.items[i].num;
      }
      return t;
    }
  },
  onShow() {
    if (!utils_auth.isLogin()) {
      this.items = [];
      return null;
    }
    this.load();
  },
  methods: {
    money(n) {
      return api_mappers.fmtMoney(n);
    },
    load() {
      this.loading = true;
      api_cart.cartList(1, 100).then((res) => {
        const vms = [];
        for (let i = 0; i < res.list.length; i++)
          vms.push(api_mappers.toCart(res.list[i]));
        this.items = vms;
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    toggle(i) {
      this.items[i].checked = !this.items[i].checked;
    },
    toggleAllItems() {
      const next = !this.allChecked;
      for (let i = 0; i < this.items.length; i++)
        this.items[i].checked = next;
    },
    setQ(i, q) {
      if (q < 1)
        return null;
      const it = this.items[i];
      if (it.stock > 0 && q > it.stock) {
        common_vendor.index.showToast({ title: "已达库存上限", icon: "none" });
        return null;
      }
      const old = it.num;
      it.num = q;
      api_cart.cartNum(it.id, q).catch((_e = null) => {
        it.num = old;
      });
    },
    del(id) {
      api_cart.cartDelete([id]).then((_d) => {
        this.items = this.items.filter((x) => {
          return x.id != id;
        });
      }).catch((_e = null) => {
      });
    },
    goShop() {
      common_vendor.index.navigateTo({ url: "/pages/shop/list" });
    },
    switchTo(i) {
      const urls = ["/pages/index/index", "/pages/choose/choose", "/pages/gift/gift", "/pages/origin/origin", "/pages/mine/mine"];
      if (i >= 0 && i < urls.length)
        common_vendor.index.switchTab({ url: urls[i] });
    },
    goBrand() {
      common_vendor.index.navigateTo({ url: "/pages/brand/brand" });
    },
    toCheckout() {
      if (this.checkedCount == 0)
        return null;
      let ids = "";
      for (let i = 0; i < this.items.length; i++) {
        if (this.items[i].checked)
          ids += (ids.length > 0 ? "," : "") + this.items[i].id;
      }
      common_vendor.index.navigateTo({ url: "/pages/cart/checkout?ids=" + ids });
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
  return common_vendor.e({
    a: !$data.loading && $data.items.length == 0
  }, !$data.loading && $data.items.length == 0 ? {
    b: common_assets._imports_0$7,
    c: common_vendor.o((...args) => $options.goShop && $options.goShop(...args), "2a")
  } : {
    d: common_vendor.f($data.items, (c, i, i0) => {
      return common_vendor.e({
        a: common_vendor.n(c.checked ? "cb-on" : ""),
        b: common_vendor.o(($event) => $options.toggle(i), i),
        c: c.cover,
        d: common_vendor.t(c.name),
        e: common_vendor.o(($event) => $options.del(c.id), i),
        f: !c.valid
      }, !c.valid ? {} : {}, {
        g: common_vendor.t($options.money(c.price)),
        h: common_vendor.o(($event) => $options.setQ(i, c.num - 1), i),
        i: common_vendor.t(c.num),
        j: common_vendor.o(($event) => $options.setQ(i, c.num + 1), i),
        k: i
      });
    }),
    e: common_assets._imports_0$6
  }, {
    f: $data.items.length > 0
  }, $data.items.length > 0 ? {
    g: common_assets._imports_0$6,
    h: common_vendor.n($options.allChecked ? "cb-on" : ""),
    i: common_vendor.o((...args) => $options.toggleAllItems && $options.toggleAllItems(...args), "0a"),
    j: common_vendor.t($options.money($options.total)),
    k: common_vendor.t($options.checkedCount),
    l: common_vendor.n($options.checkedCount == 0 ? "btn-off" : ""),
    m: common_vendor.o((...args) => $options.toCheckout && $options.toCheckout(...args), "35")
  } : {}, {
    n: common_vendor.p({
      current: -1
    }),
    o: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    p: `${_ctx.u_s_b_h}px`,
    q: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/cart/cart.js.map
