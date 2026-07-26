"use strict";
const common_vendor = require("../../common/vendor.js");
const api_user = require("../../api/user.js");
const api_auth = require("../../api/auth.js");
const utils_auth = require("../../utils/auth.js");
const api_mappers = require("../../api/mappers.js");
const utils_assets = require("../../utils/assets.js");
const common_assets = require("../../common/assets.js");
class OrderType extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          label: { type: String, optional: false },
          icon: { type: String, optional: false },
          status: { type: Number, optional: false }
        };
      },
      name: "OrderType"
    };
  }
  constructor(options, metadata = OrderType.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.label = this.__props__.label;
    this.icon = this.__props__.icon;
    this.status = this.__props__.status;
    delete this.__props__;
  }
}
class PromiseItem extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          name: { type: String, optional: false },
          desc: { type: String, optional: false },
          icon: { type: String, optional: false }
        };
      },
      name: "PromiseItem"
    };
  }
  constructor(options, metadata = PromiseItem.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.name = this.__props__.name;
    this.desc = this.__props__.desc;
    this.icon = this.__props__.icon;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      statusBarH: 20,
      showService: false,
      logged: false,
      imgProfile: utils_assets.assetUrl("mine_profile", "/static/gen/mine-profile.jpg"),
      imgConsult: utils_assets.assetUrl("mine_consult", "/static/gen/mine-consult.jpg"),
      imgRecent: utils_assets.assetUrl("mine_recent", "/static/gen/mine-recent.jpg"),
      kefuTitle: "添加专属选茶师",
      kefuSub: "企业微信 1 对 1 服务 · 选茶 / 茶礼 / 售后咨询",
      kefuQr: "",
      promises: [
        new PromiseItem({ name: "正品保证", desc: "源头原产地", icon: "/static/icons/shield-d.png" }),
        new PromiseItem({ name: "安心包装", desc: "专业防护", icon: "/static/icons/gift-d.png" }),
        new PromiseItem({ name: "售后无忧", desc: "贴心服务", icon: "/static/icons/star-d.png" })
      ],
      orderTypes: [
        new OrderType({ label: "待付款", icon: "/static/icons/doc-d.png", status: 0 }),
        new OrderType({ label: "待发货", icon: "/static/icons/gift-d.png", status: 1 }),
        new OrderType({ label: "待收货", icon: "/static/icons/cart-d.png", status: 2 }),
        new OrderType({ label: "已完成", icon: "/static/icons/check-d.png", status: 4 })
      ],
      user: new api_mappers.UserVM({ nickname: "游客", avatar: "", balance: 0, integral: 0, coupons: 0, level: "" })
    };
  },
  onLoad() {
    this.statusBarH = common_vendor.index.getWindowInfo().statusBarHeight;
    this.imgProfile = utils_assets.assetUrl("mine_profile", "/static/gen/mine-profile.jpg");
    this.imgConsult = utils_assets.assetUrl("mine_consult", "/static/gen/mine-consult.jpg");
    this.imgRecent = utils_assets.assetUrl("mine_recent", "/static/gen/mine-recent.jpg");
  },
  onShow() {
    this.refresh();
  },
  methods: {
    noop() {
    },
    refresh() {
      this.logged = utils_auth.isLogin();
      if (!this.logged) {
        this.user = new api_mappers.UserVM({ nickname: "游客", avatar: "", balance: 0, integral: 0, coupons: 0, level: "" });
        return null;
      }
      api_user.userInfo().then((data) => {
        this.user = api_mappers.toUser(data);
      }).catch((_e = null) => {
      });
    },
    onName() {
      if (!this.logged)
        common_vendor.index.navigateTo({ url: "/pages/login/login" });
    },
    nav(url) {
      if (!this.logged) {
        common_vendor.index.navigateTo({ url: "/pages/login/login" });
        return null;
      }
      common_vendor.index.navigateTo({ url });
    },
    openService() {
      this.showService = true;
      api_user.workKefu().then((data) => {
        var _a, _b, _c;
        this.kefuTitle = (_a = data.getString("title")) !== null && _a !== void 0 ? _a : this.kefuTitle;
        this.kefuSub = (_b = data.getString("subtitle")) !== null && _b !== void 0 ? _b : this.kefuSub;
        this.kefuQr = (_c = data.getString("qrcodeImage")) !== null && _c !== void 0 ? _c : "";
      }).catch((_e = null) => {
      });
    },
    showFavorite() {
      if (!this.logged) {
        this.onName();
        return null;
      }
      common_vendor.index.showToast({ title: "收藏页正在接入", icon: "none" });
    },
    reserve() {
      common_vendor.index.showModal(new common_vendor.UTSJSONObject({ title: "预约到店品鉴", content: "请先联系客服登记到店人数和时间，我们会为你准备合适茶样。", showCancel: false }));
    },
    goGift() {
      common_vendor.index.switchTab({ url: "/pages/gift/gift" });
    },
    goChoose() {
      common_vendor.index.switchTab({ url: "/pages/choose/choose" });
    },
    goOrigin() {
      common_vendor.index.switchTab({ url: "/pages/origin/origin" });
    },
    doLogout() {
      api_auth.logout().then((_d) => {
        this.refresh();
        common_vendor.index.showToast({ title: "已退出", icon: "none" });
      }).catch((_e = null) => {
        this.refresh();
      });
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
    a: common_vendor.s("padding-top:" + ($data.statusBarH + 8) + "px"),
    b: $data.imgProfile,
    c: common_assets._imports_0$1,
    d: common_vendor.t($data.logged ? $data.user.nickname : "欢迎来到凤凰云岫"),
    e: common_vendor.t($data.logged ? "喝茶有分寸，生活有温度" : "茶不必说满，诚意会自己抵达。"),
    f: common_vendor.t($data.logged ? $data.user.level.length > 0 ? $data.user.level : "初识茶香" : "初识茶香"),
    g: common_vendor.o((...args) => $options.onName && $options.onName(...args), "96"),
    h: common_vendor.o(($event) => $options.nav("/pages/orders/orders"), "8a"),
    i: common_vendor.f($data.orderTypes, (item, i, i0) => {
      return {
        a: item.icon,
        b: common_vendor.t(item.label),
        c: i,
        d: common_vendor.o(($event) => $options.nav("/pages/orders/orders?status=" + item.status), i)
      };
    }),
    j: common_assets._imports_2$2,
    k: common_vendor.o((...args) => $options.showFavorite && $options.showFavorite(...args), "c0"),
    l: common_assets._imports_2$1,
    m: common_vendor.o(($event) => $options.nav("/pages/address/address"), "99"),
    n: common_assets._imports_5,
    o: common_vendor.o((...args) => $options.openService && $options.openService(...args), "3c"),
    p: common_assets._imports_2,
    q: common_vendor.o((...args) => $options.reserve && $options.reserve(...args), "04"),
    r: common_assets._imports_5$1,
    s: common_vendor.o((...args) => $options.goGift && $options.goGift(...args), "92"),
    t: common_assets._imports_1$1,
    v: common_vendor.o((...args) => $options.goOrigin && $options.goOrigin(...args), "c9"),
    w: common_vendor.o((...args) => $options.openService && $options.openService(...args), "6f"),
    x: common_vendor.o((...args) => $options.goChoose && $options.goChoose(...args), "a9"),
    y: common_vendor.f($data.promises, (item, i, i0) => {
      return {
        a: item.icon,
        b: common_vendor.t(item.name),
        c: common_vendor.t(item.desc),
        d: i
      };
    }),
    z: $data.imgConsult,
    A: common_vendor.o((...args) => $options.openService && $options.openService(...args), "04"),
    B: $data.imgRecent,
    C: common_vendor.o((...args) => $options.goChoose && $options.goChoose(...args), "16"),
    D: common_assets._imports_3,
    E: common_assets._imports_5$1,
    F: common_assets._imports_2$2,
    G: $data.logged
  }, $data.logged ? {
    H: common_vendor.t($data.user.coupons),
    I: common_vendor.o(($event) => $options.nav("/pages/coupons/coupons"), "4e"),
    J: common_vendor.t($data.user.integral),
    K: common_vendor.o(($event) => $options.nav("/pages/points/points"), "08"),
    L: common_vendor.t($data.user.balance),
    M: common_vendor.o(($event) => $options.nav("/pages/recharge/recharge"), "db")
  } : {}, {
    N: $data.logged
  }, $data.logged ? {
    O: common_vendor.o((...args) => $options.doLogout && $options.doLogout(...args), "2b")
  } : {}, {
    P: $data.showService
  }, $data.showService ? common_vendor.e({
    Q: common_vendor.t($data.kefuTitle),
    R: common_vendor.t($data.kefuSub),
    S: $data.kefuQr.length > 0
  }, $data.kefuQr.length > 0 ? {
    T: $data.kefuQr
  } : {
    U: common_assets._imports_5
  }, {
    V: common_vendor.o(($event) => $data.showService = false, "a0"),
    W: common_vendor.o((...args) => $options.noop && $options.noop(...args), "fd"),
    X: common_vendor.o(($event) => $data.showService = false, "be")
  }) : {}, {
    Y: common_vendor.p({
      current: 4
    }),
    Z: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    aa: `${_ctx.u_s_b_h}px`,
    ab: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
