"use strict";
const common_vendor = require("../common/vendor.js");
class ProductVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          cover: { type: String, optional: false },
          price: { type: Number, optional: false },
          otPrice: { type: Number, optional: false },
          sales: { type: Number, optional: false },
          unit: { type: String, optional: false },
          stock: { type: Number, optional: false }
        };
      },
      name: "ProductVM"
    };
  }
  constructor(options, metadata = ProductVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.cover = this.__props__.cover;
    this.price = this.__props__.price;
    this.otPrice = this.__props__.otPrice;
    this.sales = this.__props__.sales;
    this.unit = this.__props__.unit;
    this.stock = this.__props__.stock;
    delete this.__props__;
  }
}
function numOf(o, keys) {
  for (let i = 0; i < keys.length; i++) {
    const v = o.getNumber(keys[i]);
    if (v != null)
      return v;
    const s = o.getString(keys[i]);
    if (s != null && s.length > 0) {
      const p = parseFloat(s);
      if (p == p)
        return p;
    }
  }
  return 0;
}
function strOf(o, keys) {
  for (let i = 0; i < keys.length; i++) {
    const v = o.getString(keys[i]);
    if (v != null && v.length > 0)
      return v;
  }
  return "";
}
function fmtMoney(n) {
  return n.toFixed(2);
}
function toProduct(o) {
  return new ProductVM({
    id: numOf(o, ["id", "productId", "storeId"]),
    name: strOf(o, ["storeName", "store_name", "title", "name"]),
    cover: strOf(o, ["image", "storeImage", "cover"]),
    price: numOf(o, ["price", "storePrice"]),
    otPrice: numOf(o, ["otPrice", "ot_price", "productPrice"]),
    sales: numOf(o, ["sales", "fakeSales"]),
    unit: strOf(o, ["unitName", "unit_name", "unit"]),
    stock: numOf(o, ["stock", "productStock"])
  });
}
function toProductList(arr) {
  const out = [];
  for (let i = 0; i < arr.length; i++)
    out.push(toProduct(arr[i]));
  return out;
}
class OrderVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          orderId: { type: String, optional: false },
          statusText: { type: String, optional: false },
          status: { type: Number, optional: false },
          payPrice: { type: Number, optional: false },
          totalNum: { type: Number, optional: false },
          createTime: { type: String, optional: false },
          items: { type: common_vendor.UTS.UTSType.withGenerics(Array, [OrderItemVM]), optional: false },
          refundStatus: { type: Number, optional: false }
        };
      },
      name: "OrderVM"
    };
  }
  constructor(options, metadata = OrderVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.orderId = this.__props__.orderId;
    this.statusText = this.__props__.statusText;
    this.status = this.__props__.status;
    this.payPrice = this.__props__.payPrice;
    this.totalNum = this.__props__.totalNum;
    this.createTime = this.__props__.createTime;
    this.items = this.__props__.items;
    this.refundStatus = this.__props__.refundStatus;
    delete this.__props__;
  }
}
class OrderItemVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          name: { type: String, optional: false },
          cover: { type: String, optional: false },
          price: { type: Number, optional: false },
          num: { type: Number, optional: false }
        };
      },
      name: "OrderItemVM"
    };
  }
  constructor(options, metadata = OrderItemVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.name = this.__props__.name;
    this.cover = this.__props__.cover;
    this.price = this.__props__.price;
    this.num = this.__props__.num;
    delete this.__props__;
  }
}
function orderStatusText(o) {
  var _a;
  const paid = (_a = o.getBoolean("paid")) !== null && _a !== void 0 ? _a : false;
  const status = numOf(o, ["status"]);
  if (!paid)
    return "待付款";
  if (status == 0)
    return "待发货";
  if (status == 1)
    return "待收货";
  if (status == 2)
    return "待评价";
  if (status == 3)
    return "已完成";
  return "已完成";
}
function toOrder(o) {
  var _a, _b, _c;
  const items = [];
  const list = (_b = (_a = o["orderInfoList"]) !== null && _a !== void 0 ? _a : o["productList"]) !== null && _b !== void 0 ? _b : o["orderDetailList"];
  if (list != null && Array.isArray(list)) {
    const arr = list;
    for (let i = 0; i < arr.length; i++) {
      const it = arr[i];
      const info = (_c = it["productInfo"]) !== null && _c !== void 0 ? _c : it;
      items.push(new OrderItemVM({
        name: strOf(info, ["storeName", "productName", "store_name"]),
        cover: strOf(info, ["image", "productImage"]),
        price: numOf(it, ["price", "truePrice"]),
        num: numOf(it, ["payNum", "cartNum", "num"])
      }));
    }
  }
  return new OrderVM({
    orderId: strOf(o, ["orderId", "order_id", "orderNo"]),
    id: numOf(o, ["id"]),
    status: numOf(o, ["status"]),
    statusText: orderStatusText(o),
    payPrice: numOf(o, ["payPrice", "pay_price"]),
    totalNum: numOf(o, ["totalNum", "total_num"]),
    createTime: strOf(o, ["createTime", "add_time", "createDate"]),
    items,
    refundStatus: numOf(o, ["refundStatus", "refund_status"])
  });
}
class CartVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          productId: { type: Number, optional: false },
          name: { type: String, optional: false },
          cover: { type: String, optional: false },
          price: { type: Number, optional: false },
          num: { type: Number, optional: false },
          sku: { type: String, optional: false },
          valid: { type: Boolean, optional: false },
          checked: { type: Boolean, optional: false },
          stock: { type: Number, optional: false }
        };
      },
      name: "CartVM"
    };
  }
  constructor(options, metadata = CartVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.productId = this.__props__.productId;
    this.name = this.__props__.name;
    this.cover = this.__props__.cover;
    this.price = this.__props__.price;
    this.num = this.__props__.num;
    this.sku = this.__props__.sku;
    this.valid = this.__props__.valid;
    this.checked = this.__props__.checked;
    this.stock = this.__props__.stock;
    delete this.__props__;
  }
}
function toCart(o) {
  var _a, _b;
  const info = (_a = o["productInfo"]) !== null && _a !== void 0 ? _a : o;
  const attr = info["attrInfo"];
  let cover = strOf(info, ["image", "storeImage"]);
  let price = numOf(info, ["price"]);
  let stock = numOf(o, ["stock"]);
  if (attr != null) {
    const aImg = strOf(attr, ["image"]);
    if (aImg.length > 0)
      cover = aImg;
    const aPrice = numOf(attr, ["price"]);
    if (aPrice > 0)
      price = aPrice;
    const aStock = numOf(attr, ["stock"]);
    if (aStock > 0)
      stock = aStock;
  }
  return new CartVM({
    id: numOf(o, ["id"]),
    productId: numOf(o, ["productId", "product_id"]),
    name: strOf(info, ["storeName", "store_name"]),
    cover,
    price,
    num: numOf(o, ["cartNum", "cart_num"]),
    sku: strOf(o, ["productAttrUnique", "unique"]),
    valid: (_b = o.getBoolean("attrStatus")) !== null && _b !== void 0 ? _b : true,
    checked: true,
    stock
  });
}
class UserVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          nickname: { type: String, optional: false },
          avatar: { type: String, optional: false },
          balance: { type: Number, optional: false },
          integral: { type: Number, optional: false },
          coupons: { type: Number, optional: false },
          level: { type: String, optional: false }
        };
      },
      name: "UserVM"
    };
  }
  constructor(options, metadata = UserVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.nickname = this.__props__.nickname;
    this.avatar = this.__props__.avatar;
    this.balance = this.__props__.balance;
    this.integral = this.__props__.integral;
    this.coupons = this.__props__.coupons;
    this.level = this.__props__.level;
    delete this.__props__;
  }
}
function toUser(o) {
  return new UserVM({
    nickname: strOf(o, ["nickname", "nikeName", "name"]),
    avatar: strOf(o, ["avatar", "headImg"]),
    balance: numOf(o, ["nowMoney", "now_money", "balance"]),
    integral: numOf(o, ["integral", "integralCount"]),
    coupons: numOf(o, ["couponCount", "count"]),
    level: strOf(o, ["vipName", "levelName", "level_name"])
  });
}
class AddressVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          phone: { type: String, optional: false },
          region: { type: String, optional: false },
          detail: { type: String, optional: false },
          isDefault: { type: Boolean, optional: false }
        };
      },
      name: "AddressVM"
    };
  }
  constructor(options, metadata = AddressVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.phone = this.__props__.phone;
    this.region = this.__props__.region;
    this.detail = this.__props__.detail;
    this.isDefault = this.__props__.isDefault;
    delete this.__props__;
  }
}
function toAddress(o) {
  var _a;
  const province = strOf(o, ["province"]);
  const city = strOf(o, ["city"]);
  const district = strOf(o, ["district"]);
  return new AddressVM({
    id: numOf(o, ["id"]),
    name: strOf(o, ["realName", "real_name", "name"]),
    phone: strOf(o, ["phone"]),
    region: province + " " + city + " " + district,
    detail: strOf(o, ["detail"]),
    isDefault: (_a = o.getBoolean("isDefault")) !== null && _a !== void 0 ? _a : numOf(o, ["isDefault"]) == 1
  });
}
class AdoptRecordVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          gardenName: { type: String, optional: false },
          cover: { type: String, optional: false },
          shares: { type: Number, optional: false },
          period: { type: String, optional: false },
          batchNo: { type: String, optional: false },
          statusText: { type: String, optional: false },
          plateName: { type: String, optional: false },
          plateMsg: { type: String, optional: false },
          orderNo: { type: String, optional: false }
        };
      },
      name: "AdoptRecordVM"
    };
  }
  constructor(options, metadata = AdoptRecordVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.gardenName = this.__props__.gardenName;
    this.cover = this.__props__.cover;
    this.shares = this.__props__.shares;
    this.period = this.__props__.period;
    this.batchNo = this.__props__.batchNo;
    this.statusText = this.__props__.statusText;
    this.plateName = this.__props__.plateName;
    this.plateMsg = this.__props__.plateMsg;
    this.orderNo = this.__props__.orderNo;
    delete this.__props__;
  }
}
function toAdoptRecord(o) {
  var _a, _b;
  const garden = (_b = (_a = o["garden"]) !== null && _a !== void 0 ? _a : o["gardenInfo"]) !== null && _b !== void 0 ? _b : o;
  const start = strOf(o, ["periodStart", "startTime", "adoptStartTime"]);
  const end = strOf(o, ["periodEnd", "endTime", "adoptEndTime"]);
  let period = "";
  if (start.length > 0)
    period = end.length > 0 ? start + " - " + end : start;
  let cover = strOf(garden, ["coverImage", "cover", "image"]);
  return new AdoptRecordVM({
    id: numOf(o, ["id", "recordId"]),
    gardenName: strOf(o, ["gardenName"]).length > 0 ? strOf(o, ["gardenName"]) : strOf(garden, ["name"]),
    cover,
    shares: numOf(o, ["shareNum", "shares", "num"]),
    period,
    batchNo: strOf(o, ["certNo", "orderNo", "batchNo"]),
    statusText: adoptStatusText(numOf(o, ["status"])),
    plateName: strOf(o, ["plateName", "nickname"]),
    plateMsg: strOf(o, ["plateMessage", "message"]),
    orderNo: strOf(o, ["orderNo", "order_no"])
  });
}
function adoptStatusText(status) {
  if (status == 0)
    return "待支付";
  if (status == 1)
    return "认养中";
  if (status == 2)
    return "已到期";
  if (status == -1)
    return "已取消";
  return "认养中";
}
class CouponVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          amount: { type: String, optional: false },
          cond: { type: String, optional: false },
          name: { type: String, optional: false },
          expire: { type: String, optional: false }
        };
      },
      name: "CouponVM"
    };
  }
  constructor(options, metadata = CouponVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.amount = this.__props__.amount;
    this.cond = this.__props__.cond;
    this.name = this.__props__.name;
    this.expire = this.__props__.expire;
    delete this.__props__;
  }
}
function toCoupon(o) {
  const money = numOf(o, ["money", "couponPrice"]);
  const min = numOf(o, ["minPrice", "useMinPrice"]);
  return new CouponVM({
    id: numOf(o, ["id", "couponId"]),
    amount: money > 0 ? "" + money : "—",
    cond: min > 0 ? "满 " + min + " 可用" : "无门槛",
    name: strOf(o, ["couponTitle", "name", "title"]),
    expire: strOf(o, ["useEndTime", "endTime", "useEndTimeStr"])
  });
}
class VoucherVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          desc: { type: String, optional: false },
          status: { type: Number, optional: false }
        };
      },
      name: "VoucherVM"
    };
  }
  constructor(options, metadata = VoucherVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.desc = this.__props__.desc;
    this.status = this.__props__.status;
    delete this.__props__;
  }
}
function toVoucher(o) {
  return new VoucherVM({
    id: numOf(o, ["id", "voucherId"]),
    name: strOf(o, ["name", "title", "productName"]),
    desc: strOf(o, ["desc", "description", "remark"]),
    status: numOf(o, ["status"])
  });
}
class GardenVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          variety: { type: String, optional: false },
          altitude: { type: Number, optional: false },
          area: { type: String, optional: false },
          treeAge: { type: Number, optional: false },
          pricePerShare: { type: Number, optional: false },
          sharesLeft: { type: Number, optional: false },
          cover: { type: String, optional: false },
          location: { type: String, optional: false },
          sales: { type: Number, optional: false },
          benefits: { type: common_vendor.UTS.UTSType.withGenerics(Array, [String]), optional: false },
          weatherEui: { type: String, optional: false },
          cameraEui: { type: String, optional: false },
          traceCode: { type: String, optional: false }
        };
      },
      name: "GardenVM"
    };
  }
  constructor(options, metadata = GardenVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.variety = this.__props__.variety;
    this.altitude = this.__props__.altitude;
    this.area = this.__props__.area;
    this.treeAge = this.__props__.treeAge;
    this.pricePerShare = this.__props__.pricePerShare;
    this.sharesLeft = this.__props__.sharesLeft;
    this.cover = this.__props__.cover;
    this.location = this.__props__.location;
    this.sales = this.__props__.sales;
    this.benefits = this.__props__.benefits;
    this.weatherEui = this.__props__.weatherEui;
    this.cameraEui = this.__props__.cameraEui;
    this.traceCode = this.__props__.traceCode;
    delete this.__props__;
  }
}
function toGarden(o) {
  var _a;
  const total = numOf(o, ["totalShare"]);
  const sold = numOf(o, ["soldShare"]);
  let left = numOf(o, ["sharesLeft"]);
  if (left == 0 && total > 0)
    left = total - sold;
  const benefits = [];
  const benefitRaw = (_a = o["benefit"]) !== null && _a !== void 0 ? _a : o["benefits"];
  if (benefitRaw != null) {
    if (Array.isArray(benefitRaw)) {
      const arr = benefitRaw;
      for (let i = 0; i < arr.length; i++)
        benefits.push(arr[i]);
    } else {
      const s = "" + benefitRaw;
      const parts = s.split("\n");
      for (let i = 0; i < parts.length; i++) {
        const seg = parts[i].trim();
        if (seg.length > 0)
          benefits.push(seg);
      }
    }
  }
  let cover = strOf(o, ["coverImage", "cover", "image"]);
  if (cover.length == 0)
    cover = strOf(o, ["sliderImage"]);
  return new GardenVM({
    id: numOf(o, ["id"]),
    name: strOf(o, ["name", "gardenName"]),
    variety: strOf(o, ["variety"]),
    altitude: numOf(o, ["altitude"]),
    area: strOf(o, ["area"]),
    treeAge: numOf(o, ["treeAge"]),
    pricePerShare: numOf(o, ["price", "pricePerShare"]),
    sharesLeft: left,
    cover,
    location: strOf(o, ["location"]),
    sales: sold > 0 ? sold : numOf(o, ["sales"]),
    benefits,
    weatherEui: strOf(o, ["weatherEui"]),
    cameraEui: strOf(o, ["cameraEui"]),
    traceCode: strOf(o, ["traceCode"])
  });
}
function toGardenList(arr) {
  const out = [];
  for (let i = 0; i < arr.length; i++)
    out.push(toGarden(arr[i]));
  return out;
}
class PlanVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          intro: { type: String, optional: false },
          cover: { type: String, optional: false },
          price: { type: Number, optional: false },
          periods: { type: Number, optional: false },
          intervalDays: { type: Number, optional: false },
          spec: { type: String, optional: false },
          stock: { type: Number, optional: false }
        };
      },
      name: "PlanVM"
    };
  }
  constructor(options, metadata = PlanVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.intro = this.__props__.intro;
    this.cover = this.__props__.cover;
    this.price = this.__props__.price;
    this.periods = this.__props__.periods;
    this.intervalDays = this.__props__.intervalDays;
    this.spec = this.__props__.spec;
    this.stock = this.__props__.stock;
    delete this.__props__;
  }
}
function toPlan(o) {
  return new PlanVM({
    id: numOf(o, ["id"]),
    name: strOf(o, ["name", "title"]),
    intro: strOf(o, ["intro", "desc", "description"]),
    cover: strOf(o, ["coverImage", "cover", "image"]),
    price: numOf(o, ["price"]),
    periods: numOf(o, ["periods"]),
    intervalDays: numOf(o, ["intervalDays"]),
    spec: strOf(o, ["specPerDelivery", "spec"]),
    stock: numOf(o, ["stock"])
  });
}
class CategoryItemVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          title: { type: String, optional: false },
          subTitle: { type: String, optional: false },
          image: { type: String, optional: false },
          bgColor: { type: String, optional: false },
          linkType: { type: String, optional: false },
          linkValue: { type: String, optional: false }
        };
      },
      name: "CategoryItemVM"
    };
  }
  constructor(options, metadata = CategoryItemVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.title = this.__props__.title;
    this.subTitle = this.__props__.subTitle;
    this.image = this.__props__.image;
    this.bgColor = this.__props__.bgColor;
    this.linkType = this.__props__.linkType;
    this.linkValue = this.__props__.linkValue;
    delete this.__props__;
  }
}
class CategoryZoneVM extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          items: { type: common_vendor.UTS.UTSType.withGenerics(Array, [CategoryItemVM]), optional: false }
        };
      },
      name: "CategoryZoneVM"
    };
  }
  constructor(options, metadata = CategoryZoneVM.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.items = this.__props__.items;
    delete this.__props__;
  }
}
exports.CartVM = CartVM;
exports.GardenVM = GardenVM;
exports.OrderVM = OrderVM;
exports.ProductVM = ProductVM;
exports.UserVM = UserVM;
exports.fmtMoney = fmtMoney;
exports.numOf = numOf;
exports.toAddress = toAddress;
exports.toAdoptRecord = toAdoptRecord;
exports.toCart = toCart;
exports.toCoupon = toCoupon;
exports.toGarden = toGarden;
exports.toGardenList = toGardenList;
exports.toOrder = toOrder;
exports.toPlan = toPlan;
exports.toProduct = toProduct;
exports.toProductList = toProductList;
exports.toUser = toUser;
exports.toVoucher = toVoucher;
