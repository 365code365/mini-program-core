"use strict";
const common_vendor = require("../../common/vendor.js");
const api_product = require("../../api/product.js");
require("../../api/models.js");
const api_mappers = require("../../api/mappers.js");
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      keyword: "",
      list: [],
      page: 1,
      limit: 10,
      loading: false,
      noMore: false
    };
  },
  onLoad(opt) {
    const keyword = opt["keyword"];
    if (keyword != null)
      this.keyword = keyword;
    this.load(true);
  },
  methods: {
    load(reset) {
      if (this.loading)
        return null;
      if (reset) {
        this.page = 1;
        this.noMore = false;
      }
      this.loading = true;
      api_product.productList(this.keyword, 0, this.page, this.limit).then((res) => {
        const vms = api_mappers.toProductList(res.list);
        if (reset) {
          this.list = vms;
        } else {
          this.list = this.list.concat(vms);
        }
        if (vms.length < this.limit)
          this.noMore = true;
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    loadMore() {
      if (this.noMore || this.loading)
        return null;
      this.page++;
      this.load(false);
    },
    open(id) {
      common_vendor.index.navigateTo({ url: "/pages/shop/detail?id=" + id });
    },
    add(p) {
      common_vendor.index.navigateTo({ url: "/pages/shop/detail?id=" + p.id });
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
    a: $data.loading && $data.list.length == 0
  }, $data.loading && $data.list.length == 0 ? {
    b: common_vendor.f([1, 2, 3, 4], (n, i, i0) => {
      return {
        a: i
      };
    })
  } : {
    c: common_vendor.f($data.list, (p, i, i0) => {
      return common_vendor.e({
        a: p.cover,
        b: common_vendor.o(($event) => $options.open(p.id), i),
        c: common_vendor.t(p.name),
        d: common_vendor.o(($event) => $options.open(p.id), i),
        e: common_vendor.t(p.sales),
        f: common_vendor.t(p.unit.length > 0 ? " " + p.unit : ""),
        g: common_vendor.t(p.price),
        h: p.otPrice > 0
      }, p.otPrice > 0 ? {
        i: common_vendor.t(p.otPrice)
      } : {}, {
        j: common_vendor.o(($event) => $options.add(p), i),
        k: i
      });
    })
  }, {
    d: !$data.loading && $data.list.length == 0
  }, !$data.loading && $data.list.length == 0 ? {} : {}, {
    e: $data.noMore && $data.list.length > 0
  }, $data.noMore && $data.list.length > 0 ? {} : {}, {
    f: common_vendor.o((...args) => $options.loadMore && $options.loadMore(...args), "8e"),
    g: common_vendor.p({
      current: -1
    }),
    h: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    i: `${_ctx.u_s_b_h}px`,
    j: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
