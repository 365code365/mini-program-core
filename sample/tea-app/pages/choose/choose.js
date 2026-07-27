"use strict";
const common_vendor = require("../../common/vendor.js");
const utils_assets = require("../../utils/assets.js");
const common_assets = require("../../common/assets.js");
class Direction extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          name: { type: String, optional: false },
          desc: { type: String, optional: false },
          cover: { type: String, optional: false },
          text: { type: String, optional: false }
        };
      },
      name: "Direction"
    };
  }
  constructor(options, metadata = Direction.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.name = this.__props__.name;
    this.desc = this.__props__.desc;
    this.cover = this.__props__.cover;
    this.text = this.__props__.text;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      statusBarH: 20,
      safeBottom: 0,
      imgHero: utils_assets.assetUrl("choose_hero", "/static/gen/choose-hero.jpg"),
      inputText: "",
      quickOptions: ["我自己喝", "第一次喝单枞", "想试品鉴装", "想喝好一点", "我想送人", "不确定，帮我推荐"],
      // 图标用包内 PNG：品牌宋体没有几何图形类码位(U+25xx)，用文字字形真机会掉成豆腐块/emoji
      quickIcons: ["/static/icons/user.png", "/static/icons/leaf.png", "/static/icons/plate.png", "/static/icons/star.png", "/static/icons/gift.png", "/static/icons/chat.png"],
      directions: [
        new Direction({ name: "新手入门", desc: "清香柔和，容易喜欢", cover: utils_assets.assetUrl("choose_dir_newbie", "/static/gen/dir-newbie.jpg"), text: "第一次喝单枞，想从清香柔和的入门款开始" }),
        new Direction({ name: "日常自饮", desc: "香气馥郁，口感平衡", cover: utils_assets.assetUrl("choose_dir_daily", "/static/gen/dir-daily.jpg"), text: "自己日常喝，希望香气馥郁、口感平衡" }),
        new Direction({ name: "高阶老枞", desc: "山韵醇厚，回味悠长", cover: utils_assets.assetUrl("choose_dir_advanced", "/static/gen/dir-advanced.jpg"), text: "喝过基础款，想体验山韵醇厚、回味悠长的老枞" })
      ]
    };
  },
  onLoad() {
    this.statusBarH = common_vendor.index.getWindowInfo().statusBarHeight;
    this.safeBottom = common_vendor.index.getWindowInfo().safeAreaInsets.bottom;
    this.imgHero = utils_assets.assetUrl("choose_hero", "/static/gen/choose-hero.jpg");
    const keys = ["choose_dir_newbie", "choose_dir_daily", "choose_dir_advanced"];
    const next = [];
    for (let i = 0; i < this.directions.length; i++) {
      const d = this.directions[i];
      next.push(new Direction({ name: d.name, desc: d.desc, cover: utils_assets.assetUrl(keys[i], d.cover), text: d.text }));
    }
    this.directions = next;
  },
  methods: {
    chooseQuick(i) {
      const bodies = [
        new common_vendor.UTSJSONObject({ intent: "self", level: "normal", taste: "unknown", budget: "middle", text: "我自己喝，想选一款日常好茶" }),
        new common_vendor.UTSJSONObject({ intent: "self", level: "newbie", taste: "aroma", budget: "low", text: "第一次喝单枞，想先从香气清晰的入门款开始" }),
        new common_vendor.UTSJSONObject({ intent: "self", level: "newbie", taste: "unknown", budget: "low", text: "想试品鉴装，先了解不同香型" }),
        new common_vendor.UTSJSONObject({ intent: "self", level: "advanced", taste: "thick", budget: "high", text: "想喝好一点，偏好山韵和层次" }),
        new common_vendor.UTSJSONObject({ intent: "gift", level: "normal", taste: "unknown", budget: "middle", text: "我想送人，希望得体有分寸" }),
        new common_vendor.UTSJSONObject({ intent: "self", level: "normal", taste: "unknown", budget: "middle", text: "不确定，帮我推荐一款适合大多数人的茶" })
      ];
      this.openResult(bodies[i]);
    },
    chooseDirection(i) {
      const d = this.directions[i];
      let level = "normal";
      let taste = "unknown";
      let budget = "middle";
      if (d.name == "新手入门") {
        level = "newbie";
        taste = "aroma";
        budget = "low";
      }
      if (d.name == "高阶老枞") {
        level = "advanced";
        taste = "thick";
        budget = "high";
      }
      this.openResult(new common_vendor.UTSJSONObject({ intent: "self", level, taste, budget, text: d.text }));
    },
    sendText() {
      const text = this.inputText.trim();
      if (text.length == 0) {
        common_vendor.index.showToast({ title: "先告诉我你的需求吧", icon: "none" });
        return null;
      }
      this.openResult(new common_vendor.UTSJSONObject({ intent: "self", level: "normal", taste: "unknown", budget: "middle", text }));
    },
    openResult(body) {
      common_vendor.index.setStorageSync("yunxiu_ai_request", body);
      common_vendor.index.navigateTo({ url: "/pages/ai-result/ai-result" });
    },
    rotateDirections() {
      if (this.directions.length <= 1)
        return null;
      const next = [];
      for (let i = 1; i < this.directions.length; i++)
        next.push(this.directions[i]);
      next.push(this.directions[0]);
      this.directions = next;
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
    c: common_assets._imports_2$1,
    d: common_assets._imports_2$1,
    e: common_vendor.f($data.quickOptions, (item, i, i0) => {
      return {
        a: $data.quickIcons[i],
        b: common_vendor.t(item),
        c: i,
        d: common_vendor.o(($event) => $options.chooseQuick(i), i)
      };
    }),
    f: common_assets._imports_2$1,
    g: common_vendor.o((...args) => $options.rotateDirections && $options.rotateDirections(...args), "2d"),
    h: common_vendor.f($data.directions, (item, i, i0) => {
      return {
        a: item.cover,
        b: common_vendor.t(item.name),
        c: common_vendor.t(item.desc),
        d: i,
        e: common_vendor.o(($event) => $options.chooseDirection(i), i)
      };
    }),
    i: common_vendor.o((...args) => $options.sendText && $options.sendText(...args), "12"),
    j: $data.inputText,
    k: common_vendor.o(($event) => $data.inputText = $event.detail.value, "bc"),
    l: common_assets._imports_1$1,
    m: common_vendor.o((...args) => $options.sendText && $options.sendText(...args), "57"),
    n: common_vendor.s("bottom:" + ($data.safeBottom + 108) + "px"),
    o: common_vendor.p({
      current: 1
    }),
    p: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    q: `${_ctx.u_s_b_h}px`,
    r: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  };
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
