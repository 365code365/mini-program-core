"use strict";
const common_vendor = require("../../common/vendor.js");
const utils_assets = require("../../utils/assets.js");
class ValueItem extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          word: { type: String, optional: false },
          l1: { type: String, optional: false },
          l2: { type: String, optional: false }
        };
      },
      name: "ValueItem"
    };
  }
  constructor(options, metadata = ValueItem.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.word = this.__props__.word;
    this.l1 = this.__props__.l1;
    this.l2 = this.__props__.l2;
    delete this.__props__;
  }
}
class StandardItem extends common_vendor.UTS.UTSType {
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
      name: "StandardItem"
    };
  }
  constructor(options, metadata = StandardItem.get$UTSMetadata$(), isJSONParse = false) {
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
      imgHero: utils_assets.assetUrl("origin_hero", "/static/gen/origin-hero.jpg"),
      imgArea: utils_assets.assetUrl("origin_area", "/static/gen/origin-area.jpg"),
      imgGarden: utils_assets.assetUrl("origin_garden", "/static/gen/origin-garden.jpg"),
      imgMaster: utils_assets.assetUrl("origin_master", "/static/gen/origin-master.jpg"),
      values: [
        new ValueItem({ word: "敬", l1: "敬茶敬人", l2: "心存敬畏" }),
        new ValueItem({ word: "诚", l1: "诚实不欺", l2: "真实透明" }),
        new ValueItem({ word: "情义", l1: "重情重义", l2: "彼此成就" }),
        new ValueItem({ word: "工夫", l1: "用心用时", l2: "做好每件事" }),
        new ValueItem({ word: "本分", l1: "不妄为", l2: "专注做好茶" })
      ],
      standards: [
        new StandardItem({ name: "产区真实", desc: "只选核心产区\n真实可溯", icon: "/static/icons/mountain-d.png" }),
        new StandardItem({ name: "师傅可信", desc: "本地制茶师傅\n经验可靠", icon: "/static/icons/user-d.png" }),
        new StandardItem({ name: "工艺稳定", desc: "传统工艺为本\n稳定可控", icon: "/static/icons/leaf-d.png" }),
        new StandardItem({ name: "口感适配", desc: "多次盲评品鉴\n好喝耐泡", icon: "/static/icons/star-d.png" }),
        new StandardItem({ name: "价格相符", desc: "合理透明定价\n价值相称", icon: "/static/icons/doc-d.png" }),
        new StandardItem({ name: "长期可托付", desc: "长期合作共赢\n品质如一", icon: "/static/icons/shield-d.png" })
      ]
    };
  },
  onLoad() {
    this.statusBarH = common_vendor.index.getWindowInfo().statusBarHeight;
    this.imgHero = utils_assets.assetUrl("origin_hero", "/static/gen/origin-hero.jpg");
    this.imgArea = utils_assets.assetUrl("origin_area", "/static/gen/origin-area.jpg");
    this.imgGarden = utils_assets.assetUrl("origin_garden", "/static/gen/origin-garden.jpg");
    this.imgMaster = utils_assets.assetUrl("origin_master", "/static/gen/origin-master.jpg");
  },
  methods: {
    openArea() {
      common_vendor.index.navigateTo({ url: "/pages/category/category" });
    },
    openProcess() {
      common_vendor.index.navigateTo({ url: "/pages/process/process" });
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
  return {
    a: common_vendor.s("padding-top:" + ($data.statusBarH + 8) + "px"),
    b: $data.imgHero,
    c: common_vendor.f($data.values, (item, i, i0) => {
      return {
        a: common_vendor.t(item.word),
        b: common_vendor.t(item.l1),
        c: common_vendor.t(item.l2),
        d: i
      };
    }),
    d: $data.imgArea,
    e: common_vendor.o((...args) => $options.openArea && $options.openArea(...args), "5a"),
    f: common_vendor.f($data.standards, (item, i, i0) => {
      return {
        a: item.icon,
        b: common_vendor.t(item.name),
        c: common_vendor.t(item.desc),
        d: i
      };
    }),
    g: $data.imgGarden,
    h: common_vendor.o((...args) => $options.openArea && $options.openArea(...args), "c7"),
    i: $data.imgMaster,
    j: common_vendor.o((...args) => $options.openProcess && $options.openProcess(...args), "5d"),
    k: common_vendor.p({
      current: 3
    }),
    l: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    m: `${_ctx.u_s_b_h}px`,
    n: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
