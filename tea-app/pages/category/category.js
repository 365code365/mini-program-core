"use strict";
const common_vendor = require("../../common/vendor.js");
const common_assets = require("../../common/assets.js");
class MenuItem extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          label: { type: String, optional: false },
          icon: { type: String, optional: false },
          iconOn: { type: String, optional: false }
        };
      },
      name: "MenuItem"
    };
  }
  constructor(options, metadata = MenuItem.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.label = this.__props__.label;
    this.icon = this.__props__.icon;
    this.iconOn = this.__props__.iconOn;
    delete this.__props__;
  }
}
class TeaCard extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          name: { type: String, optional: false },
          subtitle: { type: String, optional: false },
          cover: { type: String, optional: false },
          tag: { type: String, optional: false }
        };
      },
      name: "TeaCard"
    };
  }
  constructor(options, metadata = TeaCard.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.name = this.__props__.name;
    this.subtitle = this.__props__.subtitle;
    this.cover = this.__props__.cover;
    this.tag = this.__props__.tag;
    delete this.__props__;
  }
}
class TrustItem extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          title: { type: String, optional: false },
          subtitle: { type: String, optional: false },
          icon: { type: String, optional: false }
        };
      },
      name: "TrustItem"
    };
  }
  constructor(options, metadata = TrustItem.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.title = this.__props__.title;
    this.subtitle = this.__props__.subtitle;
    this.icon = this.__props__.icon;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      statusBarH: 20,
      currentTab: 0,
      menu: [
        new MenuItem({ label: "产区选茶", icon: "/static/icons/leaf-m.png", iconOn: "/static/icons/leaf-d.png" }),
        new MenuItem({ label: "香型选茶", icon: "/static/icons/plate-m.png", iconOn: "/static/icons/plate-d.png" }),
        new MenuItem({ label: "价位选茶", icon: "/static/icons/doc-m.png", iconOn: "/static/icons/doc-d.png" }),
        new MenuItem({ label: "古树认养", icon: "/static/icons/tree-m.png", iconOn: "/static/icons/tree-d.png" })
      ],
      origins: [
        new TeaCard({ name: "乌岽", subtitle: "高山代表产区 · 山韵深厚", cover: "/static/slices/category-origin/card-wudong.jpg", tag: "乌岽" }),
        new TeaCard({ name: "大庵", subtitle: "传统山场 · 香气稳净", cover: "/static/slices/category-origin/card-jianrao-organic.jpg", tag: "大庵" }),
        new TeaCard({ name: "坪坑头", subtitle: "核心古树产区 · 个性鲜明", cover: "/static/slices/category-origin/card-pingkengkou.jpg", tag: "坪坑头" }),
        new TeaCard({ name: "凤溪石瓮", subtitle: "高山云雾 · 回甘细长", cover: "/static/slices/category-origin/card-fengxi-shiwen.jpg", tag: "凤溪石瓮" })
      ],
      aromas: [
        new TeaCard({ name: "鸭屎香", subtitle: "银花清香 · 山韵悠长", cover: "/static/slices/category-aroma/card-yashixiang.jpg", tag: "鸭屎香" }),
        new TeaCard({ name: "蜜兰香", subtitle: "蜜甜兰韵 · 滋味醇厚", cover: "/static/slices/category-aroma/card-milanxiang.jpg", tag: "蜜兰香" }),
        new TeaCard({ name: "芝兰香", subtitle: "幽兰清芬 · 山骨韵显", cover: "/static/slices/category-aroma/card-zhilanxiang.jpg", tag: "芝兰香" }),
        new TeaCard({ name: "桂花香", subtitle: "桂花甜韵 · 柔和顺滑", cover: "/static/slices/category-aroma/card-guihuaxiang.jpg", tag: "桂花香" })
      ],
      prices: [
        new TeaCard({ name: "日常口粮", subtitle: "百元好茶 · 日常常饮", cover: "/static/slices/category-aroma/card-daye.jpg", tag: "100-300" }),
        new TeaCard({ name: "品鉴进阶", subtitle: "山场香韵 · 进阶细品", cover: "/static/slices/category-aroma/card-huazhixiang.jpg", tag: "300-800" }),
        new TeaCard({ name: "珍藏臻品", subtitle: "核心产区 · 送礼收藏", cover: "/static/slices/category-aroma/card-aofuhou.jpg", tag: "800-1500" }),
        new TeaCard({ name: "大师典藏", subtitle: "稀缺古树 · 高端定制", cover: "/static/slices/category-aroma/card-shuixian.jpg", tag: "1500+" })
      ],
      adoptions: [
        new TeaCard({ name: "百年古树", subtitle: "一树一证 · 专属认养", cover: "/static/slices/category-origin/card-pingkengkou.jpg", tag: "百年古树" }),
        new TeaCard({ name: "核心山场", subtitle: "源头直达 · 全程可溯", cover: "/static/slices/category-origin/card-tianfengshan.jpg", tag: "核心山场" }),
        new TeaCard({ name: "认养茶礼", subtitle: "专属茶品 · 定制心意", cover: "/static/slices/category-origin/card-fengxi-shiwen.jpg", tag: "认养茶礼" }),
        new TeaCard({ name: "我的茶园", subtitle: "实时守护 · 云端看园", cover: "/static/slices/category-origin/card-wudong.jpg", tag: "我的茶园" })
      ],
      trust: [
        new TrustItem({ title: "真实产区", subtitle: "源头可溯", icon: "/static/icons/mountain-d.png" }),
        new TrustItem({ title: "香型清晰", subtitle: "科学分类", icon: "/static/icons/leaf-d.png" }),
        new TrustItem({ title: "价位分层", subtitle: "按需选择", icon: "/static/icons/doc-d.png" }),
        new TrustItem({ title: "认养预留", subtitle: "专属好茶", icon: "/static/icons/user-d.png" })
      ]
    };
  },
  computed: {
    displayCards() {
      if (this.currentTab == 1)
        return this.aromas;
      if (this.currentTab == 2)
        return this.prices;
      if (this.currentTab == 3)
        return this.adoptions;
      return this.origins;
    }
  },
  onLoad() {
    this.statusBarH = common_vendor.index.getWindowInfo().statusBarHeight;
  },
  methods: {
    selectTab(i) {
      this.currentTab = i;
    },
    onSearch(e) {
      const keyword = e.detail.value;
      common_vendor.index.navigateTo({ url: "/pages/search/search?keyword=" + encodeURIComponent(keyword) });
    },
    goList(tag) {
      if (this.currentTab == 3 && tag == "我的茶园") {
        common_vendor.index.navigateTo({ url: "/pages/my-garden/my-garden" });
        return null;
      }
      if (this.currentTab == 3) {
        common_vendor.index.navigateTo({ url: "/pages/adoption/list?variety=" + encodeURIComponent("古树") });
        return null;
      }
      common_vendor.index.navigateTo({ url: "/pages/shop/list?keyword=" + encodeURIComponent(tag) });
    },
    goBack() {
      common_vendor.index.navigateBack(new common_vendor.UTSJSONObject({ fail: () => {
        common_vendor.index.switchTab({ url: "/pages/index/index" });
      } }));
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
    a: common_assets._imports_0$3,
    b: common_vendor.s("top:" + ($data.statusBarH + 22) + "px"),
    c: common_vendor.o((...args) => $options.goBack && $options.goBack(...args), "6a"),
    d: common_assets._imports_0$4,
    e: common_vendor.o((...args) => $options.onSearch && $options.onSearch(...args), "5d"),
    f: common_vendor.s("padding-top:" + ($data.statusBarH + 22) + "px"),
    g: common_vendor.f($data.menu, (item, i, i0) => {
      return common_vendor.e({
        a: $data.currentTab == i
      }, $data.currentTab == i ? {} : {}, {
        b: $data.currentTab == i ? item.iconOn : item.icon,
        c: common_vendor.t(item.label),
        d: common_vendor.n($data.currentTab == i ? "side-text-on" : ""),
        e: i,
        f: common_vendor.n($data.currentTab == i ? "side-item-on" : ""),
        g: common_vendor.o(($event) => $options.selectTab(i), i)
      });
    }),
    h: common_vendor.f($options.displayCards, (card, i, i0) => {
      return {
        a: card.cover,
        b: common_vendor.t(card.name),
        c: common_vendor.t(card.subtitle),
        d: i,
        e: common_vendor.o(($event) => $options.goList(card.tag), i)
      };
    }),
    i: common_vendor.f($data.trust, (item, i, i0) => {
      return {
        a: item.icon,
        b: common_vendor.t(item.title),
        c: common_vendor.t(item.subtitle),
        d: i
      };
    }),
    j: common_vendor.p({
      current: -1
    }),
    k: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    l: `${_ctx.u_s_b_h}px`,
    m: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
