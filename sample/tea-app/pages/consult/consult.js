"use strict";
const common_vendor = require("../../common/vendor.js");
const api_consult = require("../../api/consult.js");
const utils_auth = require("../../utils/auth.js");
const api_user = require("../../api/user.js");
const common_assets = require("../../common/assets.js");
class DateOpt extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          offset: { type: Number, optional: false },
          label: { type: String, optional: false },
          date: { type: String, optional: false }
        };
      },
      name: "DateOpt"
    };
  }
  constructor(options, metadata = DateOpt.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.offset = this.__props__.offset;
    this.label = this.__props__.label;
    this.date = this.__props__.date;
    delete this.__props__;
  }
}
class TimeOpt extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          label: { type: String, optional: false },
          value: { type: String, optional: false }
        };
      },
      name: "TimeOpt"
    };
  }
  constructor(options, metadata = TimeOpt.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.label = this.__props__.label;
    this.value = this.__props__.value;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      scene: "consult",
      sourcePage: "",
      intentType: "personal",
      productId: 0,
      fName: "",
      fPhone: "",
      fWechat: "",
      fCompany: "",
      fBudget: "",
      fQuantity: "",
      fDemand: "",
      dayOffset: -1,
      timeSlot: "",
      submitting: false,
      timeOpts: [
        new TimeOpt({ label: "上午", value: "10:00" }),
        new TimeOpt({ label: "下午", value: "14:00" }),
        new TimeOpt({ label: "傍晚", value: "17:00" })
      ]
    };
  },
  computed: {
    needAppoint() {
      return this.scene == "appointment";
    },
    showCompany() {
      return this.scene == "enterprise";
    },
    showBudgetQty() {
      return this.scene == "gift" || this.scene == "enterprise";
    },
    sceneTitle() {
      if (this.scene == "appointment")
        return "预约到店品鉴";
      if (this.scene == "enterprise")
        return "企业茶礼咨询";
      if (this.scene == "gift")
        return "茶礼方案咨询";
      return "咨询选茶师";
    },
    sceneIntro() {
      if (this.scene == "appointment")
        return "选一个方便的时间，我们提前备好合适茶样，到店专人接待。";
      if (this.scene == "enterprise")
        return "批量采购、企业定制、节日客户维护，留个联系方式，专属顾问为你对接。";
      if (this.scene == "gift")
        return "告诉我们送礼对象、关系与预算，帮你选一份送得得体的茶礼。";
      return "说说你的饮茶习惯与偏好，选茶师帮你挑到合适的那一杯。";
    },
    demandPlaceholder() {
      if (this.scene == "appointment")
        return "想品鉴的茶类、到店人数等，可在此备注";
      if (this.scene == "enterprise")
        return "用途、交付时间、是否需要定制包装等";
      if (this.scene == "gift")
        return "送礼对象、关系、预算与忌讳等";
      return "口感偏好、预算、饮茶阶段等";
    },
    dateOpts() {
      const labels = ["今天", "明天", "后天"];
      const arr = [];
      for (let i = 0; i < 3; i++) {
        const full = this.dateStr(i);
        arr.push(new DateOpt({ offset: i, label: labels[i], date: full.substring(5) }));
      }
      return arr;
    }
  },
  onLoad(opt) {
    const s = opt["scene"];
    if (s != null && s.length > 0) {
      this.scene = s;
    }
    const src = opt["source"];
    if (src != null) {
      this.sourcePage = src;
    }
    const pid = opt["productId"];
    if (pid != null) {
      const n = parseInt(pid);
      this.productId = n == n ? n : 0;
    }
    this.intentType = this.scene == "enterprise" ? "enterprise" : "personal";
    common_vendor.index.setNavigationBarTitle({ title: this.sceneTitle });
    if (utils_auth.isLogin()) {
      this.prefill();
    }
  },
  methods: {
    // 手机号/座机宽松校验：只用已验证支持的 string 方法(substring/indexOf)，
    // 规避 uts RegExp 的跨端不确定性；后端 service 层有正则兜底，这里只做体验层拦截。
    isPhoneLike(s) {
      if (s.length < 5 || s.length > 20)
        return false;
      const allowed = "0123456789+-() ";
      const digitset = "0123456789";
      let digits = 0;
      for (let i = 0; i < s.length; i++) {
        const ch = s.substring(i, i + 1);
        if (allowed.indexOf(ch) < 0)
          return false;
        if (digitset.indexOf(ch) >= 0)
          digits = digits + 1;
      }
      return digits >= 5;
    },
    pad(n) {
      return n < 10 ? "0" + n : "" + n;
    },
    /** 返回距今 offset 天的日期，格式 yyyy-MM-dd */
    dateStr(offset) {
      const d = new Date(Date.now() + offset * 864e5);
      return d.getFullYear() + "-" + this.pad(d.getMonth() + 1) + "-" + this.pad(d.getDate());
    },
    pickDate(offset) {
      this.dayOffset = offset;
    },
    pickTime(value) {
      this.timeSlot = value;
    },
    /** 已登录时预填手机号，减少用户输入 */
    prefill() {
      api_user.userInfo().then((d) => {
        const p = d.getString("phone");
        if (p != null && p.length > 0 && this.fPhone.length == 0) {
          this.fPhone = p;
        }
      }).catch((_e = null) => {
      });
    },
    submit() {
      if (this.submitting)
        return null;
      const phone = this.fPhone.trim();
      const wechat = this.fWechat.trim();
      if (phone.length == 0 && wechat.length == 0) {
        common_vendor.index.showToast({ title: "请至少填手机号或微信号", icon: "none" });
        return null;
      }
      if (phone.length > 0 && !this.isPhoneLike(phone)) {
        common_vendor.index.showToast({ title: "手机号格式不正确", icon: "none" });
        return null;
      }
      let appoint = "";
      if (this.needAppoint) {
        if (this.dayOffset < 0 || this.timeSlot.length == 0) {
          common_vendor.index.showToast({ title: "请选择期望到店时间", icon: "none" });
          return null;
        }
        appoint = this.dateStr(this.dayOffset) + " " + this.timeSlot + ":00";
      }
      const body = new common_vendor.UTSJSONObject({
        scene: this.scene,
        intentType: this.intentType,
        sourcePage: this.sourcePage,
        contactName: this.fName.trim(),
        phone,
        wechat,
        demand: this.fDemand.trim()
      });
      if (this.showCompany) {
        body["company"] = this.fCompany.trim();
      }
      if (this.showBudgetQty) {
        body["budget"] = this.fBudget.trim();
        body["quantity"] = this.fQuantity.trim();
      }
      if (appoint.length > 0) {
        body["appointTime"] = appoint;
      }
      if (this.productId > 0) {
        body["productId"] = this.productId;
      }
      this.submitting = true;
      api_consult.submitConsult(body).then((_msg) => {
        this.submitting = false;
        common_vendor.index.showToast({ title: "提交成功", icon: "success" });
        setTimeout(() => {
          common_vendor.index.navigateBack(new common_vendor.UTSJSONObject({ delta: 1 }));
        }, 1e3);
      }).catch((_e = null) => {
        this.submitting = false;
      });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_vendor.t($options.sceneTitle),
    b: common_vendor.t($options.sceneIntro),
    c: $data.fName,
    d: common_vendor.o(($event) => $data.fName = $event.detail.value, "27"),
    e: $data.fPhone,
    f: common_vendor.o(($event) => $data.fPhone = $event.detail.value, "70"),
    g: $data.fWechat,
    h: common_vendor.o(($event) => $data.fWechat = $event.detail.value, "c5"),
    i: $options.needAppoint
  }, $options.needAppoint ? {
    j: common_vendor.f($options.dateOpts, (d, i, i0) => {
      return {
        a: common_vendor.t(d.label),
        b: common_vendor.n($data.dayOffset == d.offset ? "chip-t-on" : ""),
        c: common_vendor.t(d.date),
        d: common_vendor.n($data.dayOffset == d.offset ? "chip-t-on" : ""),
        e: i,
        f: common_vendor.n($data.dayOffset == d.offset ? "chip-on" : ""),
        g: common_vendor.o(($event) => $options.pickDate(d.offset), i)
      };
    }),
    k: common_vendor.f($data.timeOpts, (t, i, i0) => {
      return {
        a: common_vendor.t(t.label),
        b: common_vendor.n($data.timeSlot == t.value ? "chip-t-on" : ""),
        c: common_vendor.t(t.value),
        d: common_vendor.n($data.timeSlot == t.value ? "chip-t-on" : ""),
        e: i,
        f: common_vendor.n($data.timeSlot == t.value ? "chip-on" : ""),
        g: common_vendor.o(($event) => $options.pickTime(t.value), i)
      };
    })
  } : {}, {
    l: $options.showCompany
  }, $options.showCompany ? {
    m: $data.fCompany,
    n: common_vendor.o(($event) => $data.fCompany = $event.detail.value, "f6")
  } : {}, {
    o: $options.showBudgetQty
  }, $options.showBudgetQty ? {
    p: $data.fBudget,
    q: common_vendor.o(($event) => $data.fBudget = $event.detail.value, "b3"),
    r: $data.fQuantity,
    s: common_vendor.o(($event) => $data.fQuantity = $event.detail.value, "25")
  } : {}, {
    t: $options.demandPlaceholder,
    v: $data.fDemand,
    w: common_vendor.o(($event) => $data.fDemand = $event.detail.value, "42"),
    x: common_assets._imports_0$2,
    y: common_vendor.t($data.submitting ? "提交中…" : "提交"),
    z: common_vendor.n($data.submitting ? "submit-off" : ""),
    A: common_vendor.o((...args) => $options.submit && $options.submit(...args), "75"),
    B: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    C: `${_ctx.u_s_b_h}px`,
    D: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/consult/consult.js.map
