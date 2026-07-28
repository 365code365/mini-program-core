"use strict";
const common_vendor = require("../common/vendor.js");
class Page extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$(T) {
    return {
      kind: 2,
      get fields() {
        return {
          total: { type: Number, optional: false },
          list: { type: common_vendor.UTS.UTSType.withGenerics(Array, ["Unknown"]), optional: false }
        };
      },
      name: "Page"
    };
  }
  constructor(options, metadata = Page.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.total = this.__props__.total;
    this.list = this.__props__.list;
    delete this.__props__;
  }
}
class GardenDTO extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          variety: { type: String, optional: false },
          altitude: { type: Number, optional: false },
          treeAge: { type: Number, optional: false },
          area: { type: Number, optional: false },
          pricePerShare: { type: Number, optional: false },
          sharesLeft: { type: Number, optional: false },
          cover: { type: String, optional: false },
          location: { type: String, optional: false },
          sales: { type: Number, optional: false },
          batch: { type: String, optional: false },
          benefits: { type: common_vendor.UTS.UTSType.withGenerics(Array, [String]), optional: false }
        };
      },
      name: "GardenDTO"
    };
  }
  constructor(options, metadata = GardenDTO.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.variety = this.__props__.variety;
    this.altitude = this.__props__.altitude;
    this.treeAge = this.__props__.treeAge;
    this.area = this.__props__.area;
    this.pricePerShare = this.__props__.pricePerShare;
    this.sharesLeft = this.__props__.sharesLeft;
    this.cover = this.__props__.cover;
    this.location = this.__props__.location;
    this.sales = this.__props__.sales;
    this.batch = this.__props__.batch;
    this.benefits = this.__props__.benefits;
    delete this.__props__;
  }
}
class ProductDTO extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          storeName: { type: String, optional: false },
          image: { type: String, optional: false },
          price: { type: Number, optional: false },
          otPrice: { type: Number, optional: false },
          sales: { type: Number, optional: false },
          unitName: { type: String, optional: false }
        };
      },
      name: "ProductDTO"
    };
  }
  constructor(options, metadata = ProductDTO.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.storeName = this.__props__.storeName;
    this.image = this.__props__.image;
    this.price = this.__props__.price;
    this.otPrice = this.__props__.otPrice;
    this.sales = this.__props__.sales;
    this.unitName = this.__props__.unitName;
    delete this.__props__;
  }
}
class CartItemDTO extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          productId: { type: Number, optional: false },
          storeName: { type: String, optional: false },
          image: { type: String, optional: false },
          price: { type: Number, optional: false },
          cartNum: { type: Number, optional: false },
          attrStatus: { type: Boolean, optional: false }
        };
      },
      name: "CartItemDTO"
    };
  }
  constructor(options, metadata = CartItemDTO.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.productId = this.__props__.productId;
    this.storeName = this.__props__.storeName;
    this.image = this.__props__.image;
    this.price = this.__props__.price;
    this.cartNum = this.__props__.cartNum;
    this.attrStatus = this.__props__.attrStatus;
    delete this.__props__;
  }
}
class OrderDTO extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          orderId: { type: String, optional: false },
          status: { type: Number, optional: false },
          payPrice: { type: Number, optional: false },
          totalNum: { type: Number, optional: false },
          createTime: { type: String, optional: false },
          productList: { type: common_vendor.UTS.UTSType.withGenerics(Array, [OrderItemDTO]), optional: false }
        };
      },
      name: "OrderDTO"
    };
  }
  constructor(options, metadata = OrderDTO.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.orderId = this.__props__.orderId;
    this.status = this.__props__.status;
    this.payPrice = this.__props__.payPrice;
    this.totalNum = this.__props__.totalNum;
    this.createTime = this.__props__.createTime;
    this.productList = this.__props__.productList;
    delete this.__props__;
  }
}
class OrderItemDTO extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          storeName: { type: String, optional: false },
          image: { type: String, optional: false },
          price: { type: Number, optional: false },
          cartNum: { type: Number, optional: false }
        };
      },
      name: "OrderItemDTO"
    };
  }
  constructor(options, metadata = OrderItemDTO.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.storeName = this.__props__.storeName;
    this.image = this.__props__.image;
    this.price = this.__props__.price;
    this.cartNum = this.__props__.cartNum;
    delete this.__props__;
  }
}
class UserDTO extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          uid: { type: Number, optional: false },
          nickname: { type: String, optional: false },
          avatar: { type: String, optional: false },
          nowMoney: { type: Number, optional: false },
          integral: { type: Number, optional: false },
          couponCount: { type: Number, optional: false }
        };
      },
      name: "UserDTO"
    };
  }
  constructor(options, metadata = UserDTO.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.uid = this.__props__.uid;
    this.nickname = this.__props__.nickname;
    this.avatar = this.__props__.avatar;
    this.nowMoney = this.__props__.nowMoney;
    this.integral = this.__props__.integral;
    this.couponCount = this.__props__.couponCount;
    delete this.__props__;
  }
}
class AddressDTO extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          realName: { type: String, optional: false },
          phone: { type: String, optional: false },
          province: { type: String, optional: false },
          city: { type: String, optional: false },
          district: { type: String, optional: false },
          detail: { type: String, optional: false },
          isDefault: { type: Boolean, optional: false }
        };
      },
      name: "AddressDTO"
    };
  }
  constructor(options, metadata = AddressDTO.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.realName = this.__props__.realName;
    this.phone = this.__props__.phone;
    this.province = this.__props__.province;
    this.city = this.__props__.city;
    this.district = this.__props__.district;
    this.detail = this.__props__.detail;
    this.isDefault = this.__props__.isDefault;
    delete this.__props__;
  }
}
class MyGardenWeatherDTO extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          gardenId: { type: Number, optional: false },
          gardenName: { type: String, optional: false },
          recordId: { type: Number, optional: false },
          batchNo: { type: String, optional: false },
          temperature: { type: String, optional: false },
          humidity: { type: String, optional: false },
          light: { type: String, optional: false },
          altitude: { type: Number, optional: false },
          online: { type: Boolean, optional: false }
        };
      },
      name: "MyGardenWeatherDTO"
    };
  }
  constructor(options, metadata = MyGardenWeatherDTO.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.gardenId = this.__props__.gardenId;
    this.gardenName = this.__props__.gardenName;
    this.recordId = this.__props__.recordId;
    this.batchNo = this.__props__.batchNo;
    this.temperature = this.__props__.temperature;
    this.humidity = this.__props__.humidity;
    this.light = this.__props__.light;
    this.altitude = this.__props__.altitude;
    this.online = this.__props__.online;
    delete this.__props__;
  }
}
class SubscribePlanDTO extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: Number, optional: false },
          name: { type: String, optional: false },
          desc: { type: String, optional: false },
          coverImage: { type: String, optional: false },
          price: { type: Number, optional: false },
          periods: { type: Number, optional: false },
          intervalDays: { type: Number, optional: false },
          specPerDelivery: { type: String, optional: false },
          status: { type: Number, optional: false }
        };
      },
      name: "SubscribePlanDTO"
    };
  }
  constructor(options, metadata = SubscribePlanDTO.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.desc = this.__props__.desc;
    this.coverImage = this.__props__.coverImage;
    this.price = this.__props__.price;
    this.periods = this.__props__.periods;
    this.intervalDays = this.__props__.intervalDays;
    this.specPerDelivery = this.__props__.specPerDelivery;
    this.status = this.__props__.status;
    delete this.__props__;
  }
}
//# sourceMappingURL=../../.sourcemap/mp-weixin/api/models.js.map
