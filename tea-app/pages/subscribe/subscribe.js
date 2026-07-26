"use strict";
const common_vendor = require("../../common/vendor.js");
const api_subscribe = require("../../api/subscribe.js");
const api_address = require("../../api/address.js");
require("../../api/models.js");
const api_mappers = require("../../api/mappers.js");
const utils_auth = require("../../utils/auth.js");
class MineVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          orderNo: { type: String, optional: false },
          statusText: { type: String, optional: false },
          status: { type: Number, optional: false },
          periods: { type: Number, optional: false },
          delivered: { type: Number, optional: false },
          nextTime: { type: String, optional: false }
        };
      },
      name: "MineVM"
    };
  }
  constructor(options, metadata = MineVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.orderNo = this.__props__.orderNo;
    this.statusText = this.__props__.statusText;
    this.status = this.__props__.status;
    this.periods = this.__props__.periods;
    this.delivered = this.__props__.delivered;
    this.nextTime = this.__props__.nextTime;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return { tab: 0, plans: [], mine: [], loading: false };
  },
  onLoad() {
    this.loadPlans();
  },
  methods: {
    loadPlans() {
      this.loading = true;
      api_subscribe.planList().then((arr) => {
        const vms = [];
        for (let i = 0; i < arr.length; i++)
          vms.push(api_mappers.toPlan(arr[i]));
        this.plans = vms;
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    switchMine() {
      this.tab = 1;
      if (!utils_auth.isLogin()) {
        common_vendor.index.navigateTo({ url: "/pages/login/login?redirect=/pages/subscribe/subscribe" });
        return null;
      }
      this.loadMine();
    },
    loadMine() {
      this.loading = true;
      api_subscribe.myList(0, 1, 50).then((res) => {
        var _a, _b, _c, _f, _g, _h, _j, _k, _l, _m, _o;
        const vms = [];
        for (let i = 0; i < res.list.length; i++) {
          const o = res.list[i];
          const st = (_a = o.getNumber("status")) !== null && _a !== void 0 ? _a : 0;
          vms.push(new MineVM({
            id: (_b = o.getNumber("id")) !== null && _b !== void 0 ? _b : 0,
            name: (_c = o.getString("planName")) !== null && _c !== void 0 ? _c : (_f = o.getString("name")) !== null && _f !== void 0 ? _f : "山泉订阅",
            orderNo: (_g = o.getString("orderNo")) !== null && _g !== void 0 ? _g : "",
            status: st,
            statusText: st == 1 ? "进行中" : st == 2 ? "已完成" : "已取消",
            periods: (_h = o.getNumber("totalPeriods")) !== null && _h !== void 0 ? _h : (_j = o.getNumber("periods")) !== null && _j !== void 0 ? _j : 0,
            delivered: (_k = o.getNumber("deliveredPeriods")) !== null && _k !== void 0 ? _k : (_l = o.getNumber("delivered")) !== null && _l !== void 0 ? _l : 0,
            nextTime: (_m = o.getString("nextDeliveryDate")) !== null && _m !== void 0 ? _m : (_o = o.getString("nextDeliveryTime")) !== null && _o !== void 0 ? _o : "—"
          }));
        }
        this.mine = vms;
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    subscribe(p) {
      if (p.stock == 0)
        return null;
      if (!utils_auth.isLogin()) {
        common_vendor.index.navigateTo({ url: "/pages/login/login?redirect=/pages/subscribe/subscribe" });
        return null;
      }
      api_address.addressDefault().then((a) => {
        var _a;
        const addressId = (_a = a.getNumber("id")) !== null && _a !== void 0 ? _a : 0;
        if (addressId == 0) {
          common_vendor.index.showToast({ title: "请先添加收货地址", icon: "none" });
          common_vendor.index.navigateTo({ url: "/pages/address/address" });
          return null;
        }
        api_subscribe.createSubscribe(p.id, addressId, "yue", "routine", true, 0, "").then((r) => {
          var _a2;
          (_a2 = r.getString("payType")) !== null && _a2 !== void 0 ? _a2 : "";
          common_vendor.index.showToast({ title: "订阅成功", icon: "success" });
          setTimeout(() => {
            this.switchMine();
          }, 700);
        }).catch((_e = null) => {
        });
      }).catch((_e = null) => {
        common_vendor.index.showToast({ title: "请先添加收货地址", icon: "none" });
        common_vendor.index.navigateTo({ url: "/pages/address/address" });
      });
    },
    doCancel(id) {
      api_subscribe.cancel(id).then((_d) => {
        common_vendor.index.showToast({ title: "已取消", icon: "none" });
        this.loadMine();
      }).catch((_e = null) => {
      });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_vendor.n($data.tab == 0 ? "tab-on" : ""),
    b: common_vendor.o(($event) => $data.tab = 0, "c7"),
    c: common_vendor.n($data.tab == 1 ? "tab-on" : ""),
    d: common_vendor.o((...args) => $options.switchMine && $options.switchMine(...args), "23"),
    e: $data.tab == 0
  }, $data.tab == 0 ? common_vendor.e({
    f: !$data.loading && $data.plans.length == 0
  }, !$data.loading && $data.plans.length == 0 ? {} : {}, {
    g: common_vendor.f($data.plans, (p, i, i0) => {
      return common_vendor.e({
        a: p.cover,
        b: common_vendor.t(p.name),
        c: common_vendor.t(p.intro),
        d: common_vendor.t(p.periods),
        e: common_vendor.t(p.intervalDays),
        f: p.spec.length > 0
      }, p.spec.length > 0 ? {
        g: common_vendor.t(p.spec)
      } : {}, {
        h: common_vendor.t(p.price),
        i: common_vendor.t(p.stock == 0 ? "已售罄" : "立即订阅"),
        j: common_vendor.n(p.stock == 0 ? "btn-off" : ""),
        k: common_vendor.o(($event) => $options.subscribe(p), i),
        l: i
      });
    })
  }) : common_vendor.e({
    h: !$data.loading && $data.mine.length == 0
  }, !$data.loading && $data.mine.length == 0 ? {} : {}, {
    i: common_vendor.f($data.mine, (m, i, i0) => {
      return common_vendor.e({
        a: common_vendor.t(m.name),
        b: common_vendor.t(m.statusText),
        c: common_vendor.t(m.orderNo),
        d: common_vendor.t(m.delivered),
        e: common_vendor.t(m.periods),
        f: common_vendor.t(m.nextTime),
        g: m.status == 1
      }, m.status == 1 ? {
        h: common_vendor.o(($event) => $options.doCancel(m.id), i)
      } : {}, {
        i
      });
    })
  }), {
    j: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    k: `${_ctx.u_s_b_h}px`,
    l: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
