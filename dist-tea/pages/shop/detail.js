"use strict";
const common_vendor = require("../../common/vendor.js");
const api_product = require("../../api/product.js");
const api_cart = require("../../api/cart.js");
const api_mappers = require("../../api/mappers.js");
const utils_assets = require("../../utils/assets.js");
const common_assets = require("../../common/assets.js");
class AttrGroup extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          name: { type: String, optional: false },
          values: { type: common_vendor.UTS.UTSType.withGenerics(Array, [String]), optional: false }
        };
      },
      name: "AttrGroup"
    };
  }
  constructor(options, metadata = AttrGroup.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.name = this.__props__.name;
    this.values = this.__props__.values;
    delete this.__props__;
  }
}
function parseSlider(raw) {
  const out = [];
  if (raw.length == 0)
    return out;
  const arr = common_vendor.UTS.JSON.parseArray(raw);
  if (arr == null)
    return out;
  for (let i = 0; i < arr.length; i++) {
    const u = ("" + arr[i]).trim();
    if (u.length > 0)
      out.push(u);
  }
  return out;
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      id: 0,
      imgCover: utils_assets.assetUrl("detail_cover", "/static/gen/detail-1.jpg"),
      imgOrigin: utils_assets.assetUrl("detail_origin", "/static/gen/detail-3.jpg"),
      p: new api_mappers.ProductVM({ id: 0, name: "", cover: "", price: 0, otPrice: 0, sales: 0, unit: "", stock: 0 }),
      attrGroups: [],
      sel: [],
      pv: new common_vendor.UTSJSONObject({}),
      curUnique: "",
      curPrice: "",
      curOt: "",
      curImage: "",
      curStock: 0,
      sliderImages: [],
      slideIndex: 0,
      qty: 1,
      subtitle: "认识凤凰云岫的这一杯好茶",
      vipPrice: "",
      suited: "日常饮茶、想认识凤凰单枞香型，或希望挑选一份有分寸茶礼的你",
      reason: "从当前在售好茶中认真筛选，兼顾香气、滋味、耐泡度与价格是否相符。",
      notFor: "偏好浓烈重口感，或只追求名气与高溢价的人群",
      brew: "投茶 5–6g｜水温 95–100°C｜前段快速出汤 5–10 秒，可按口感逐泡延长",
      tasteTags: ["花香清雅", "滋味甘醇", "山韵悠长", "回甘明显"],
      intro: "源自凤凰核心产区，遵循传统工夫茶制作逻辑，认真复核每一批茶的产区、工艺与口感。"
    };
  },
  computed: {
    tasteLine() {
      return this.tasteTags.join(" ｜ ");
    },
    /** 轮播图源：选中规格图排首位，其后是商品轮播图；都没有时回退主图/占位图 */
    slides() {
      const out = [];
      if (this.curImage.length > 0)
        out.push(this.curImage);
      for (let i = 0; i < this.sliderImages.length; i++) {
        const u = this.sliderImages[i];
        if (u != this.curImage)
          out.push(u);
      }
      if (out.length == 0)
        out.push(this.p.cover.length > 0 ? this.p.cover : this.imgCover);
      return out;
    }
  },
  onLoad(opt) {
    this.imgCover = utils_assets.assetUrl("detail_cover", "/static/gen/detail-1.jpg");
    this.imgOrigin = utils_assets.assetUrl("detail_origin", "/static/gen/detail-3.jpg");
    const id = opt["id"];
    if (id != null) {
      this.id = parseInt(id);
      this.load();
    }
  },
  methods: {
    load() {
      api_product.productDetail(this.id).then((d) => {
        var _a, _b, _c, _f, _g, _h;
        const info = (_b = (_a = d["productInfo"]) !== null && _a !== void 0 ? _a : d["storeInfo"]) !== null && _b !== void 0 ? _b : d;
        this.p = api_mappers.toProduct(info);
        this.sliderImages = parseSlider((_c = info.getString("sliderImage")) !== null && _c !== void 0 ? _c : "");
        this.slideIndex = 0;
        this.applyGuide(info);
        const desc = (_f = info.getString("content")) !== null && _f !== void 0 ? _f : info.getString("description");
        if (desc != null && desc.length > 0) {
          this.intro = desc.replaceAll("<img", '<img style="max-width:100%;height:auto;display:block;margin:10rpx 0;"');
        }
        const groups = [];
        const initSel = [];
        const pa = d["productAttr"];
        if (pa != null && Array.isArray(pa)) {
          const arr = pa;
          for (let i = 0; i < arr.length; i++) {
            const nm = (_g = arr[i].getString("attrName")) !== null && _g !== void 0 ? _g : "";
            const vstr = (_h = arr[i].getString("attrValues")) !== null && _h !== void 0 ? _h : "";
            const vals = [];
            const parts = vstr.split(",");
            for (let j = 0; j < parts.length; j++) {
              const t = parts[j].trim();
              if (t.length > 0)
                vals.push(t);
            }
            groups.push(new AttrGroup({ name: nm, values: vals }));
            initSel.push(vals.length > 0 ? vals[0] : "");
          }
        }
        this.attrGroups = groups;
        this.sel = initSel;
        const pvObj = d["productValue"];
        if (pvObj != null)
          this.pv = pvObj;
        this.recompute();
      }).catch((_e = null) => {
      });
    },
    applyGuide(info) {
      var _a, _b, _c, _f, _g, _h, _j, _k;
      const suited = (_a = info.getString("suited")) !== null && _a !== void 0 ? _a : "";
      const reason = (_b = info.getString("reason")) !== null && _b !== void 0 ? _b : "";
      const notFor = (_c = info.getString("notFor")) !== null && _c !== void 0 ? _c : "";
      const brew = (_f = info.getString("brew")) !== null && _f !== void 0 ? _f : "";
      if (suited.length > 0)
        this.suited = suited;
      if (reason.length > 0)
        this.reason = reason;
      if (notFor.length > 0)
        this.notFor = notFor;
      if (brew.length > 0)
        this.brew = brew;
      const storeInfo = (_g = info.getString("storeInfo")) !== null && _g !== void 0 ? _g : "";
      if (storeInfo.length > 0)
        this.subtitle = storeInfo;
      const vip = (_h = info.getNumber("vipPrice")) !== null && _h !== void 0 ? _h : 0;
      if (vip > 0 && vip < this.p.price)
        this.vipPrice = api_mappers.fmtMoney(vip);
      const tt = (_j = info.getString("tasteTags")) !== null && _j !== void 0 ? _j : "";
      if (tt.length > 0) {
        const list = [];
        const parts = tt.replaceAll("，", ",").split(",");
        for (let i = 0; i < parts.length; i++) {
          const t = parts[i].trim();
          if (t.length > 0)
            list.push(t);
        }
        if (list.length > 0) {
          this.tasteTags = list;
          return null;
        }
      }
      const name = (_k = info.getString("storeName")) !== null && _k !== void 0 ? _k : "";
      const tags = [];
      if (name.indexOf("鸭屎香") >= 0)
        tags.push("银花清香");
      if (name.indexOf("蜜兰") >= 0)
        tags.push("蜜甜兰韵");
      if (name.indexOf("芝兰") >= 0)
        tags.push("幽兰清芬");
      if (name.indexOf("宋种") >= 0 || name.indexOf("茶王") >= 0)
        tags.push("山韵悠长");
      if (tags.length > 0) {
        tags.push("回甘明显");
        this.tasteTags = tags;
      }
    },
    pick(gi, v) {
      const next = [];
      for (let i = 0; i < this.sel.length; i++)
        next.push(i == gi ? v : this.sel[i]);
      this.sel = next;
      this.recompute();
    },
    recompute() {
      var _a, _b, _c, _f, _g;
      const key = this.sel.join(",");
      const node = this.pv[key];
      if (node != null) {
        const o = node;
        this.curUnique = "" + ((_a = o.getNumber("id")) !== null && _a !== void 0 ? _a : 0);
        this.curPrice = (_b = o.getString("price")) !== null && _b !== void 0 ? _b : "";
        this.curOt = (_c = o.getString("otPrice")) !== null && _c !== void 0 ? _c : "";
        this.curImage = (_f = o.getString("image")) !== null && _f !== void 0 ? _f : "";
        this.curStock = (_g = o.getNumber("stock")) !== null && _g !== void 0 ? _g : 0;
      } else {
        this.curUnique = "";
      }
      this.slideIndex = 0;
    },
    onSlide(e = null) {
      var _a;
      const detail = e.detail;
      this.slideIndex = (_a = detail.getNumber("current")) !== null && _a !== void 0 ? _a : 0;
    },
    inc() {
      if (this.qty < 99)
        this.qty += 1;
    },
    dec() {
      if (this.qty > 1)
        this.qty -= 1;
    },
    checkSku() {
      if (this.attrGroups.length > 0 && this.curUnique.length == 0) {
        common_vendor.index.showToast({ title: "请选择规格", icon: "none" });
        return false;
      }
      return true;
    },
    addCart() {
      if (!this.checkSku())
        return null;
      api_cart.cartSave(this.p.id, this.qty, this.curUnique).then((_d) => {
        common_vendor.index.showToast({ title: "已加入购物车", icon: "none" });
      }).catch((_e = null) => {
      });
    },
    buyNow() {
      if (!this.checkSku())
        return null;
      api_cart.cartSave(this.p.id, this.qty, this.curUnique).then((_d) => {
        common_vendor.index.navigateTo({ url: "/pages/cart/cart" });
      }).catch((_e = null) => {
      });
    },
    consult() {
      common_vendor.index.showModal(new common_vendor.UTSJSONObject({ title: "咨询选茶师", content: "还拿不准？可以告诉选茶师你的饮茶习惯、送礼对象和预算。", showCancel: false }));
    },
    showBrew() {
      common_vendor.index.showModal(new common_vendor.UTSJSONObject({ title: "冲泡建议", content: this.brew, showCancel: false }));
    },
    openOrigin() {
      common_vendor.index.switchTab({ url: "/pages/origin/origin" });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_vendor.f($options.slides, (img, i, i0) => {
      return {
        a: img,
        b: i
      };
    }),
    b: $data.slideIndex,
    c: $options.slides.length > 1,
    d: common_vendor.o((...args) => $options.onSlide && $options.onSlide(...args), "6c"),
    e: common_vendor.t($data.p.name.length > 0 ? $data.p.name : "云岫品鉴装"),
    f: common_vendor.t($data.subtitle),
    g: $data.p.unit.length > 0
  }, $data.p.unit.length > 0 ? {
    h: common_vendor.t($data.p.unit)
  } : {}, {
    i: $options.slides.length > 1
  }, $options.slides.length > 1 ? {
    j: common_vendor.f($options.slides, (img, i, i0) => {
      return {
        a: i,
        b: common_vendor.n(i == $data.slideIndex ? "cdot-on" : "")
      };
    })
  } : {}, {
    k: common_vendor.t($data.p.name),
    l: common_vendor.t($data.subtitle),
    m: common_vendor.t($data.curPrice.length > 0 ? $data.curPrice : "" + $data.p.price),
    n: $data.curOt.length > 0
  }, $data.curOt.length > 0 ? {
    o: common_vendor.t($data.curOt)
  } : {}, {
    p: $data.vipPrice.length > 0
  }, $data.vipPrice.length > 0 ? {
    q: common_vendor.t($data.vipPrice)
  } : {}, {
    r: common_vendor.t($data.p.sales),
    s: common_assets._imports_0$3,
    t: common_vendor.t($data.suited),
    v: common_assets._imports_2$1,
    w: common_vendor.t($data.reason),
    x: common_assets._imports_2$3,
    y: common_vendor.t($options.tasteLine),
    z: common_assets._imports_3,
    A: common_vendor.t($data.notFor),
    B: common_assets._imports_4,
    C: common_vendor.o((...args) => $options.showBrew && $options.showBrew(...args), "7f"),
    D: common_vendor.t($data.brew),
    E: $data.imgOrigin,
    F: common_vendor.o((...args) => $options.openOrigin && $options.openOrigin(...args), "e1"),
    G: common_vendor.f($data.attrGroups, (g, gi, i0) => {
      return {
        a: common_vendor.t(g.name),
        b: common_vendor.f(g.values, (v, vi, i1) => {
          return {
            a: common_vendor.t(v),
            b: vi,
            c: common_vendor.n($data.sel[gi] == v ? "chip-on" : ""),
            d: common_vendor.o(($event) => $options.pick(gi, v), vi)
          };
        }),
        c: gi
      };
    }),
    H: common_vendor.t($data.curStock > 0 ? "（库存 " + $data.curStock + "）" : ""),
    I: common_vendor.o((...args) => $options.dec && $options.dec(...args), "11"),
    J: common_vendor.t($data.qty),
    K: common_vendor.o((...args) => $options.inc && $options.inc(...args), "8a"),
    L: $data.intro,
    M: common_assets._imports_5,
    N: common_vendor.o((...args) => $options.consult && $options.consult(...args), "a7"),
    O: common_vendor.o((...args) => $options.addCart && $options.addCart(...args), "91"),
    P: common_vendor.o((...args) => $options.buyNow && $options.buyNow(...args), "a4"),
    Q: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    R: `${_ctx.u_s_b_h}px`,
    S: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/shop/detail.js.map
