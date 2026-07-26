"use strict";
const common_vendor = require("../../common/vendor.js");
const api_order = require("../../api/order.js");
require("../../api/models.js");
const api_mappers = require("../../api/mappers.js");
const utils_auth = require("../../utils/auth.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      tabs: ["待付款", "待发货", "待收货", "待评价", "已完成"],
      tabIndex: 0,
      list: [],
      page: 1,
      limit: 10,
      loading: false,
      noMore: false
    };
  },
  onLoad(opt) {
    if (!utils_auth.isLogin()) {
      common_vendor.index.redirectTo({ url: "/pages/login/login?redirect=/pages/orders/orders" });
      return null;
    }
    const status = opt["status"];
    if (status != null) {
      const parsed = parseInt(status);
      if (parsed >= 0 && parsed <= 4)
        this.tabIndex = parsed;
    }
    this.load(true);
  },
  methods: {
    money(n) {
      return api_mappers.fmtMoney(n);
    },
    switchTab(i) {
      if (this.tabIndex == i)
        return null;
      this.tabIndex = i;
      this.load(true);
    },
    load(reset) {
      if (reset) {
        this.page = 1;
        this.noMore = false;
        this.list = [];
      } else if (this.loading)
        return null;
      this.loading = true;
      const reqTab = this.tabIndex;
      const reqPage = this.page;
      api_order.orderList(reqTab, reqPage, this.limit).then((res) => {
        if (reqTab != this.tabIndex) {
          this.loading = false;
          return null;
        }
        const vms = [];
        for (let i = 0; i < res.list.length; i++)
          vms.push(api_mappers.toOrder(res.list[i]));
        this.list = reqPage == 1 ? vms : this.list.concat(vms);
        if (vms.length < this.limit)
          this.noMore = true;
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    loadMore() {
      if (this.noMore || this.loading)
        return null;
      this.page++;
      this.load(false);
    },
    open(id) {
      common_vendor.index.navigateTo({ url: "/pages/orders/detail?id=" + id });
    },
    goShop() {
      common_vendor.index.navigateTo({ url: "/pages/shop/list" });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_vendor.f($data.tabs, (t, i, i0) => {
      return {
        a: common_vendor.t(t),
        b: i,
        c: common_vendor.n($data.tabIndex == i ? "tab-on" : ""),
        d: common_vendor.o(($event) => $options.switchTab(i), i)
      };
    }),
    b: !$data.loading && $data.list.length == 0
  }, !$data.loading && $data.list.length == 0 ? {
    c: common_vendor.o((...args) => $options.goShop && $options.goShop(...args), "c2")
  } : {}, {
    d: common_vendor.f($data.list, (o, i, i0) => {
      return {
        a: common_vendor.t(o.orderId),
        b: common_vendor.t(o.statusText),
        c: common_vendor.n(o.statusText == "已完成" ? "status-muted" : "status-gold"),
        d: common_vendor.f(o.items, (it, j, i1) => {
          return {
            a: it.cover,
            b: common_vendor.t(it.name),
            c: common_vendor.t($options.money(it.price)),
            d: common_vendor.t(it.num),
            e: j
          };
        }),
        e: common_vendor.t(o.totalNum),
        f: common_vendor.t($options.money(o.payPrice)),
        g: i,
        h: common_vendor.o(($event) => $options.open(o.orderId), i)
      };
    }),
    e: $data.noMore && $data.list.length > 0
  }, $data.noMore && $data.list.length > 0 ? {} : {}, {
    f: common_vendor.o((...args) => $options.loadMore && $options.loadMore(...args), "1f"),
    g: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    h: `${_ctx.u_s_b_h}px`,
    i: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
