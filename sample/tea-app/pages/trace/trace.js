"use strict";
const common_vendor = require("../../common/vendor.js");
const api_iot = require("../../api/iot.js");
const common_assets = require("../../common/assets.js");
class Row extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          k: { type: String, optional: false },
          v: { type: String, optional: false }
        };
      },
      name: "Row"
    };
  }
  constructor(options, metadata = Row.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.k = this.__props__.k;
    this.v = this.__props__.v;
    delete this.__props__;
  }
}
class Ev extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          date: { type: String, optional: false },
          title: { type: String, optional: false },
          desc: { type: String, optional: false }
        };
      },
      name: "Ev"
    };
  }
  constructor(options, metadata = Ev.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.date = this.__props__.date;
    this.title = this.__props__.title;
    this.desc = this.__props__.desc;
    delete this.__props__;
  }
}
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      batch: "YN-2024-PK-0061",
      gardenName: "凤凰坪坑口古树园",
      variety: "鸭屎香单丛",
      temp: "18.6°C",
      humidity: "76%",
      light: "适中",
      reportNo: "编号 SGS-2024-CHA-0061",
      showReport: false,
      events: [
        new Ev({ date: "2024.03.18", title: "春茶开采", desc: "古树鲜叶人工采摘，一芽两叶" }),
        new Ev({ date: "2024.03.19", title: "传统工艺制作", desc: "十道工序古法制茶" }),
        new Ev({ date: "2024.04.20", title: "SGS 送检合格", desc: "农残/重金属检测通过" })
      ],
      rows: [
        new Row({ k: "报告编号", v: "SGS-2024-CHA-0061" }),
        new Row({ k: "送检样品", v: "凤凰坪坑口 · 鸭屎香单丛" }),
        new Row({ k: "检测机构", v: "SGS 通标标准技术服务" }),
        new Row({ k: "农药残留", v: "未检出（符合 GB 2763）" }),
        new Row({ k: "重金属（铅）", v: "＜0.5 mg/kg 合格" }),
        new Row({ k: "检测结论", v: "符合食品安全国家标准" }),
        new Row({ k: "报告日期", v: "2024-04-20" })
      ]
    };
  },
  onLoad(opt) {
    const b = opt["batch"];
    if (b != null)
      this.batch = b;
    this.load();
  },
  methods: {
    noop() {
    },
    load() {
      api_iot.traceArchive(this.batch).then((d) => {
        var _a, _b, _c, _d, _f, _g, _h, _j, _k, _l, _m;
        const gn = (_a = d.getString("gardenName")) !== null && _a !== void 0 ? _a : "";
        if (gn.length > 0)
          this.gardenName = gn;
        const va = (_b = d.getString("variety")) !== null && _b !== void 0 ? _b : "";
        if (va.length > 0)
          this.variety = va;
        const env = d["env"];
        if (env != null) {
          const t = (_c = env.getString("temperature")) !== null && _c !== void 0 ? _c : "";
          if (t.length > 0)
            this.temp = t;
          const h_1 = (_d = env.getString("humidity")) !== null && _d !== void 0 ? _d : "";
          if (h_1.length > 0)
            this.humidity = h_1;
          const l = (_f = env.getString("light")) !== null && _f !== void 0 ? _f : "";
          if (l.length > 0)
            this.light = l;
        }
        const tl = (_g = d["timeline"]) !== null && _g !== void 0 ? _g : d["events"];
        if (tl != null && Array.isArray(tl)) {
          const arr = tl;
          const evs = [];
          for (let i = 0; i < arr.length; i++) {
            evs.push(new Ev({
              date: (_h = arr[i].getString("date")) !== null && _h !== void 0 ? _h : (_j = arr[i].getString("time")) !== null && _j !== void 0 ? _j : "",
              title: (_k = arr[i].getString("title")) !== null && _k !== void 0 ? _k : "",
              desc: (_l = arr[i].getString("desc")) !== null && _l !== void 0 ? _l : (_m = arr[i].getString("content")) !== null && _m !== void 0 ? _m : ""
            }));
          }
          if (evs.length > 0)
            this.events = evs;
        }
      }).catch((_e = null) => {
      });
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: common_vendor.t($data.variety),
    b: common_vendor.t($data.gardenName),
    c: common_vendor.t($data.batch),
    d: common_vendor.t($data.temp),
    e: common_vendor.t($data.humidity),
    f: common_vendor.t($data.light),
    g: common_vendor.f($data.events, (e, i, i0) => {
      return {
        a: common_vendor.t(e.date),
        b: common_vendor.t(e.title),
        c: common_vendor.t(e.desc),
        d: i
      };
    }),
    h: common_assets._imports_0$9,
    i: common_vendor.t($data.reportNo),
    j: common_vendor.o(($event) => $data.showReport = true, "b3"),
    k: $data.showReport
  }, $data.showReport ? {
    l: common_assets._imports_1$4,
    m: common_vendor.o(($event) => $data.showReport = false, "bf"),
    n: common_assets._imports_0$5,
    o: common_vendor.f($data.rows, (r, i, i0) => {
      return {
        a: common_vendor.t(r.k),
        b: common_vendor.t(r.v),
        c: i
      };
    }),
    p: common_vendor.o((...args) => $options.noop && $options.noop(...args), "08"),
    q: common_vendor.o(($event) => $data.showReport = false, "b6")
  } : {}, {
    r: common_vendor.sei(common_vendor.gei(_ctx, ""), "scroll-view"),
    s: `${_ctx.u_s_b_h}px`,
    t: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
