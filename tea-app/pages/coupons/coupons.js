"use strict";
const common_vendor = require("../../common/vendor.js");
const api_marketing = require("../../api/marketing.js");
require("../../api/models.js");
const api_mappers = require("../../api/mappers.js");
const utils_auth = require("../../utils/auth.js");
const common_assets = require("../../common/assets.js");
class ShareRec extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          receiveUid: { type: Number, optional: false },
          createTime: { type: String, optional: false },
          status: { type: Number, optional: false },
          rewardStatus: { type: Number, optional: false },
          rewardIntegral: { type: Number, optional: false }
        };
      },
      name: "ShareRec"
    };
  }
  constructor(options, metadata = ShareRec.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.receiveUid = this.__props__.receiveUid;
    this.createTime = this.__props__.createTime;
    this.status = this.__props__.status;
    this.rewardStatus = this.__props__.rewardStatus;
    this.rewardIntegral = this.__props__.rewardIntegral;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      tab: 0,
      tabs: ["可使用", "品鉴券", "已失效"],
      coupons: [],
      vouchers: [],
      loading: false,
      showShare: false,
      sharing: false,
      shareCodeVal: "",
      shareMsg: "",
      showRedeem: false,
      redeemCode: "",
      redeeming: false,
      showShareList: false,
      shareList: []
    };
  },
  onLoad() {
    if (!utils_auth.isLogin()) {
      common_vendor.index.redirectTo({ url: "/pages/login/login?redirect=/pages/coupons/coupons" });
      return null;
    }
    this.loadCoupons(1);
  },
  methods: {
    noop() {
    },
    switchTab(i) {
      this.tab = i;
      if (i == 1) {
        this.loadVouchers();
      } else {
        this.loadCoupons(i == 0 ? 1 : 3);
      }
    },
    loadCoupons(type) {
      this.loading = true;
      api_marketing.myCoupons(type, 1, 50).then((res) => {
        const vms = [];
        for (let i = 0; i < res.list.length; i++)
          vms.push(api_mappers.toCoupon(res.list[i]));
        this.coupons = vms;
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    loadVouchers() {
      this.loading = true;
      api_marketing.voucherList(1, 50).then((res) => {
        const vms = [];
        for (let i = 0; i < res.list.length; i++)
          vms.push(api_mappers.toVoucher(res.list[i]));
        this.vouchers = vms;
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    share() {
      if (this.sharing)
        return null;
      this.sharing = true;
      api_marketing.shareCode().then((code) => {
        this.sharing = false;
        this.shareCodeVal = code;
        this.shareMsg = "我在【云茶园】送你一张古树茶品鉴券，口令：" + code + "。打开小程序进入「卡券中心 - 品鉴券」输入口令即可领取~";
        this.showShare = true;
      }).catch((_e = null) => {
        this.sharing = false;
      });
    },
    copyShare() {
      common_vendor.index.setClipboardData({
        data: this.shareMsg,
        success: () => {
          common_vendor.index.showToast({ title: "口令已复制，去发给好友吧", icon: "none" });
        }
      });
    },
    openRedeem() {
      this.redeemCode = "";
      this.showRedeem = true;
    },
    useVoucher(voucherId) {
      common_vendor.index.showModal({
        title: "核销品鉴券",
        content: "确认现在核销这张品鉴券？核销后不可撤销",
        success: (res) => {
          if (res.confirm) {
            api_marketing.voucherUse(voucherId).then((_r) => {
              common_vendor.index.showToast({ title: "核销成功", icon: "success" });
              this.loadVouchers();
            }).catch((_e = null) => {
            });
          }
        }
      });
    },
    openShareList() {
      this.showShareList = true;
      api_marketing.shareRecords(1, 50).then((res) => {
        var _a;
        const arr = [];
        for (let i = 0; i < res.list.length; i++) {
          const r = res.list[i];
          arr.push(new ShareRec({
            receiveUid: api_mappers.numOf(r, ["receiveUid"]),
            createTime: (_a = r.getString("createTime")) !== null && _a !== void 0 ? _a : "",
            status: api_mappers.numOf(r, ["status"]),
            rewardStatus: api_mappers.numOf(r, ["rewardStatus"]),
            rewardIntegral: api_mappers.numOf(r, ["rewardIntegral"])
          }));
        }
        this.shareList = arr;
      }).catch((_e = null) => {
      });
    },
    doRedeem() {
      if (this.redeeming)
        return null;
      const c = this.redeemCode.trim();
      if (c.length == 0) {
        common_vendor.index.showToast({ title: "请输入分享口令", icon: "none" });
        return null;
      }
      this.redeeming = true;
      api_marketing.shareReceive(c).then((_r) => {
        this.redeeming = false;
        this.showRedeem = false;
        common_vendor.index.showToast({ title: "领取成功", icon: "success" });
        this.loadVouchers();
      }).catch((_e = null) => {
        this.redeeming = false;
      });
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
        c: common_vendor.n($data.tab == i ? "tab-on" : ""),
        d: common_vendor.o(($event) => $options.switchTab(i), i)
      };
    }),
    b: $data.tab == 1
  }, $data.tab == 1 ? common_vendor.e({
    c: common_vendor.o((...args) => $options.openRedeem && $options.openRedeem(...args), "a1"),
    d: common_vendor.o((...args) => $options.openShareList && $options.openShareList(...args), "db"),
    e: $data.vouchers.length == 0 && !$data.loading
  }, $data.vouchers.length == 0 && !$data.loading ? {} : {}, {
    f: common_vendor.f($data.vouchers, (c, i, i0) => {
      return common_vendor.e({
        a: common_vendor.t(c.name),
        b: common_vendor.t(c.desc),
        c: c.status == 0
      }, c.status == 0 ? {
        d: common_vendor.o(($event) => $options.useVoucher(c.id), i)
      } : {}, {
        e: common_vendor.o((...args) => $options.share && $options.share(...args), i),
        f: i
      });
    }),
    g: common_assets._imports_0$14
  }) : common_vendor.e({
    h: $data.coupons.length == 0 && !$data.loading
  }, $data.coupons.length == 0 && !$data.loading ? {} : {}, {
    i: common_vendor.f($data.coupons, (c, i, i0) => {
      return {
        a: common_vendor.t(c.amount),
        b: common_vendor.t(c.cond),
        c: common_vendor.t(c.name),
        d: common_vendor.t(c.expire),
        e: i
      };
    })
  }), {
    j: $data.showShare
  }, $data.showShare ? {
    k: common_vendor.t($data.shareCodeVal),
    l: $data.shareMsg,
    m: common_vendor.o(($event) => $data.showShare = false, "03"),
    n: common_vendor.o((...args) => $options.copyShare && $options.copyShare(...args), "52"),
    o: common_vendor.o((...args) => $options.noop && $options.noop(...args), "22"),
    p: common_vendor.o(($event) => $data.showShare = false, "95")
  } : {}, {
    q: $data.showRedeem
  }, $data.showRedeem ? {
    r: $data.redeemCode,
    s: common_vendor.o(($event) => $data.redeemCode = $event.detail.value, "34"),
    t: common_vendor.o(($event) => $data.showRedeem = false, "1b"),
    v: common_vendor.t($data.redeeming ? "领取中…" : "立即领取"),
    w: common_vendor.o((...args) => $options.doRedeem && $options.doRedeem(...args), "ea"),
    x: common_vendor.o((...args) => $options.noop && $options.noop(...args), "28"),
    y: common_vendor.o(($event) => $data.showRedeem = false, "e1")
  } : {}, {
    z: $data.showShareList
  }, $data.showShareList ? common_vendor.e({
    A: common_vendor.f($data.shareList, (r, i, i0) => {
      return {
        a: common_vendor.t(r.receiveUid),
        b: common_vendor.t(r.createTime),
        c: common_vendor.t(r.rewardStatus == 1 ? "已奖励 +" + r.rewardIntegral + "积分" : r.status == 1 ? "已领取" : "待完成"),
        d: common_vendor.n(r.rewardStatus == 1 ? "sl-done" : ""),
        e: i
      };
    }),
    B: $data.shareList.length == 0
  }, $data.shareList.length == 0 ? {} : {}, {
    C: common_vendor.o(($event) => $data.showShareList = false, "58"),
    D: common_vendor.o((...args) => $options.noop && $options.noop(...args), "9d"),
    E: common_vendor.o(($event) => $data.showShareList = false, "8b")
  }) : {}, {
    F: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    G: `${_ctx.u_s_b_h}px`,
    H: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
