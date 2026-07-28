"use strict";
const common_vendor = require("../../common/vendor.js");
const api_ai = require("../../api/ai.js");
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
      products: []
    };
  },
  onLoad() {
    const cached = common_vendor.index.getStorageSync("yunxiu_ai_request");
    if (cached != null)
      this.requestBody = cached;
    this.load();
  },
  methods: {
    load() {
      this.loading = true;
      api_ai.recommendTea(this.requestBody).then((data) => {
        var _a, _b, _c, _d, _f, _g, _h, _j, _k, _l, _m, _o;
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
      common_vendor.index.showModal(new common_vendor.UTSJSONObject({ title: "咨询选茶师", content: this.consultantText, showCancel: false }));
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_assets._imports_5,
    b: common_vendor.t($data.summary),
    c: $data.loading
  }, $data.loading ? {} : $data.products.length == 0 ? {
    e: common_vendor.o((...args) => $options.retry && $options.retry(...args), "56")
  } : {
    f: common_vendor.f($data.products, (item, i, i0) => {
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
    g: common_assets._imports_0$2,
    h: common_assets._imports_2$1,
    i: common_assets._imports_3$1
  }, {
    d: $data.products.length == 0,
    j: common_assets._imports_2$1,
    k: common_vendor.f($data.tips, (item, i, i0) => {
      return {
        a: item.icon,
        b: common_vendor.t(item.title),
        c: common_vendor.t(item.desc),
        d: i
      };
    }),
    l: $data.skillName.length > 0
  }, $data.skillName.length > 0 ? {
    m: common_vendor.t($data.skillName)
  } : {}, {
    n: common_assets._imports_5,
    o: common_vendor.o((...args) => $options.consult && $options.consult(...args), "fb"),
    p: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    q: `${_ctx.u_s_b_h}px`,
    r: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
