"use strict";
const common_vendor = require("../../common/vendor.js");
const api_auth = require("../../api/auth.js");
const common_assets = require("../../common/assets.js");
const USER_AGREEMENT = '欢迎使用「凤凰云岫」（以下简称"本平台"）。在注册或使用本平台服务前，请您仔细阅读并充分理解本协议各条款。\n\n一、服务内容\n本平台提供凤凰山古树茶认养、茶品商城、山泉订阅、全流程溯源、签到积分等服务。\n\n二、账号注册与使用\n1. 您应提供真实、准确的注册信息，并妥善保管账号与密码；\n2. 因账号保管不善造成的损失由您自行承担。\n\n三、认养服务\n1. 一份认养份额对应一棵茶树的年度产出（约 500g 干茶），认养周期自支付成功起 1 年；\n2. 约定产量为预估值，实际以当季采制为准；\n3. 平台将于约定时间内完成挂牌并上传实拍照片。\n\n四、订单与支付\n1. 商品/认养价格以下单时页面展示为准；\n2. 支持余额、微信等支付方式；\n3. 完成支付后订单生效，发货与配送按平台规则执行。\n\n五、用户行为规范\n您不得利用本平台从事任何违法违规活动，不得发布违法、侵权或不实信息。\n\n六、知识产权\n本平台内的文字、图片、标识等内容的知识产权均归凤凰云岫所有，未经许可不得使用。\n\n七、免责声明\n因不可抗力、第三方原因或您自身原因导致的损失，平台在法律允许范围内不承担责任。\n\n八、协议变更\n本平台有权根据法律法规及业务需要更新本协议，更新后将在平台内公示。\n\n本协议最终解释权归凤凰云岫所有。';
const PRIVACY_POLICY = '凤凰云岫（以下简称"我们"）非常重视您的个人信息与隐私保护。本政策说明我们如何收集、使用与保护您的信息。\n\n一、我们收集的信息\n1. 账号信息：手机号、昵称、头像；\n2. 交易信息：收货地址、订单、支付记录；\n3. 设备与日志：用于保障服务安全与体验优化的必要信息。\n\n二、信息的使用\n用于账号登录、订单履约、物流配送、客户服务、会员权益（积分/优惠券）发放等。\n\n三、信息的共享与披露\n除以下情形外，我们不会向第三方提供您的个人信息：\n1. 取得您的明确授权；\n2. 为完成支付、配送等服务所必需；\n3. 法律法规要求或主管机关依法要求。\n\n四、信息存储与安全\n我们采用加密传输、访问控制等措施保护您的信息，存储期限不超过实现目的所必需的时间。\n\n五、您的权利\n您有权查询、更正、删除您的个人信息，或注销账号。\n\n六、未成年人保护\n如您为未成年人，请在监护人指导下使用本平台。\n\n七、联系我们\n如对本政策有任何疑问，可通过平台内"企业微信客服"与我们联系。\n\n本政策最终解释权归凤凰云岫所有。';
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      mode: "mobile",
      account: "",
      password: "",
      phone: "",
      captcha: "",
      loading: false,
      counting: false,
      countdown: 60,
      redirect: "",
      agreed: false,
      showAgree: false,
      agreeTitle: "",
      agreeBody: ""
    };
  },
  onLoad(opt) {
    const r = opt["redirect"];
    if (r != null)
      this.redirect = r;
  },
  methods: {
    noop() {
    },
    openAgree(type) {
      if (type == "user") {
        this.agreeTitle = "用户协议";
        this.agreeBody = USER_AGREEMENT;
      } else {
        this.agreeTitle = "隐私政策";
        this.agreeBody = PRIVACY_POLICY;
      }
      this.showAgree = true;
    },
    agreeAndClose() {
      this.agreed = true;
      this.showAgree = false;
    },
    submit() {
      if (this.loading)
        return null;
      if (!this.agreed) {
        common_vendor.index.showToast({ title: "请先阅读并同意用户协议与隐私政策", icon: "none" });
        return null;
      }
      if (this.mode == "account") {
        if (this.account.length == 0 || this.password.length == 0) {
          common_vendor.index.showToast({ title: "请输入账号和密码", icon: "none" });
          return null;
        }
        this.loading = true;
        api_auth.loginAccount(this.account, this.password).then((_d) => {
          this.loading = false;
          this.onSuccess();
        }).catch((_e = null) => {
          this.loading = false;
        });
      } else {
        if (this.phone.length == 0 || this.captcha.length == 0) {
          common_vendor.index.showToast({ title: "请输入手机号和验证码", icon: "none" });
          return null;
        }
        this.loading = true;
        api_auth.loginMobile(this.phone, this.captcha).then((_d) => {
          this.loading = false;
          this.onSuccess();
        }).catch((_e = null) => {
          this.loading = false;
        });
      }
    },
    onSuccess() {
      common_vendor.index.showToast({ title: "登录成功", icon: "success" });
      setTimeout(() => {
        if (this.redirect.length > 0) {
          common_vendor.index.redirectTo({ url: this.redirect });
        } else {
          common_vendor.index.switchTab({ url: "/pages/mine/mine" });
        }
      }, 600);
    },
    getCode() {
      if (this.counting)
        return null;
      if (this.phone.length != 11) {
        common_vendor.index.showToast({ title: "请输入正确手机号", icon: "none" });
        return null;
      }
      api_auth.sendCode(this.phone).then((d) => {
        const code = d.getString("code");
        if (code != null && code.length > 0) {
          this.captcha = code;
          common_vendor.index.showToast({ title: "测试模式：验证码已自动填入", icon: "none" });
        } else {
          common_vendor.index.showToast({ title: "验证码已发送", icon: "none" });
        }
        this.startCountdown();
      }).catch((_e = null) => {
      });
    },
    startCountdown() {
      this.counting = true;
      this.countdown = 60;
      const timer = setInterval(() => {
        this.countdown--;
        if (this.countdown <= 0) {
          clearInterval(timer);
          this.counting = false;
        }
      }, 1e3);
    },
    // 微信「一键获取手机号」授权回调
    onWxPhone(e = null) {
      var _a, _b, _c;
      if (!this.agreed) {
        common_vendor.index.showToast({ title: "请先阅读并同意用户协议与隐私政策", icon: "none" });
        return null;
      }
      const detail = e.detail;
      const errMsg = (_a = detail.getString("errMsg")) !== null && _a !== void 0 ? _a : "";
      if (errMsg.indexOf("ok") < 0) {
        common_vendor.index.showToast({ title: "已取消微信授权", icon: "none" });
        return null;
      }
      const encryptedData = (_b = detail.getString("encryptedData")) !== null && _b !== void 0 ? _b : "";
      const iv = (_c = detail.getString("iv")) !== null && _c !== void 0 ? _c : "";
      if (encryptedData.length == 0 || iv.length == 0) {
        common_vendor.index.showToast({ title: "未获取到手机号，请用验证码登录", icon: "none" });
        return null;
      }
      if (this.loading)
        return null;
      this.loading = true;
      this.wxQuickLogin(encryptedData, iv);
    },
    // 第一步：uni.login 拿 code → 小程序授权登录
    wxQuickLogin(encryptedData, iv) {
      common_vendor.index.login(new common_vendor.UTSJSONObject({
        provider: "weixin",
        success: (res = null) => {
          var _a;
          const code = (_a = res["code"]) !== null && _a !== void 0 ? _a : "";
          if (code.length == 0) {
            this.loading = false;
            common_vendor.index.showToast({ title: "微信登录失败", icon: "none" });
            return null;
          }
          api_auth.programLogin(code, new common_vendor.UTSJSONObject({ nickName: "微信用户" })).then((d) => {
            var _a2;
            const type = (_a2 = d.getString("type")) !== null && _a2 !== void 0 ? _a2 : "";
            if (type == "login") {
              this.loading = false;
              this.onSuccess();
              return null;
            }
            const key = d.getString("key");
            if (key == null) {
              this.loading = false;
              common_vendor.index.showToast({ title: "登录异常，请重试", icon: "none" });
              return null;
            }
            this.wxBindPhone(key, encryptedData, iv);
          }).catch((_e = null) => {
            this.loading = false;
          });
        },
        fail: (_e = null) => {
          this.loading = false;
          common_vendor.index.showToast({ title: "微信登录失败", icon: "none" });
        }
      }));
    },
    // 第二步：新 code + 加密手机号 → 绑定手机号并自动登录
    wxBindPhone(key, encryptedData, iv) {
      common_vendor.index.login(new common_vendor.UTSJSONObject({
        provider: "weixin",
        success: (res = null) => {
          var _a;
          const code = (_a = res["code"]) !== null && _a !== void 0 ? _a : "";
          const body = new common_vendor.UTSJSONObject({ type: "routine", key, code, encryptedData, iv });
          api_auth.bindWxPhone(body).then((_d) => {
            this.loading = false;
            this.onSuccess();
          }).catch((_e = null) => {
            this.loading = false;
          });
        },
        fail: (_e = null) => {
          this.loading = false;
          common_vendor.index.showToast({ title: "微信登录失败", icon: "none" });
        }
      }));
    },
    skip() {
      common_vendor.index.switchTab({ url: "/pages/index/index" });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_vendor.n($data.mode == "mobile" ? "tab-on" : ""),
    b: common_vendor.o(($event) => $data.mode = "mobile", "5f"),
    c: common_vendor.n($data.mode == "account" ? "tab-on" : ""),
    d: common_vendor.o(($event) => $data.mode = "account", "c9"),
    e: $data.mode == "account"
  }, $data.mode == "account" ? {
    f: $data.account,
    g: common_vendor.o(($event) => $data.account = $event.detail.value, "38"),
    h: $data.password,
    i: common_vendor.o(($event) => $data.password = $event.detail.value, "81")
  } : {
    j: $data.phone,
    k: common_vendor.o(($event) => $data.phone = $event.detail.value, "41"),
    l: $data.captcha,
    m: common_vendor.o(($event) => $data.captcha = $event.detail.value, "dd"),
    n: common_vendor.t($data.counting ? $data.countdown + "s" : "获取验证码"),
    o: common_vendor.n($data.counting ? "code-btn-off" : ""),
    p: common_vendor.o((...args) => $options.getCode && $options.getCode(...args), "5f")
  }, {
    q: common_vendor.t($data.loading ? "登录中…" : "登 录"),
    r: common_vendor.n($data.loading ? "submit-off" : ""),
    s: common_vendor.o((...args) => $options.submit && $options.submit(...args), "e8"),
    t: common_vendor.o((...args) => $options.onWxPhone && $options.onWxPhone(...args), "fb"),
    v: common_vendor.o((...args) => $options.skip && $options.skip(...args), "d8"),
    w: $data.agreed
  }, $data.agreed ? {
    x: common_assets._imports_0$5
  } : {}, {
    y: common_vendor.n($data.agreed ? "cb-on" : ""),
    z: common_vendor.o(($event) => $data.agreed = !$data.agreed, "fd"),
    A: common_vendor.o(($event) => $options.openAgree("user"), "95"),
    B: common_vendor.o(($event) => $options.openAgree("privacy"), "91"),
    C: $data.showAgree
  }, $data.showAgree ? {
    D: common_vendor.t($data.agreeTitle),
    E: common_assets._imports_1$4,
    F: common_vendor.o(($event) => $data.showAgree = false, "d6"),
    G: common_vendor.t($data.agreeBody),
    H: common_vendor.o((...args) => $options.agreeAndClose && $options.agreeAndClose(...args), "8c"),
    I: common_vendor.o((...args) => $options.noop && $options.noop(...args), "51"),
    J: common_vendor.o(($event) => $data.showAgree = false, "9d")
  } : {}, {
    K: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    L: `${_ctx.u_s_b_h}px`,
    M: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
