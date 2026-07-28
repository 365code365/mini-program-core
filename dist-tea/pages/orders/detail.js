"use strict";
const common_vendor = require("../../common/vendor.js");
const api_order = require("../../api/order.js");
const api_cart = require("../../api/cart.js");
const api_mappers = require("../../api/mappers.js");
const common_assets = require("../../common/assets.js");
class ExpressTrace extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          context: { type: String, optional: false },
          time: { type: String, optional: false }
        };
      },
      name: "ExpressTrace"
    };
  }
  constructor(options, metadata = ExpressTrace.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.context = this.__props__.context;
    this.time = this.__props__.time;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      id: "",
      o: new api_mappers.OrderVM({ id: 0, orderId: "", statusText: "", status: 0, payPrice: 0, totalNum: 0, createTime: "", items: [], refundStatus: 0 }),
      addrName: "",
      addrPhone: "",
      addrDetail: "",
      productIds: [],
      showRefund: false,
      reasons: [],
      reasonSel: "",
      refundExplain: "",
      refunding: false,
      showExpress: false,
      expressName: "物流公司",
      expressNo: "",
      expressTraces: [],
      expressTip: "暂无物流轨迹，请稍后再试"
    };
  },
  computed: {
    hint() {
      if (this.o.statusText == "待付款")
        return "请尽快完成支付";
      if (this.o.statusText == "待发货")
        return "商家正在备货，请耐心等待";
      if (this.o.statusText == "待收货")
        return "商品已发出，注意查收";
      return "交易已完成，感谢惠顾";
    }
  },
  onLoad(opt) {
    const id = opt["id"];
    if (id != null) {
      this.id = id;
      this.load();
    }
  },
  methods: {
    money(n) {
      return api_mappers.fmtMoney(n);
    },
    noop() {
    },
    openRefund() {
      this.showRefund = true;
      if (this.reasons.length == 0) {
        api_order.refundReason().then((arr) => {
          this.reasons = arr;
        }).catch((_e = null) => {
          this.reasons = ["不想要了", "商品缺货", "信息填写错误", "商品质量问题", "其他"];
        });
      }
    },
    submitRefund() {
      if (this.reasonSel.length == 0 || this.refunding)
        return null;
      this.refunding = true;
      api_order.orderRefund(this.o.id, this.reasonSel, this.refundExplain, this.o.orderId).then((_d) => {
        this.refunding = false;
        this.showRefund = false;
        common_vendor.index.showToast({ title: "退款申请已提交", icon: "none" });
        this.load();
      }).catch((_e = null) => {
        this.refunding = false;
      });
    },
    openExpress() {
      this.showExpress = true;
      api_order.orderExpress(this.o.orderId).then((d) => {
        var _a, _b, _c, _f, _g, _h, _j, _k, _l, _m, _o, _p;
        this.expressName = (_a = d.getString("expressName")) !== null && _a !== void 0 ? _a : (_b = d.getString("deliveryName")) !== null && _b !== void 0 ? _b : "物流公司";
        this.expressNo = (_c = d.getString("expressNo")) !== null && _c !== void 0 ? _c : (_f = d.getString("deliveryId")) !== null && _f !== void 0 ? _f : "";
        const traces = [];
        const list = (_h = (_g = d["list"]) !== null && _g !== void 0 ? _g : d["traces"]) !== null && _h !== void 0 ? _h : d["result"];
        if (list != null && Array.isArray(list)) {
          const arr = list;
          for (let i = 0; i < arr.length; i++) {
            const t = arr[i];
            traces.push(new ExpressTrace({
              context: (_j = t.getString("context")) !== null && _j !== void 0 ? _j : (_k = t.getString("status")) !== null && _k !== void 0 ? _k : (_l = t.getString("desc")) !== null && _l !== void 0 ? _l : "",
              time: (_m = t.getString("time")) !== null && _m !== void 0 ? _m : (_o = t.getString("ftime")) !== null && _o !== void 0 ? _o : (_p = t.getString("acceptTime")) !== null && _p !== void 0 ? _p : ""
            }));
          }
        }
        this.expressTraces = traces;
      }).catch((_e = null) => {
        this.expressTraces = [];
      });
    },
    load() {
      api_order.orderDetail(this.id).then((d) => {
        var _a, _b, _c, _f;
        this.o = api_mappers.toOrder(d);
        this.addrName = (_a = d.getString("realName")) !== null && _a !== void 0 ? _a : "";
        this.addrPhone = (_b = d.getString("userPhone")) !== null && _b !== void 0 ? _b : "";
        this.addrDetail = (_c = d.getString("userAddress")) !== null && _c !== void 0 ? _c : "";
        const list = (_f = d["orderInfoList"]) !== null && _f !== void 0 ? _f : d["productList"];
        if (list != null && Array.isArray(list)) {
          const arr = list;
          const ids = [];
          for (let i = 0; i < arr.length; i++) {
            const pid = arr[i].getNumber("productId");
            if (pid != null)
              ids.push(pid);
          }
          this.productIds = ids;
        }
      }).catch((_e = null) => {
      });
    },
    take() {
      api_order.orderTake(this.o.id).then((_d) => {
        common_vendor.index.showToast({ title: "已确认收货", icon: "success" });
        this.load();
      }).catch((_e = null) => {
      });
    },
    cancel() {
      api_order.orderCancel(this.o.id).then((_d) => {
        common_vendor.index.showToast({ title: "订单已取消", icon: "none" });
        this.load();
      }).catch((_e = null) => {
      });
    },
    buyAgain() {
      for (let i = 0; i < this.productIds.length; i++) {
        api_cart.cartSave(this.productIds[i], 1, "").catch((_e = null) => {
        });
      }
      common_vendor.index.navigateTo({ url: "/pages/cart/cart" });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_vendor.t($data.o.statusText),
    b: common_vendor.t($options.hint),
    c: $data.addrName.length > 0
  }, $data.addrName.length > 0 ? {
    d: common_assets._imports_0$8,
    e: common_vendor.t($data.addrName),
    f: common_vendor.t($data.addrPhone),
    g: common_vendor.t($data.addrDetail)
  } : {}, {
    h: common_vendor.f($data.o.items, (it, i, i0) => {
      return {
        a: it.cover,
        b: common_vendor.t(it.name),
        c: common_vendor.t($options.money(it.price)),
        d: common_vendor.t(it.num),
        e: i
      };
    }),
    i: common_vendor.t($data.o.totalNum),
    j: common_vendor.t($options.money($data.o.payPrice)),
    k: common_vendor.t($data.o.orderId),
    l: common_vendor.t($data.o.createTime),
    m: common_vendor.t($data.o.statusText),
    n: $data.o.statusText == "待付款"
  }, $data.o.statusText == "待付款" ? {
    o: common_vendor.o((...args) => $options.cancel && $options.cancel(...args), "0f")
  } : {}, {
    p: $data.o.statusText == "待收货"
  }, $data.o.statusText == "待收货" ? {
    q: common_vendor.o((...args) => $options.openExpress && $options.openExpress(...args), "7b")
  } : {}, {
    r: ($data.o.statusText == "待发货" || $data.o.statusText == "待收货") && $data.o.refundStatus == 0
  }, ($data.o.statusText == "待发货" || $data.o.statusText == "待收货") && $data.o.refundStatus == 0 ? {
    s: common_vendor.o((...args) => $options.openRefund && $options.openRefund(...args), "27")
  } : {}, {
    t: $data.o.statusText == "待收货"
  }, $data.o.statusText == "待收货" ? {
    v: common_vendor.o((...args) => $options.take && $options.take(...args), "6b")
  } : {}, {
    w: common_vendor.o((...args) => $options.buyAgain && $options.buyAgain(...args), "f3"),
    x: $data.showRefund
  }, $data.showRefund ? {
    y: common_assets._imports_1$4,
    z: common_vendor.o(($event) => $data.showRefund = false, "42"),
    A: common_vendor.f($data.reasons, (r, i, i0) => {
      return {
        a: common_vendor.t(r),
        b: common_vendor.n($data.reasonSel == r ? "rf-on" : ""),
        c: common_vendor.n($data.reasonSel == r ? "cb-on" : ""),
        d: i,
        e: common_vendor.o(($event) => $data.reasonSel = r, i)
      };
    }),
    B: common_assets._imports_0$5,
    C: $data.refundExplain,
    D: common_vendor.o(($event) => $data.refundExplain = $event.detail.value, "15"),
    E: common_vendor.t($data.refunding ? "提交中…" : "提交退款申请"),
    F: common_vendor.n($data.reasonSel.length == 0 || $data.refunding ? "rf-off" : ""),
    G: common_vendor.o((...args) => $options.submitRefund && $options.submitRefund(...args), "5b"),
    H: common_vendor.o((...args) => $options.noop && $options.noop(...args), "ac"),
    I: common_vendor.o(($event) => $data.showRefund = false, "43")
  } : {}, {
    J: $data.showExpress
  }, $data.showExpress ? common_vendor.e({
    K: common_assets._imports_1$4,
    L: common_vendor.o(($event) => $data.showExpress = false, "c6"),
    M: common_vendor.t($data.expressName),
    N: common_vendor.t($data.expressNo),
    O: common_vendor.f($data.expressTraces, (t, i, i0) => {
      return {
        a: common_vendor.t(t.context),
        b: common_vendor.t(t.time),
        c: i
      };
    }),
    P: $data.expressTraces.length == 0
  }, $data.expressTraces.length == 0 ? {
    Q: common_vendor.t($data.expressTip)
  } : {}, {
    R: common_vendor.o((...args) => $options.noop && $options.noop(...args), "f2"),
    S: common_vendor.o(($event) => $data.showExpress = false, "9f")
  }) : {}, {
    T: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    U: `${_ctx.u_s_b_h}px`,
    V: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
