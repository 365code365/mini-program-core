"use strict";
const common_vendor = require("../../common/vendor.js");
const api_adopt = require("../../api/adopt.js");
const api_mappers = require("../../api/mappers.js");
const config_env = require("../../config/env.js");
const common_assets = require("../../common/assets.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      g: new api_mappers.GardenVM({ id: 0, name: "", variety: "", altitude: 0, area: "", treeAge: 0, pricePerShare: 0, sharesLeft: 0, cover: "", location: "", sales: 0, benefits: [], weatherEui: "", cameraEui: "", traceCode: "" }),
      shares: 1,
      recordId: 0,
      orderNo: "",
      certNo: "",
      hash: "",
      nickname: "",
      message: "",
      submitted: false
    };
  },
  computed: {
    certImgUrl() {
      return config_env.BASE_URL + "/adopt/cert/image/" + this.orderNo;
    }
  },
  onLoad(opt) {
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
    const r = opt["recordId"];
    if (r != null)
      this.recordId = parseInt(r);
    const on = opt["orderNo"];
    if (on != null) {
      this.orderNo = on;
      api_adopt.verifyCert(this.orderNo).then((d) => {
        var _a, _b;
        this.certNo = (_a = d.getString("certNo")) !== null && _a !== void 0 ? _a : this.orderNo;
        this.hash = ((_b = d.getString("blockHash")) !== null && _b !== void 0 ? _b : "").substring(0, 12);
      }).catch((_e = null) => {
        this.certNo = this.orderNo;
      });
    }
  },
  methods: {
    previewCert() {
      if (this.orderNo.length == 0)
        return null;
      common_vendor.index.previewImage({ urls: [this.certImgUrl], current: 0 });
    },
    submit() {
      if (this.nickname.length == 0)
        return null;
      api_adopt.submitPlate(this.recordId, this.nickname, this.message).then((_d) => {
        this.submitted = true;
      }).catch((_e = null) => {
      });
    },
    toGarden() {
      common_vendor.index.navigateTo({ url: "/pages/my-garden/my-garden" });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_assets._imports_0$5,
    b: common_vendor.t($data.g.name),
    c: common_vendor.t($data.certNo),
    d: common_vendor.t($data.g.name),
    e: common_vendor.t($data.shares),
    f: common_vendor.t($data.shares * 500),
    g: common_vendor.t($data.orderNo),
    h: common_vendor.t($data.hash),
    i: $data.orderNo.length > 0
  }, $data.orderNo.length > 0 ? {
    j: $options.certImgUrl,
    k: common_vendor.o((...args) => $options.previewCert && $options.previewCert(...args), "dc")
  } : {}, {
    l: $data.orderNo.length > 0
  }, $data.orderNo.length > 0 ? {} : {}, {
    m: !$data.submitted
  }, !$data.submitted ? {
    n: $data.nickname,
    o: common_vendor.o(($event) => $data.nickname = $event.detail.value, "84"),
    p: $data.message,
    q: common_vendor.o(($event) => $data.message = $event.detail.value, "81"),
    r: common_vendor.n($data.nickname.length == 0 ? "btn-off" : ""),
    s: common_vendor.o((...args) => $options.submit && $options.submit(...args), "0e")
  } : {
    t: common_vendor.o((...args) => $options.toGarden && $options.toGarden(...args), "fa")
  }, {
    v: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    w: `${_ctx.u_s_b_h}px`,
    x: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
