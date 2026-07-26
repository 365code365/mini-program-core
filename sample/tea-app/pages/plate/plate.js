"use strict";
const common_vendor = require("../../common/vendor.js");
const api_adopt = require("../../api/adopt.js");
const utils_auth = require("../../utils/auth.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      recordId: 0,
      gardenName: "",
      nickname: "",
      message: "",
      done: false,
      loading: false
    };
  },
  onLoad(opt) {
    var _a;
    if (!utils_auth.isLogin()) {
      common_vendor.index.redirectTo({ url: "/pages/login/login" });
      return null;
    }
    const rid = opt["recordId"];
    if (rid != null)
      this.recordId = parseInt("" + rid);
    const gn = opt["gardenName"];
    if (gn != null)
      this.gardenName = (_a = decodeURIComponent("" + gn)) !== null && _a !== void 0 ? _a : "";
    if (this.recordId > 0)
      this.loadInfo();
  },
  methods: {
    loadInfo() {
      api_adopt.plateInfo(this.recordId).then((d = null) => {
        if (d == null)
          return null;
        const nk = d.getString("nickname");
        if (nk != null)
          this.nickname = nk;
        const msg = d.getString("message");
        if (msg != null)
          this.message = msg;
        const st = d.getNumber("status");
        this.done = st != null && st == 1;
      }).catch((_e = null) => {
      });
    },
    submit() {
      if (this.loading)
        return null;
      if (this.done) {
        common_vendor.index.showToast({ title: "挂牌已完成，不可修改", icon: "none" });
        return null;
      }
      if (this.recordId <= 0) {
        common_vendor.index.showToast({ title: "认养记录无效", icon: "none" });
        return null;
      }
      const nk = this.nickname.trim();
      if (nk.length == 0) {
        common_vendor.index.showToast({ title: "请填写挂牌名", icon: "none" });
        return null;
      }
      this.loading = true;
      api_adopt.submitPlate(this.recordId, nk, this.message.trim()).then((_d) => {
        this.loading = false;
        common_vendor.index.showToast({ title: "挂牌信息已提交", icon: "success" });
        setTimeout(() => {
          common_vendor.index.navigateBack();
        }, 700);
      }).catch((_e = null) => {
        this.loading = false;
      });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: common_vendor.t($data.gardenName.length > 0 ? $data.gardenName : "我的古树茶园"),
    b: common_vendor.t($data.done ? "已挂牌" : "待挂牌"),
    c: common_vendor.n($data.done ? "h-tag-done" : ""),
    d: common_vendor.t($data.nickname.length > 0 ? $data.nickname : "请填写挂牌名"),
    e: common_vendor.t($data.message.length > 0 ? $data.message : "一棵树，长了一百年，我们只是把它分享给更爱茶的人"),
    f: $data.nickname,
    g: common_vendor.o(($event) => $data.nickname = $event.detail.value, "5b"),
    h: common_vendor.t($data.nickname.length),
    i: $data.message,
    j: common_vendor.o(($event) => $data.message = $event.detail.value, "26"),
    k: common_vendor.t($data.message.length),
    l: common_vendor.t($data.loading ? "提交中…" : $data.done ? "挂牌已完成" : "提交挂牌"),
    m: common_vendor.n($data.loading || $data.done ? "submit-off" : ""),
    n: common_vendor.o((...args) => $options.submit && $options.submit(...args), "3b"),
    o: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    p: `${_ctx.u_s_b_h}px`,
    q: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
