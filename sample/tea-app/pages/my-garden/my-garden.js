"use strict";
const common_vendor = require("../../common/vendor.js");
const api_adopt = require("../../api/adopt.js");
require("../../api/models.js");
const api_mappers = require("../../api/mappers.js");
const utils_auth = require("../../utils/auth.js");
const config_env = require("../../config/env.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return { list: [], loading: false };
  },
  onLoad() {
    if (!utils_auth.isLogin()) {
      common_vendor.index.redirectTo({ url: "/pages/login/login?redirect=/pages/my-garden/my-garden" });
      return null;
    }
  },
  onShow() {
    if (utils_auth.isLogin())
      this.load();
  },
  methods: {
    load() {
      this.loading = true;
      api_adopt.myList(1, 20).then((res) => {
        const vms = [];
        for (let i = 0; i < res.list.length; i++)
          vms.push(api_mappers.toAdoptRecord(res.list[i]));
        this.list = vms;
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    nav(url) {
      common_vendor.index.navigateTo({ url });
    },
    goPlate(g) {
      if (g.id <= 0) {
        common_vendor.index.showToast({ title: "认养记录无效", icon: "none" });
        return null;
      }
      common_vendor.index.navigateTo({ url: "/pages/plate/plate?recordId=" + g.id + "&gardenName=" + encodeURIComponent(g.gardenName) });
    },
    viewCert(orderNo) {
      if (orderNo.length == 0) {
        common_vendor.index.showToast({ title: "暂无认养证", icon: "none" });
        return null;
      }
      const url = config_env.BASE_URL + "/adopt/cert/image/" + orderNo;
      common_vendor.index.previewImage({ urls: [url], current: 0 });
    },
    goAdopt() {
      common_vendor.index.navigateTo({ url: "/pages/adoption/list" });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: $data.loading && $data.list.length == 0
  }, $data.loading && $data.list.length == 0 ? {} : {}, {
    b: !$data.loading && $data.list.length == 0
  }, !$data.loading && $data.list.length == 0 ? {
    c: common_vendor.o((...args) => $options.goAdopt && $options.goAdopt(...args), "c3")
  } : {}, {
    d: common_vendor.f($data.list, (g, i, i0) => {
      return common_vendor.e({
        a: g.cover.length > 0 ? g.cover : "/static/slices/category-origin/card-pingkengkou.jpg",
        b: common_vendor.t(g.statusText.length > 0 ? g.statusText : "认养中"),
        c: common_vendor.o(($event) => $options.goPlate(g), i),
        d: common_vendor.t(g.gardenName),
        e: common_vendor.t(g.shares),
        f: common_vendor.t(g.period),
        g: common_vendor.t(g.batchNo),
        h: common_vendor.t(g.plateName.length > 0 ? "已完成 ›" : "去完善 ›"),
        i: common_vendor.o(($event) => $options.goPlate(g), i),
        j: g.plateName.length > 0
      }, g.plateName.length > 0 ? {
        k: common_vendor.t(g.plateName),
        l: common_vendor.t(g.plateMsg)
      } : {}, {
        m: common_vendor.o(($event) => $options.nav("/pages/trace/trace?batch=" + g.batchNo), i),
        n: common_vendor.o(($event) => $options.nav("/pages/garden/garden"), i),
        o: g.statusText == "认养中" || g.statusText == "已到期"
      }, g.statusText == "认养中" || g.statusText == "已到期" ? {
        p: common_vendor.o(($event) => $options.viewCert(g.orderNo), i)
      } : {}, {
        q: i
      });
    }),
    e: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    f: `${_ctx.u_s_b_h}px`,
    g: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
