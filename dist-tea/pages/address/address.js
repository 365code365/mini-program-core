"use strict";
const common_vendor = require("../../common/vendor.js");
const api_address = require("../../api/address.js");
const api_mappers = require("../../api/mappers.js");
const store_selection = require("../../store/selection.js");
const utils_auth = require("../../utils/auth.js");
const common_assets = require("../../common/assets.js");
let CITY_TREE = [];
const _sfc_main = common_vendor.defineComponent({
  data() {
    return {
      list: [],
      loading: false,
      editing: false,
      saving: false,
      fId: 0,
      fName: "",
      fPhone: "",
      fRegion: "",
      fDetail: "",
      fDef: false,
      fProvince: "",
      fCity: "",
      fDistrict: "",
      fCityId: 0,
      selectMode: false,
      // 地区选择器（树存在模块级非响应缓存，data 里只放轻量字符串数组）
      regionPicking: false,
      regionLoading: false,
      provNames: [],
      cityNames: [],
      distNames: [],
      provIdx: -1,
      cityIdx: -1,
      step: 0,
      selProvinceName: "",
      selCityName: "",
      selDistrictName: ""
    };
  },
  computed: {
    curNames() {
      if (this.step == 0)
        return this.provNames;
      if (this.step == 1)
        return this.cityNames;
      return this.distNames;
    },
    curPickedName() {
      if (this.step == 0)
        return this.selProvinceName;
      if (this.step == 1)
        return this.selCityName;
      return this.selDistrictName;
    }
  },
  onLoad(opt) {
    if (!utils_auth.isLogin()) {
      common_vendor.index.redirectTo({ url: "/pages/login/login?redirect=/pages/address/address" });
      return null;
    }
    const sel = opt["select"];
    this.selectMode = sel != null && sel == "1";
    this.load();
  },
  methods: {
    noop() {
    },
    load() {
      this.loading = true;
      api_address.addressList().then((res) => {
        const vms = [];
        for (let i = 0; i < res.length; i++)
          vms.push(api_mappers.toAddress(res[i]));
        this.list = vms;
        this.loading = false;
      }).catch((_e = null) => {
        this.loading = false;
      });
    },
    openNew() {
      this.fId = 0;
      this.fName = "";
      this.fPhone = "";
      this.fRegion = "";
      this.fDetail = "";
      this.fDef = false;
      this.fProvince = "";
      this.fCity = "";
      this.fDistrict = "";
      this.fCityId = 0;
      this.editing = true;
    },
    edit(a) {
      this.fId = a.id;
      this.fName = a.name;
      this.fPhone = a.phone;
      this.fRegion = a.region;
      this.fDetail = a.detail;
      this.fDef = a.isDefault;
      this.editing = true;
      const parts = a.region.split(" ");
      this.fProvince = parts.length > 0 ? parts[0] : "";
      this.fCity = parts.length > 1 ? parts[1] : "";
      this.fDistrict = parts.length > 2 ? parts[2] : "";
      this.fCityId = 0;
    },
    // ===== 地区选择器 =====
    openRegion() {
      this.selProvinceName = this.fProvince;
      this.selCityName = this.fCity;
      this.selDistrictName = this.fDistrict;
      this.provIdx = -1;
      this.cityIdx = -1;
      this.cityNames = [];
      this.distNames = [];
      this.step = 0;
      this.regionPicking = true;
      if (CITY_TREE.length > 0) {
        this.buildProvinces();
        this.restoreSelection();
      } else {
        this.regionLoading = true;
        api_address.cityList().then((res) => {
          CITY_TREE = res;
          this.regionLoading = false;
          this.buildProvinces();
          this.restoreSelection();
        }).catch((_e = null) => {
          this.regionLoading = false;
        });
      }
    },
    namesOf(nodes) {
      var _a;
      const arr = [];
      for (let i = 0; i < nodes.length; i++)
        arr.push((_a = nodes[i].getString("name")) !== null && _a !== void 0 ? _a : "");
      return arr;
    },
    childrenOf(o) {
      const c = o["child"];
      if (c != null && Array.isArray(c))
        return c;
      return [];
    },
    buildProvinces() {
      this.provNames = this.namesOf(CITY_TREE);
    },
    // 编辑已有地址时，按名字回填已选层级，便于继续修改
    restoreSelection() {
      if (this.selProvinceName.length == 0)
        return null;
      const pi = this.provNames.indexOf(this.selProvinceName);
      if (pi < 0)
        return null;
      this.provIdx = pi;
      const cities = this.childrenOf(CITY_TREE[pi]);
      this.cityNames = this.namesOf(cities);
      this.step = 1;
      if (this.selCityName.length == 0)
        return null;
      const ci = this.cityNames.indexOf(this.selCityName);
      if (ci < 0)
        return null;
      this.cityIdx = ci;
      this.distNames = this.namesOf(this.childrenOf(cities[ci]));
      this.step = 2;
    },
    goStep(s) {
      if (s == 1 && this.selProvinceName.length == 0)
        return null;
      if (s == 2 && this.selCityName.length == 0)
        return null;
      this.step = s;
    },
    pickIdx(i) {
      var _a, _b, _c;
      if (this.step == 0) {
        const node = CITY_TREE[i];
        const name = (_a = node.getString("name")) !== null && _a !== void 0 ? _a : "";
        this.provIdx = i;
        this.selProvinceName = name;
        this.selCityName = "";
        this.selDistrictName = "";
        const cities = this.childrenOf(node);
        this.cityNames = this.namesOf(cities);
        this.distNames = [];
        if (cities.length > 0) {
          this.step = 1;
        } else {
          this.finishRegion(name, name, name, node);
        }
      } else if (this.step == 1) {
        const cities = this.childrenOf(CITY_TREE[this.provIdx]);
        const node = cities[i];
        const name = (_b = node.getString("name")) !== null && _b !== void 0 ? _b : "";
        this.cityIdx = i;
        this.selCityName = name;
        this.selDistrictName = "";
        const dists = this.childrenOf(node);
        this.distNames = this.namesOf(dists);
        if (dists.length > 0) {
          this.step = 2;
        } else {
          this.finishRegion(this.selProvinceName, name, name, node);
        }
      } else {
        const dists = this.childrenOf(this.childrenOf(CITY_TREE[this.provIdx])[this.cityIdx]);
        const node = dists[i];
        const name = (_c = node.getString("name")) !== null && _c !== void 0 ? _c : "";
        this.finishRegion(this.selProvinceName, this.selCityName, name, node);
      }
    },
    finishRegion(province, city, district, leaf) {
      var _a;
      this.fProvince = province;
      this.fCity = city;
      this.fDistrict = district;
      this.fCityId = (_a = leaf.getNumber("cityId")) !== null && _a !== void 0 ? _a : 0;
      this.fRegion = province + " " + city + " " + district;
      this.regionPicking = false;
    },
    save() {
      if (this.saving)
        return null;
      if (this.fName.length == 0 || this.fPhone.length == 0 || this.fDetail.length == 0) {
        common_vendor.index.showToast({ title: "请填写完整", icon: "none" });
        return null;
      }
      if (this.fProvince.length == 0 || this.fCity.length == 0 || this.fDistrict.length == 0) {
        common_vendor.index.showToast({ title: "请选择所在地区", icon: "none" });
        return null;
      }
      const addrObj = new common_vendor.UTSJSONObject({ province: this.fProvince, city: this.fCity, district: this.fDistrict, cityId: this.fCityId });
      const body = new common_vendor.UTSJSONObject({
        realName: this.fName,
        phone: this.fPhone,
        detail: this.fDetail,
        isDefault: this.fDef,
        address: addrObj
      });
      if (this.fId > 0)
        body["id"] = this.fId;
      this.saving = true;
      api_address.addressEdit(body).then((_d) => {
        this.saving = false;
        this.editing = false;
        this.load();
      }).catch((_e = null) => {
        this.saving = false;
      });
    },
    del(id) {
      api_address.addressDel(id).then((_d) => {
        this.load();
      }).catch((_e = null) => {
      });
    },
    setDefault(id) {
      api_address.addressSetDefault(id).then((_d) => {
        this.load();
      }).catch((_e = null) => {
      });
    },
    chooseAddr(a) {
      if (!this.selectMode)
        return null;
      store_selection.setPickedAddress(new store_selection.PickedAddress({ id: a.id, name: a.name, phone: a.phone, region: a.region, detail: a.detail }));
      common_vendor.index.navigateBack(new common_vendor.UTSJSONObject({ delta: 1 }));
    }
  }
});
function _sfc_render(_ctx, _cache, $props, $setup, $data, $options) {
  "raw js";
  return common_vendor.e({
    a: $data.selectMode
  }, $data.selectMode ? {} : {}, {
    b: !$data.loading && $data.list.length == 0
  }, !$data.loading && $data.list.length == 0 ? {} : {}, {
    c: common_vendor.f($data.list, (a, i, i0) => {
      return common_vendor.e({
        a: common_vendor.t(a.name),
        b: common_vendor.t(a.phone),
        c: a.isDefault
      }, a.isDefault ? {} : {}, $data.selectMode ? {} : {}, {
        d: common_vendor.t(a.region),
        e: common_vendor.t(a.detail),
        f: common_vendor.o(($event) => $options.chooseAddr(a), i),
        g: common_vendor.n(a.isDefault ? "cb-on" : ""),
        h: common_vendor.o(($event) => $options.setDefault(a.id), i),
        i: common_vendor.o(($event) => $options.edit(a), i),
        j: common_vendor.o(($event) => $options.del(a.id), i),
        k: i
      });
    }),
    d: $data.selectMode,
    e: common_assets._imports_0$6,
    f: common_vendor.o((...args) => $options.openNew && $options.openNew(...args), "33"),
    g: $data.editing
  }, $data.editing ? {
    h: common_vendor.t($data.fId == 0 ? "新增地址" : "编辑地址"),
    i: common_assets._imports_1$4,
    j: common_vendor.o(($event) => $data.editing = false, "6a"),
    k: $data.fName,
    l: common_vendor.o(($event) => $data.fName = $event.detail.value, "86"),
    m: $data.fPhone,
    n: common_vendor.o(($event) => $data.fPhone = $event.detail.value, "43"),
    o: common_vendor.t($data.fRegion.length == 0 ? "请选择省 / 市 / 区" : $data.fRegion),
    p: common_vendor.n($data.fRegion.length == 0 ? "picker-ph" : ""),
    q: common_assets._imports_2$4,
    r: common_vendor.o((...args) => $options.openRegion && $options.openRegion(...args), "2c"),
    s: $data.fDetail,
    t: common_vendor.o(($event) => $data.fDetail = $event.detail.value, "c7"),
    v: common_assets._imports_0$6,
    w: common_vendor.n($data.fDef ? "cb-on" : ""),
    x: common_vendor.o(($event) => $data.fDef = !$data.fDef, "75"),
    y: common_vendor.t($data.saving ? "保存中…" : "保存"),
    z: common_vendor.o((...args) => $options.save && $options.save(...args), "36"),
    A: common_vendor.o((...args) => $options.noop && $options.noop(...args), "df"),
    B: common_vendor.o(($event) => $data.editing = false, "9f")
  } : {}, {
    C: $data.regionPicking
  }, $data.regionPicking ? common_vendor.e({
    D: common_assets._imports_1$4,
    E: common_vendor.o(($event) => $data.regionPicking = false, "34"),
    F: common_vendor.t($data.selProvinceName.length == 0 ? "请选择" : $data.selProvinceName),
    G: common_vendor.n($data.step == 0 ? "rg-tab-t-on" : ""),
    H: common_vendor.n($data.step == 0 ? "rg-tab-on" : ""),
    I: common_vendor.o(($event) => $options.goStep(0), "12"),
    J: $data.selProvinceName.length > 0
  }, $data.selProvinceName.length > 0 ? {
    K: common_vendor.t($data.selCityName.length == 0 ? "请选择" : $data.selCityName),
    L: common_vendor.n($data.step == 1 ? "rg-tab-t-on" : ""),
    M: common_vendor.n($data.step == 1 ? "rg-tab-on" : ""),
    N: common_vendor.o(($event) => $options.goStep(1), "6b")
  } : {}, {
    O: $data.selCityName.length > 0
  }, $data.selCityName.length > 0 ? {
    P: common_vendor.t($data.selDistrictName.length == 0 ? "请选择" : $data.selDistrictName),
    Q: common_vendor.n($data.step == 2 ? "rg-tab-t-on" : ""),
    R: common_vendor.n($data.step == 2 ? "rg-tab-on" : ""),
    S: common_vendor.o(($event) => $options.goStep(2), "e7")
  } : {}, {
    T: $data.regionLoading
  }, $data.regionLoading ? {} : {}, {
    U: common_vendor.f($options.curNames, (name, i, i0) => {
      return common_vendor.e({
        a: common_vendor.t(name),
        b: common_vendor.n(name == $options.curPickedName ? "rg-item-t-on" : ""),
        c: name == $options.curPickedName
      }, name == $options.curPickedName ? {
        d: common_assets._imports_3$1
      } : {}, {
        e: i,
        f: common_vendor.o(($event) => $options.pickIdx(i), i)
      });
    }),
    V: common_vendor.o((...args) => $options.noop && $options.noop(...args), "fc"),
    W: common_vendor.o(($event) => $data.regionPicking = false, "1c")
  }) : {}, {
    X: common_vendor.sei(common_vendor.gei(_ctx, ""), "view"),
    Y: `${_ctx.u_s_b_h}px`,
    Z: common_vendor.pvhc(_ctx.$scope.data.virtualHostClass)
  });
}
const MiniProgramPage = /* @__PURE__ */ common_vendor._export_sfc(_sfc_main, [["render", _sfc_render]]);
wx.createPage(MiniProgramPage);
//# sourceMappingURL=../../../.sourcemap/mp-weixin/pages/address/address.js.map
