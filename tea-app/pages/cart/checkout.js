"use strict";
const common_vendor = require("../../common/vendor.js");
const api_order = require("../../api/order.js");
const api_mappers = require("../../api/mappers.js");
const store_selection = require("../../store/selection.js");
const utils_auth = require("../../utils/auth.js");
const common_assets = require("../../common/assets.js");
class CouponItem extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          money: { type: Number, optional: false },
          minPrice: { type: Number, optional: false },
          expire: { type: String, optional: false }
        };
      },
      name: "CouponItem"
    };
  }
  constructor(options, metadata = CouponItem.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.money = this.__props__.money;
    this.minPrice = this.__props__.minPrice;
    this.expire = this.__props__.expire;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      cartIds: [],
      preOrderNo: "",
      items: [],
      addressId: 0,
      addrName: "",
      addrPhone: "",
      addrRegion: "",
      addrDetail: "",
      proTotalFee: 0,
      couponFee: 0,
      freightFee: 0,
      payFee: 0,
      userBalance: 0,
      userCouponId: 0,
      coupons: [],
      selCouponMoney: 0,
      showCoupon: false,
      payType: "yue",
      useIntegral: false,
      paying: false
    };
  },
  computed: {
    couponLabel() {
      if (this.userCouponId > 0)
        return "已选 · 省 ¥" + api_mappers.fmtMoney(this.couponFee > 0 ? this.couponFee : this.selCouponMoney);
      if (this.coupons.length > 0)
        return this.coupons.length + " 张可用";
      return "无可用";
    }
  },
  onLoad(opt) {
    if (!utils_auth.isLogin()) {
      common_vendor.index.redirectTo({ url: "/pages/login/login?redirect=/pages/cart/cart" });
      return null;
    }
    const ids = opt["ids"];
    if (ids != null) {
      const parts = ids.split(",");
      const arr = [];
      for (let i = 0; i < parts.length; i++) {
        if (parts[i].length > 0)
          arr.push(parseInt(parts[i]));
      }
      this.cartIds = arr;
    }
    this.init();
  },
  onShow() {
    const picked = store_selection.takePickedAddress();
    if (picked != null) {
      const a = picked;
      this.addressId = a.id;
      this.addrName = a.name;
      this.addrPhone = a.phone;
      this.addrRegion = a.region;
      this.addrDetail = a.detail;
      this.recompute();
    }
  },
  methods: {
    money(n) {
      return api_mappers.fmtMoney(n);
    },
    init() {
      api_order.preOrderByCart(this.cartIds).then((d) => {
        var _a;
        this.preOrderNo = (_a = d.getString("preOrderNo")) !== null && _a !== void 0 ? _a : "";
        this.applyInfo(d["orderInfoVo"]);
        if (this.preOrderNo.length > 0 && this.items.length == 0) {
          api_order.loadPre(this.preOrderNo).then((p) => {
            this.applyInfo(p["orderInfoVo"]);
          }).catch((_e = null) => {
          });
        }
        if (this.preOrderNo.length > 0)
          this.fetchCoupons();
      }).catch((_e = null) => {
      });
    },
    fetchCoupons() {
      api_order.orderCoupons(this.preOrderNo).then((arr) => {
        var _a, _b, _c;
        const list = [];
        for (let i = 0; i < arr.length; i++) {
          const c = arr[i];
          list.push(new CouponItem({
            id: api_mappers.numOf(c, ["id"]),
            name: (_a = c.getString("name")) !== null && _a !== void 0 ? _a : "优惠券",
            money: api_mappers.numOf(c, ["money"]),
            minPrice: api_mappers.numOf(c, ["minPrice", "useMinPrice"]),
            expire: (_b = c.getString("useEndTimeStr")) !== null && _b !== void 0 ? _b : (_c = c.getString("useEndTime")) !== null && _c !== void 0 ? _c : ""
          }));
        }
        this.coupons = list;
      }).catch((_e = null) => {
      });
    },
    openCoupon() {
      this.showCoupon = true;
    },
    pickCoupon(id, money) {
      this.userCouponId = id;
      this.selCouponMoney = money;
      this.showCoupon = false;
      this.recompute();
    },
    applyInfo(info = null) {
      var _a, _b, _c, _d, _f, _g, _h, _j, _k;
      if (info == null)
        return null;
      const vo = info;
      this.proTotalFee = api_mappers.numOf(vo, ["proTotalFee"]);
      this.couponFee = api_mappers.numOf(vo, ["couponFee"]);
      this.freightFee = api_mappers.numOf(vo, ["freightFee"]);
      const pf = api_mappers.numOf(vo, ["payFee"]);
      this.payFee = pf > 0 ? pf : this.proTotalFee;
      this.userBalance = api_mappers.numOf(vo, ["userBalance"]);
      this.userCouponId = api_mappers.numOf(vo, ["userCouponId"]);
      this.addressId = api_mappers.numOf(vo, ["addressId"]);
      this.addrName = (_a = vo.getString("realName")) !== null && _a !== void 0 ? _a : "";
      this.addrPhone = (_b = vo.getString("phone")) !== null && _b !== void 0 ? _b : "";
      const prov = (_c = vo.getString("province")) !== null && _c !== void 0 ? _c : "";
      const city = (_d = vo.getString("city")) !== null && _d !== void 0 ? _d : "";
      const dist = (_f = vo.getString("district")) !== null && _f !== void 0 ? _f : "";
      this.addrRegion = prov + " " + city + " " + dist;
      this.addrDetail = (_g = vo.getString("detail")) !== null && _g !== void 0 ? _g : "";
      const list = vo["orderDetailList"];
      if (list != null && Array.isArray(list)) {
        const arr = list;
        const vms = [];
        for (let i = 0; i < arr.length; i++) {
          const it = arr[i];
          vms.push(new api_mappers.CartVM({
            id: 0,
            productId: api_mappers.numOf(it, ["productId"]),
            name: (_h = it.getString("productName")) !== null && _h !== void 0 ? _h : "",
            cover: (_j = it.getString("image")) !== null && _j !== void 0 ? _j : "",
            price: api_mappers.numOf(it, ["price", "vipPrice"]),
            num: api_mappers.numOf(it, ["payNum"]),
            sku: (_k = it.getString("sku")) !== null && _k !== void 0 ? _k : "",
            valid: true,
            checked: true,
            stock: 0
          }));
        }
        this.items = vms;
      }
    },
    toAddr() {
      common_vendor.index.navigateTo({ url: "/pages/address/address?select=1" });
    },
    noop() {
    },
    recompute() {
      if (this.preOrderNo.length > 0 && this.addressId > 0) {
        api_order.computedPrice(this.preOrderNo, this.addressId, this.userCouponId, this.useIntegral).then((d) => {
          this.proTotalFee = api_mappers.numOf(d, ["proTotalFee"]);
          this.couponFee = api_mappers.numOf(d, ["couponFee"]);
          this.freightFee = api_mappers.numOf(d, ["freightFee"]);
          const pf = api_mappers.numOf(d, ["payFee"]);
          if (pf > 0)
            this.payFee = pf;
        }).catch((_e = null) => {
        });
      } else {
        this.couponFee = this.userCouponId > 0 ? this.selCouponMoney : 0;
        let pf = this.proTotalFee - this.couponFee + this.freightFee;
        if (pf < 0)
          pf = 0;
        this.payFee = pf;
      }
    },
    submit() {
      if (this.paying)
        return null;
      if (this.addressId == 0) {
        common_vendor.index.showToast({ title: "请先选择收货地址", icon: "none" });
        return null;
      }
      if (this.preOrderNo.length == 0) {
        common_vendor.index.showToast({ title: "订单信息异常，请重试", icon: "none" });
        return null;
      }
      const payChannel = this.payType == "weixin" ? "routine" : "yue";
      this.paying = true;
      api_order.createOrder(this.preOrderNo, this.addressId, this.userCouponId, this.useIntegral, this.payType, payChannel, "").then((d) => {
        var _a, _b;
        const orderNo = (_a = d.getString("orderNo")) !== null && _a !== void 0 ? _a : (_b = d.getString("orderId")) !== null && _b !== void 0 ? _b : "";
        if (orderNo.length == 0) {
          this.paying = false;
          common_vendor.index.showToast({ title: "下单失败", icon: "none" });
          return null;
        }
        this.pay(orderNo, payChannel);
      }).catch((_e = null) => {
        this.paying = false;
      });
    },
    pay(orderNo, payChannel) {
      api_order.payment(orderNo, this.payType, payChannel).then((r) => {
        var _a;
        this.paying = false;
        const status = (_a = r.getString("status")) !== null && _a !== void 0 ? _a : "";
        if (this.payType == "yue" || status == "SUCCESS") {
          common_vendor.index.redirectTo({ url: "/pages/cart/order-success?orderNo=" + orderNo });
        } else {
          common_vendor.index.showToast({ title: "订单已创建，请在订单中完成支付", icon: "none" });
          setTimeout(() => {
            common_vendor.index.redirectTo({ url: "/pages/orders/orders" });
          }, 800);
        }
      }).catch((_e = null) => {
        this.paying = false;
      });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_assets._imports_0$8,
    b: $data.addressId > 0
  }, $data.addressId > 0 ? {
    c: common_vendor.t($data.addrName),
    d: common_vendor.t($data.addrPhone),
    e: common_vendor.t($data.addrRegion),
    f: common_vendor.t($data.addrDetail)
  } : {}, {
    g: common_vendor.o((...args) => $options.toAddr && $options.toAddr(...args), "fb"),
    h: common_vendor.f($data.items, (c, i, i0) => {
      return {
        a: c.cover,
        b: common_vendor.t(c.name),
        c: common_vendor.t($options.money(c.price)),
        d: common_vendor.t(c.num),
        e: i
      };
    }),
    i: common_vendor.t($options.couponLabel),
    j: common_vendor.n($data.userCouponId > 0 ? "coupon-on" : ""),
    k: common_vendor.o((...args) => $options.openCoupon && $options.openCoupon(...args), "d3"),
    l: common_vendor.t($options.money($data.userBalance)),
    m: common_assets._imports_0$5,
    n: common_vendor.n($data.payType == "yue" ? "cb-on" : ""),
    o: common_vendor.o(($event) => $data.payType = "yue", "11"),
    p: common_assets._imports_0$5,
    q: common_vendor.n($data.payType == "weixin" ? "cb-on" : ""),
    r: common_vendor.o(($event) => $data.payType = "weixin", "56"),
    s: common_vendor.t($options.money($data.proTotalFee)),
    t: $data.couponFee > 0
  }, $data.couponFee > 0 ? {
    v: common_vendor.t($options.money($data.couponFee))
  } : {}, {
    w: common_vendor.t($data.freightFee > 0 ? "¥" + $options.money($data.freightFee) : "包邮"),
    x: common_vendor.t($options.money($data.payFee)),
    y: common_vendor.t($options.money($data.payFee)),
    z: common_vendor.t($data.paying ? "提交中…" : "提交订单"),
    A: common_vendor.n($data.paying ? "btn-off" : ""),
    B: common_vendor.o((...args) => $options.submit && $options.submit(...args), "bc"),
    C: $data.showCoupon
  }, $data.showCoupon ? common_vendor.e({
    D: common_assets._imports_1$3,
    E: common_vendor.o(($event) => $data.showCoupon = false, "da"),
    F: common_assets._imports_0$5,
    G: common_vendor.n($data.userCouponId == 0 ? "cb-on" : ""),
    H: common_vendor.o(($event) => $options.pickCoupon(0, 0), "c2"),
    I: common_vendor.f($data.coupons, (c, i, i0) => {
      return {
        a: common_vendor.t($options.money(c.money)),
        b: common_vendor.t(c.minPrice > 0 ? "满 " + $options.money(c.minPrice) + " 可用" : "无门槛"),
        c: common_vendor.t(c.name),
        d: common_vendor.t(c.expire),
        e: common_vendor.n($data.userCouponId == c.id ? "cb-on" : ""),
        f: i,
        g: common_vendor.o(($event) => $options.pickCoupon(c.id, c.money), i)
      };
    }),
    J: common_assets._imports_0$5,
    K: $data.coupons.length == 0
  }, $data.coupons.length == 0 ? {} : {}, {
    L: common_vendor.o((...args) => $options.noop && $options.noop(...args), "df"),
    M: common_vendor.o(($event) => $data.showCoupon = false, "8a")
  }) : {}, {
    N: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    O: `${_ctx.u_s_b_h}px`,
    P: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
