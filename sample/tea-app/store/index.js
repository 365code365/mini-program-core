"use strict";
const common_vendor = require("../common/vendor.js");
require("./data.js");
class CartItem extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: String, optional: false },
          name: { type: String, optional: false },
          desc: { type: String, optional: false },
          price: { type: Number, optional: false },
          cover: { type: String, optional: false },
          qty: { type: Number, optional: false },
          checked: { type: Boolean, optional: false }
        };
      },
      name: "CartItem"
    };
  }
  constructor(options, metadata = CartItem.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.desc = this.__props__.desc;
    this.price = this.__props__.price;
    this.cover = this.__props__.cover;
    this.qty = this.__props__.qty;
    this.checked = this.__props__.checked;
    delete this.__props__;
  }
}
class OrderItem extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          name: { type: String, optional: false },
          price: { type: Number, optional: false },
          qty: { type: Number, optional: false },
          cover: { type: String, optional: false }
        };
      },
      name: "OrderItem"
    };
  }
  constructor(options, metadata = OrderItem.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.name = this.__props__.name;
    this.price = this.__props__.price;
    this.qty = this.__props__.qty;
    this.cover = this.__props__.cover;
    delete this.__props__;
  }
}
class Order extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: String, optional: false },
          createdAt: { type: Number, optional: false },
          items: { type: common_vendor.UTS.UTSType.withGenerics(Array, [OrderItem]), optional: false },
          total: { type: Number, optional: false },
          status: { type: String, optional: false }
        };
      },
      name: "Order"
    };
  }
  constructor(options, metadata = Order.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.createdAt = this.__props__.createdAt;
    this.items = this.__props__.items;
    this.total = this.__props__.total;
    this.status = this.__props__.status;
    delete this.__props__;
  }
}
class Address extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: String, optional: false },
          name: { type: String, optional: false },
          phone: { type: String, optional: false },
          region: { type: String, optional: false },
          detail: { type: String, optional: false },
          def: { type: Boolean, optional: false }
        };
      },
      name: "Address"
    };
  }
  constructor(options, metadata = Address.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.phone = this.__props__.phone;
    this.region = this.__props__.region;
    this.detail = this.__props__.detail;
    this.def = this.__props__.def;
    delete this.__props__;
  }
}
class StoreState extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          items: { type: common_vendor.UTS.UTSType.withGenerics(Array, [CartItem]), optional: false },
          orders: { type: common_vendor.UTS.UTSType.withGenerics(Array, [Order]), optional: false },
          addresses: { type: common_vendor.UTS.UTSType.withGenerics(Array, [Address]), optional: false },
          balance: { type: Number, optional: false },
          points: { type: Number, optional: false },
          coupons: { type: Number, optional: false },
          tastingCards: { type: Number, optional: false },
          userName: { type: String, optional: false }
        };
      },
      name: "StoreState"
    };
  }
  constructor(options, metadata = StoreState.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.items = this.__props__.items;
    this.orders = this.__props__.orders;
    this.addresses = this.__props__.addresses;
    this.balance = this.__props__.balance;
    this.points = this.__props__.points;
    this.coupons = this.__props__.coupons;
    this.tastingCards = this.__props__.tastingCards;
    this.userName = this.__props__.userName;
    delete this.__props__;
  }
}
const CART_KEY = "yanbin_cart";
const ORDER_KEY = "yanbin_orders";
const ADDR_KEY = "yanbin_addresses";
const store = common_vendor.reactive(new StoreState({
  items: [],
  orders: [],
  addresses: [
    new Address({ id: "addr-default", name: "四方茶友", phone: "138****0188", region: "广东省 潮州市 湘桥区", detail: "××路 88 号茶香苑 1 栋", def: true })
  ],
  balance: 1200,
  points: 860,
  coupons: 3,
  tastingCards: 2,
  userName: "四方茶友"
}));
function initStore() {
  const c = common_vendor.index.getStorageSync(CART_KEY);
  if (typeof c == "string" && c.length > 0) {
    store.items = common_vendor.UTS.JSON.parse(c);
  }
  const o = common_vendor.index.getStorageSync(ORDER_KEY);
  if (typeof o == "string" && o.length > 0) {
    store.orders = common_vendor.UTS.JSON.parse(o);
  }
  const a = common_vendor.index.getStorageSync(ADDR_KEY);
  if (typeof a == "string" && a.length > 0) {
    store.addresses = common_vendor.UTS.JSON.parse(a);
  }
}
exports.initStore = initStore;
