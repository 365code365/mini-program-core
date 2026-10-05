"use strict";
const common_vendor = require("../../common/vendor.js");
const utils_assets = require("../../utils/assets.js");
const BG_WAIT_MS = 3e3;
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      statusBarH: 20,
      countdown: 5,
      playing: true,
      timer: 0,
      bgImg: utils_assets.assetUrl("splash_bg", "/static/gen/splash.jpg"),
      // 启动页进入时页面栈里只有自己，只能 switchTab 去首页；
      // 从首页 hero 播放键进来时带 from=home，此时返回上一页即可。
      canBack: false,
      closing: false,
      // 倒计时是否已启动（背景图 load/error 或兜底超时后置为 true）
      started: false,
      guard: 0
    };
  },
  onLoad(opt) {
    this.statusBarH = common_vendor.index.getWindowInfo().statusBarHeight;
    this.bgImg = utils_assets.assetUrl("splash_bg", "/static/gen/splash.jpg");
    const from = opt["from"];
    this.canBack = from != null && "" + from == "home";
    this.guard = setTimeout(() => {
      this.start();
    }, BG_WAIT_MS);
  },
  onUnload() {
    this.stopTimers();
  },
  methods: {
    // 背景图加载完成（或加载失败）后才开始倒计时
    onBgReady() {
      this.start();
    },
    start() {
      if (this.started)
        return null;
      this.started = true;
      if (this.guard > 0) {
        clearTimeout(this.guard);
        this.guard = 0;
      }
      this.timer = setInterval(() => {
        if (!this.playing)
          return null;
        this.countdown--;
        if (this.countdown <= 0)
          this.close();
      }, 1e3);
    },
    stopTimers() {
      if (this.timer > 0) {
        clearInterval(this.timer);
        this.timer = 0;
      }
      if (this.guard > 0) {
        clearTimeout(this.guard);
        this.guard = 0;
      }
    },
    toggle() {
      this.playing = !this.playing;
    },
    close() {
      if (this.closing)
        return null;
      this.closing = true;
      this.stopTimers();
      if (this.canBack) {
        common_vendor.index.navigateBack(new common_vendor.UTSJSONObject({ fail: () => {
          common_vendor.index.switchTab({ url: "/pages/index/index" });
        } }));
      } else {
        common_vendor.index.switchTab({ url: "/pages/index/index" });
      }
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return {
    a: $data.bgImg,
    b: common_vendor.o((...args) => $options.onBgReady && $options.onBgReady(...args), "da"),
    c: common_vendor.o((...args) => $options.onBgReady && $options.onBgReady(...args), "4c"),
    d: $data.playing ? "/static/icons/pause-w.png" : "/static/icons/play-w.png",
    e: common_vendor.t($data.countdown),
    f: common_vendor.o((...args) => $options.toggle && $options.toggle(...args), "e0"),
    g: common_vendor.o((...args) => $options.close && $options.close(...args), "ca"),
    h: common_vendor.s("padding-top:" + ($data.statusBarH + 24) + "px"),
    i: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    j: `${_ctx.u_s_b_h}px`,
    k: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/splash/splash.js.map
