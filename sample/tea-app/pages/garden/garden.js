"use strict";
const common_vendor = require("../../common/vendor.js");
const api_iot = require("../../api/iot.js");
const common_assets = require("../../common/assets.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      gardenId: 0,
      playUrl: "",
      temp: "18.6°C",
      humidity: "76%",
      light: "适中",
      altitude: "980m",
      updateTip: "数据每小时更新一次"
    };
  },
  onLoad(opt) {
    const gid = opt["gardenId"];
    if (gid != null)
      this.gardenId = parseInt(gid);
    if (this.gardenId > 0)
      this.load();
  },
  methods: {
    load() {
      api_iot.weatherLatest(this.gardenId).then((d) => {
        var _a, _b;
        const t = d.getNumber("temperature");
        if (t != null)
          this.temp = t + "°C";
        const h = d.getNumber("humidity");
        if (h != null)
          this.humidity = h + "%";
        const l = (_a = d.getString("light")) !== null && _a !== void 0 ? _a : "";
        if (l.length > 0)
          this.light = l;
        const a = d.getNumber("altitude");
        if (a != null)
          this.altitude = a + "m";
        const ut = (_b = d.getString("updateTime")) !== null && _b !== void 0 ? _b : "";
        if (ut.length > 0)
          this.updateTip = "更新于 " + ut;
      }).catch((_e = null) => {
      });
      api_iot.cameraPlay(this.gardenId).then((d) => {
        var _a, _b, _c;
        this.playUrl = (_a = d.getString("url")) !== null && _a !== void 0 ? _a : (_b = d.getString("playUrl")) !== null && _b !== void 0 ? _b : (_c = d.getString("hls")) !== null && _c !== void 0 ? _c : "";
      }).catch((_e = null) => {
      });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: $data.playUrl.length > 0
  }, $data.playUrl.length > 0 ? {
    b: $data.playUrl
  } : {
    c: common_assets._imports_0$11
  }, {
    d: common_vendor.t($data.temp),
    e: common_vendor.t($data.humidity),
    f: common_vendor.t($data.light),
    g: common_vendor.t($data.altitude),
    h: common_vendor.t($data.updateTip),
    i: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    j: `${_ctx.u_s_b_h}px`,
    k: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/garden/garden.js.map
