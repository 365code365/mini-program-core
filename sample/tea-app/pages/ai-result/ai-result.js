"use strict";
const common_vendor = require("../../common/vendor.js");
const api_ai = require("../../api/ai.js");
const api_consult = require("../../api/consult.js");
const utils_assets = require("../../utils/assets.js");
const common_assets = require("../../common/assets.js");
class Tip extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          title: { type: String, optional: false },
          desc: { type: String, optional: false },
          icon: { type: String, optional: false }
        };
      },
      name: "Tip"
    };
  }
  constructor(options, metadata = Tip.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.title = this.__props__.title;
    this.desc = this.__props__.desc;
    this.icon = this.__props__.icon;
    delete this.__props__;
  }
}
class AiProduct extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          subtitle: { type: String, optional: false },
          image: { type: String, optional: false },
          price: { type: String, optional: false },
          suited: { type: String, optional: false },
          reason: { type: String, optional: false },
          notFor: { type: String, optional: false },
          tasteTags: { type: common_vendor.UTS.UTSType.withGenerics(Array, [String]), optional: false }
        };
      },
      name: "AiProduct"
    };
  }
  constructor(options, metadata = AiProduct.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.subtitle = this.__props__.subtitle;
    this.image = this.__props__.image;
    this.price = this.__props__.price;
    this.suited = this.__props__.suited;
    this.reason = this.__props__.reason;
    this.notFor = this.__props__.notFor;
    this.tasteTags = this.__props__.tasteTags;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      loading: true,
      summary: "正在理解你的选茶需求",
      consultantText: "还拿不准，可以继续告诉选茶师你的饮茶习惯、送礼对象和预算。",
      skillName: "",
      tips: [
        new Tip({ title: "先品香，后定口", desc: "从清香、花香开始\n更容易找到偏好", icon: "/static/icons/leaf-d.png" }),
        new Tip({ title: "小罐尝试更稳妥", desc: "建议先从品鉴装入手\n找到喜欢的风格再深入", icon: "/static/icons/gift-d.png" }),
        new Tip({ title: "日常泡法更关键", desc: "注意投茶量与出汤时间\n香气更容易被感知", icon: "/static/icons/doc-d.png" }),
        new Tip({ title: "好茶也需好节奏", desc: "慢慢喝、多感受\n茶会更懂你", icon: "/static/icons/star-d.png" })
      ],
      requestBody: new common_vendor.UTSJSONObject({}),
      products: [],
      // 思考过程：先用用户提交的意图占位，服务端返回识别结果后覆盖
      showThink: true,
      askText: "",
      reIntent: "self",
      reLevel: "normal",
      reTaste: "unknown",
      reBudget: "middle"
    };
  },
  computed: {
    /** 需求标签。文案与服务端 TeaAiServiceImpl 的 levelLabel/tasteLabel/budgetLabel 保持一致 */
    chips() {
      const out = [];
      out.push("场景：" + this.intentLabel);
      out.push(this.levelLabel);
      out.push(this.tasteLabel);
      out.push(this.budgetLabel);
      return out;
    },
    intentLabel() {
      if (this.reIntent == "gift")
        return "送礼";
      if (this.reIntent == "business")
        return "企业采购";
      return "自己喝";
    },
    levelLabel() {
      if (this.reLevel == "newbie")
        return "第一次喝单枞";
      if (this.reLevel == "advanced")
        return "想体验更高水准";
      return "已有日常饮茶习惯";
    },
    tasteLabel() {
      if (this.reTaste == "aroma")
        return "偏好清晰花香";
      if (this.reTaste == "sweet")
        return "偏好蜜甜回甘";
      if (this.reTaste == "thick")
        return "偏好醇厚山韵";
      if (this.reTaste == "roast")
        return "偏好焙火熟香";
      return "口感平衡";
    },
    budgetLabel() {
      if (this.reBudget == "low")
        return "预算友好";
      if (this.reBudget == "high")
        return "更重视品质";
      return "预算适中";
    },
    /** 多款推荐理由完全相同时返回该理由，否则返回空串（各款分别展示） */
    sharedReason() {
      if (this.products.length < 2)
        return "";
      const first = this.products[0].reason;
      if (first.length == 0)
        return "";
      for (let i = 1; i < this.products.length; i++) {
        if (this.products[i].reason != first)
          return "";
      }
      return first;
    }
  },
  onLoad() {
    var _a;
    const cached = common_vendor.index.getStorageSync("yunxiu_ai_request");
    if (cached != null) {
      this.requestBody = cached;
      this.applyIntent(this.requestBody);
      this.askText = (_a = this.requestBody.getString("text")) !== null && _a !== void 0 ? _a : "";
    }
    this.load();
  },
  methods: {
    toggleThink() {
      this.showThink = !this.showThink;
    },
    /** 意图四要素：请求体与响应体字段同名，复用同一段读取 */
    applyIntent(src) {
      const intent = src.getString("intent");
      const level = src.getString("level");
      const taste = src.getString("taste");
      const budget = src.getString("budget");
      if (intent != null && intent.length > 0)
        this.reIntent = intent;
      if (level != null && level.length > 0)
        this.reLevel = level;
      if (taste != null && taste.length > 0)
        this.reTaste = taste;
      if (budget != null && budget.length > 0)
        this.reBudget = budget;
    },
    load() {
      this.loading = true;
      api_ai.recommendTea(this.requestBody).then((data) => {
        var _a, _b, _c, _d, _f, _g, _h, _j, _k, _l, _m, _o;
        this.applyIntent(data);
        this.summary = (_a = data.getString("summary")) !== null && _a !== void 0 ? _a : "为你筛选了以下合适好茶";
        this.consultantText = (_b = data.getString("consultantText")) !== null && _b !== void 0 ? _b : this.consultantText;
        this.skillName = (_c = data.getString("skillName")) !== null && _c !== void 0 ? _c : "";
        const list = (_d = data.getArray("products")) !== null && _d !== void 0 ? _d : [];
        const result = [];
        for (let i = 0; i < list.length; i++) {
          const p = list[i];
          const rawTags = p["tasteTags"];
          const tags = rawTags == null ? [] : rawTags;
          result.push(new AiProduct({
            id: (_f = p.getNumber("id")) !== null && _f !== void 0 ? _f : 0,
            name: (_g = p.getString("name")) !== null && _g !== void 0 ? _g : "凤凰单枞",
            subtitle: (_h = p.getString("subtitle")) !== null && _h !== void 0 ? _h : "",
            image: (_j = p.getString("image")) !== null && _j !== void 0 ? _j : "",
            price: "" + ((_k = p.getNumber("price")) !== null && _k !== void 0 ? _k : 0),
            suited: (_l = p.getString("suited")) !== null && _l !== void 0 ? _l : "",
            reason: (_m = p.getString("reason")) !== null && _m !== void 0 ? _m : "",
            notFor: (_o = p.getString("notFor")) !== null && _o !== void 0 ? _o : "",
            tasteTags: tags
          }));
        }
        this.products = result;
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    retry() {
      common_vendor.index.navigateBack();
    },
    imgFor(item, i) {
      if (item.image.length > 0)
        return item.image;
      const fallbacks = [
        utils_assets.assetUrl("ai_prod_1", "/static/gen/ai-prod1.jpg"),
        utils_assets.assetUrl("ai_prod_2", "/static/gen/ai-prod2.jpg"),
        utils_assets.assetUrl("ai_prod_3", "/static/gen/ai-prod3.jpg")
      ];
      return fallbacks[i % 3];
    },
    openProduct(id) {
      if (id <= 0)
        return null;
      common_vendor.index.navigateTo({ url: "/pages/shop/detail?id=" + id });
    },
    consult() {
      api_consult.openConsult("consult", "ai_result", 0);
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_assets._imports_5,
    b: common_vendor.t($data.summary),
    c: common_assets._imports_2$1,
    d: common_vendor.t($data.showThink ? "收起" : "展开"),
    e: common_vendor.o((...args) => $options.toggleThink && $options.toggleThink(...args), "55"),
    f: $data.showThink
  }, $data.showThink ? common_vendor.e({
    g: $data.askText.length > 0
  }, $data.askText.length > 0 ? {
    h: common_vendor.t($data.askText)
  } : {}, {
    i: common_vendor.f($options.chips, (item, i, i0) => {
      return {
        a: common_vendor.t(item),
        b: i
      };
    }),
    j: common_vendor.n($data.loading ? "dot-doing" : "dot-done"),
    k: common_vendor.t($data.skillName.length > 0 ? $data.skillName : "默认选茶规则"),
    l: common_vendor.n($data.loading ? "dot-wait" : "dot-done"),
    m: $data.loading
  }, $data.loading ? {} : $data.products.length == 0 ? {} : {
    o: common_vendor.t($data.products.length)
  }, {
    n: $data.products.length == 0,
    p: $options.sharedReason.length > 0
  }, $options.sharedReason.length > 0 ? {
    q: common_vendor.t($options.sharedReason)
  } : {}, {
    r: common_vendor.f($data.products, (item, i, i0) => {
      return common_vendor.e({
        a: common_vendor.t(i + 1)
      }, $options.sharedReason.length > 0 ? {
        b: common_vendor.t(item.name)
      } : {
        c: common_vendor.t(item.name),
        d: common_vendor.t(item.reason.length > 0 ? item.reason : "与你的需求关键词匹配度最高")
      }, {
        e: i
      });
    }),
    s: $options.sharedReason.length > 0,
    t: !$data.loading && $data.products.length > 0
  }, !$data.loading && $data.products.length > 0 ? {
    v: common_vendor.t($data.products[0].notFor)
  } : {}) : {}, {
    w: $data.loading
  }, $data.loading ? {} : $data.products.length == 0 ? {
    y: common_vendor.o((...args) => $options.retry && $options.retry(...args), "89")
  } : {
    z: common_vendor.f($data.products, (item, i, i0) => {
      return common_vendor.e({
        a: $options.imgFor(item, i),
        b: common_vendor.t(item.name),
        c: common_vendor.t(i == 0 ? "入门推荐" : i == 1 ? "日常口粮" : "进阶推荐"),
        d: common_vendor.t(item.subtitle.length > 0 ? item.subtitle : "认识凤凰云岫的这一杯好茶"),
        e: common_vendor.t(item.suited),
        f: common_vendor.t(item.reason),
        g: common_vendor.t(item.notFor),
        h: common_vendor.o(($event) => $options.openProduct(item.id), i),
        i: common_vendor.o((...args) => $options.consult && $options.consult(...args), i),
        j: item.tasteTags.length > 0
      }, item.tasteTags.length > 0 ? {
        k: common_vendor.f(item.tasteTags, (tag, j, i1) => {
          return {
            a: common_vendor.t(tag),
            b: j
          };
        })
      } : {}, {
        l: i
      });
    }),
    A: common_assets._imports_0$3,
    B: common_assets._imports_2$1,
    C: common_assets._imports_3
  }, {
    x: $data.products.length == 0,
    D: common_assets._imports_2$1,
    E: common_vendor.f($data.tips, (item, i, i0) => {
      return {
        a: item.icon,
        b: common_vendor.t(item.title),
        c: common_vendor.t(item.desc),
        d: i
      };
    }),
    F: $data.skillName.length > 0
  }, $data.skillName.length > 0 ? {
    G: common_vendor.t($data.skillName)
  } : {}, {
    H: common_assets._imports_5,
    I: common_vendor.o((...args) => $options.consult && $options.consult(...args), "0d"),
    J: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    K: `${_ctx.u_s_b_h}px`,
    L: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/ai-result/ai-result.js.map
