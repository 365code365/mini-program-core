"use strict";
const common_vendor = require("../../common/vendor.js");
const api_user = require("../../api/user.js");
const api_mappers = require("../../api/mappers.js");
const utils_auth = require("../../utils/auth.js");
class Preset extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          amount: { type: Number, optional: false },
          bonus: { type: Number, optional: false },
          id: { type: Number, optional: false }
        };
      },
      name: "Preset"
    };
  }
  constructor(options, metadata = Preset.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.amount = this.__props__.amount;
    this.bonus = this.__props__.bonus;
    this.id = this.__props__.id;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      balance: 0,
      sel: 0,
      paying: false,
      custom: false,
      customAmount: "",
      presets: [
        new Preset({ amount: 100, bonus: 0, id: 0 }),
        new Preset({ amount: 300, bonus: 10, id: 0 }),
        new Preset({ amount: 500, bonus: 30, id: 0 }),
        new Preset({ amount: 1e3, bonus: 80, id: 0 }),
        new Preset({ amount: 2e3, bonus: 200, id: 0 }),
        new Preset({ amount: 5e3, bonus: 600, id: 0 })
      ]
    };
  },
  computed: {
    payAmount() {
      if (this.custom) {
        const v = parseFloat(this.customAmount);
        return v == v && v > 0 ? v : 0;
      }
      return this.presets[this.sel].amount;
    }
  },
  onLoad() {
    if (!utils_auth.isLogin()) {
      common_vendor.index.redirectTo({ url: "/pages/login/login?redirect=/pages/recharge/recharge" });
      return null;
    }
    this.refresh();
  },
  methods: {
    money(n) {
      return api_mappers.fmtMoney(n);
    },
    pick(i) {
      this.custom = false;
      this.sel = i;
    },
    pickCustom() {
      this.custom = true;
    },
    refresh() {
      api_user.userBalance().then((d) => {
        this.balance = api_mappers.numOf(d, ["nowMoney", "balance", "now_money"]);
      }).catch((_e = null) => {
      });
      api_user.rechargeIndex().then((d) => {
        const quota = d["rechargeQuota"];
        if (quota != null && Array.isArray(quota)) {
          const arr = quota;
          const ps = [];
          for (let i = 0; i < arr.length; i++) {
            const q = arr[i];
            ps.push(new Preset({
              amount: api_mappers.numOf(q, ["price"]),
              bonus: api_mappers.numOf(q, ["giveMoney", "give_money"]),
              id: api_mappers.numOf(q, ["id"])
            }));
          }
          if (ps.length > 0) {
            this.presets = ps;
            this.sel = 0;
          }
        }
      }).catch((_e = null) => {
      });
    },
    recharge() {
      if (this.paying)
        return null;
      let price = 0;
      let id = 0;
      if (this.custom) {
        price = this.payAmount;
        id = 0;
        if (price <= 0) {
          common_vendor.index.showToast({ title: "请输入充值金额", icon: "none" });
          return null;
        }
      } else {
        const p = this.presets[this.sel];
        price = p.amount;
        id = p.id;
      }
      this.paying = true;
      api_user.rechargeRoutine(price, id).then((_d) => {
        this.paying = false;
        common_vendor.index.showToast({ title: "充值订单已创建，请完成支付", icon: "none" });
        setTimeout(() => {
          this.refresh();
        }, 1e3);
      }).catch((_e = null) => {
        this.paying = false;
      });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_vendor.t($options.money($data.balance)),
    b: common_vendor.f($data.presets, (p, i, i0) => {
      return common_vendor.e({
        a: common_vendor.t(p.amount),
        b: p.bonus > 0
      }, p.bonus > 0 ? {
        c: common_vendor.t(p.bonus)
      } : {}, {
        d: i,
        e: common_vendor.n(!$data.custom && $data.sel == i ? "cell-on" : ""),
        f: common_vendor.o(($event) => $options.pick(i), i)
      });
    }),
    c: common_vendor.n($data.custom ? "cell-on" : ""),
    d: common_vendor.o((...args) => $options.pickCustom && $options.pickCustom(...args), "b5"),
    e: $data.custom
  }, $data.custom ? {
    f: $data.customAmount,
    g: common_vendor.o(($event) => $data.customAmount = $event.detail.value, "e0")
  } : {}, {
    h: common_vendor.t($options.money($options.payAmount)),
    i: common_vendor.t($data.paying ? "充值中…" : "微信支付充值"),
    j: common_vendor.n($data.paying ? "btn-off" : ""),
    k: common_vendor.o((...args) => $options.recharge && $options.recharge(...args), "85"),
    l: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    m: `${_ctx.u_s_b_h}px`,
    n: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
