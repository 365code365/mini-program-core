"use strict";
const common_vendor = require("../../common/vendor.js");
const api_adopt = require("../../api/adopt.js");
const api_agreement = require("../../api/agreement.js");
const api_user = require("../../api/user.js");
const api_mappers = require("../../api/mappers.js");
const utils_auth = require("../../utils/auth.js");
const common_assets = require("../../common/assets.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      balance: 0,
      g: new api_mappers.GardenVM({ id: 0, name: "", variety: "", altitude: 0, area: "", treeAge: 0, pricePerShare: 0, sharesLeft: 0, cover: "", location: "", sales: 0, benefits: [], weatherEui: "", cameraEui: "", traceCode: "" }),
      shares: 1,
      agreed: false,
      showAgree: false,
      // 托管协议（后台「协议管理」维护，type=custody）。content 为空时用页面内置兜底文案。
      agreeTplId: 0,
      agreeName: "茶树托管协议",
      agreeContent: "",
      agreeIsHtml: false,
      useBalance: true,
      paying: false,
      couponOff: 0
    };
  },
  computed: {
    subtotal() {
      return this.shares * this.g.pricePerShare;
    },
    payable() {
      const p = this.subtotal - this.couponOff;
      return p > 0 ? p : 0;
    },
    balanceUse() {
      if (!this.useBalance)
        return 0;
      return Math.min(this.balance, this.payable);
    },
    wechatPay() {
      return this.payable - this.balanceUse;
    }
  },
  onLoad(opt) {
    if (!utils_auth.isLogin()) {
      common_vendor.index.redirectTo({ url: "/pages/login/login?redirect=/pages/adoption/list" });
      return null;
    }
    const id = opt["id"];
    if (id != null) {
      api_adopt.gardenDetail(parseInt(id)).then((res) => {
        this.g = api_mappers.toGarden(res);
      }).catch((_e = null) => {
      });
    }
    const s = opt["shares"];
    if (s != null)
      this.shares = parseInt(s);
    api_user.userBalance().then((d) => {
      this.balance = api_mappers.numOf(d, ["nowMoney", "balance", "now_money"]);
    }).catch((_e = null) => {
    });
    api_agreement.agreementTemplate("custody").then((t) => {
      var _a, _b, _c;
      this.agreeTplId = (_a = t.getNumber("id")) !== null && _a !== void 0 ? _a : 0;
      const nm = (_b = t.getString("name")) !== null && _b !== void 0 ? _b : "";
      if (nm.length > 0)
        this.agreeName = nm;
      const content = (_c = t.getString("content")) !== null && _c !== void 0 ? _c : "";
      this.agreeContent = content;
      this.agreeIsHtml = content.indexOf("<") >= 0 && content.indexOf(">") >= 0;
    }).catch((_e = null) => {
    });
  },
  methods: {
    money(n) {
      return api_mappers.fmtMoney(n);
    },
    noop() {
    },
    agreeOk() {
      this.agreed = true;
      this.showAgree = false;
    },
    pay() {
      if (!this.agreed || this.paying)
        return null;
      this.paying = true;
      api_adopt.createAdopt(this.g.id, this.shares).then((rec) => {
        var _a, _b;
        const orderNo = (_a = rec.getString("orderNo")) !== null && _a !== void 0 ? _a : "";
        const recordId = (_b = rec.getNumber("id")) !== null && _b !== void 0 ? _b : 0;
        if (orderNo.length == 0) {
          this.paying = false;
          common_vendor.index.showToast({ title: "下单失败", icon: "none" });
          return null;
        }
        const payType = this.balanceUse >= this.payable ? "yue" : "weixin";
        api_adopt.payAdopt(orderNo, payType, "routine", this.balanceUse).then((r) => {
          var _a2;
          this.paying = false;
          const status = (_a2 = r.getString("status")) !== null && _a2 !== void 0 ? _a2 : "";
          if (payType == "yue" || status == "SUCCESS") {
            if (this.agreeTplId > 0) {
              api_agreement.signAgreement(this.agreeTplId, "adopt", orderNo).catch((_e = null) => {
              });
            }
            common_vendor.index.redirectTo({ url: "/pages/checkout/success?orderNo=" + orderNo + "&recordId=" + recordId + "&id=" + this.g.id + "&shares=" + this.shares });
          } else {
            common_vendor.index.showToast({ title: "认养订单已创建，请完成支付", icon: "none" });
            setTimeout(() => {
              common_vendor.index.redirectTo({ url: "/pages/my-garden/my-garden" });
            }, 800);
          }
        }).catch((_e = null) => {
          this.paying = false;
        });
      }).catch((_e = null) => {
        this.paying = false;
      });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_vendor.t($data.g.name),
    b: common_vendor.t($data.g.variety),
    c: common_vendor.t($data.g.altitude),
    d: common_vendor.t($options.money($data.g.pricePerShare)),
    e: common_vendor.t($data.shares),
    f: common_assets._imports_0$5,
    g: common_vendor.n($data.agreed ? "cb-on" : ""),
    h: common_vendor.t($data.agreeName),
    i: common_vendor.o(($event) => $data.showAgree = true, "b9"),
    j: common_vendor.o(($event) => $data.agreed = !$data.agreed, "39"),
    k: common_vendor.t($options.money($data.balance)),
    l: common_assets._imports_0$5,
    m: common_vendor.n($data.useBalance ? "cb-on" : ""),
    n: common_vendor.o(($event) => $data.useBalance = true, "6b"),
    o: $data.useBalance && $options.wechatPay > 0
  }, $data.useBalance && $options.wechatPay > 0 ? {
    p: common_vendor.t($options.money($options.wechatPay))
  } : {}, {
    q: common_assets._imports_0$5,
    r: common_vendor.n(!$data.useBalance ? "cb-on" : ""),
    s: common_vendor.o(($event) => $data.useBalance = false, "a1"),
    t: common_vendor.t($options.money($options.subtotal)),
    v: $data.couponOff > 0
  }, $data.couponOff > 0 ? {
    w: common_vendor.t($options.money($data.couponOff))
  } : {}, {
    x: $data.useBalance && $options.balanceUse > 0
  }, $data.useBalance && $options.balanceUse > 0 ? {
    y: common_vendor.t($options.money($options.balanceUse))
  } : {}, {
    z: $options.wechatPay > 0
  }, $options.wechatPay > 0 ? {
    A: common_vendor.t($options.money($options.wechatPay))
  } : {}, {
    B: common_vendor.t($options.money($options.payable)),
    C: common_vendor.t($options.money($options.payable)),
    D: common_vendor.t($data.paying ? "支付中…" : "确认支付"),
    E: common_vendor.n(!$data.agreed || $data.paying ? "btn-off" : ""),
    F: common_vendor.o((...args) => $options.pay && $options.pay(...args), "cc"),
    G: $data.showAgree
  }, $data.showAgree ? common_vendor.e({
    H: common_vendor.t($data.agreeName),
    I: common_assets._imports_1$3,
    J: common_vendor.o(($event) => $data.showAgree = false, "18"),
    K: $data.agreeContent.length > 0 && $data.agreeIsHtml
  }, $data.agreeContent.length > 0 && $data.agreeIsHtml ? {
    L: $data.agreeContent
  } : $data.agreeContent.length > 0 ? {
    N: common_vendor.t($data.agreeContent)
  } : {}, {
    M: $data.agreeContent.length > 0,
    O: common_vendor.o((...args) => $options.agreeOk && $options.agreeOk(...args), "ef"),
    P: common_vendor.o((...args) => $options.noop && $options.noop(...args), "e4"),
    Q: common_vendor.o(($event) => $data.showAgree = false, "58")
  }) : {}, {
    R: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    S: `${_ctx.u_s_b_h}px`,
    T: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
