"use strict";
const common_vendor = require("../common/vendor.js");
class Garden extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: String, optional: false },
          name: { type: String, optional: false },
          variety: { type: String, optional: false },
          altitude: { type: Number, optional: false },
          area: { type: Number, optional: false },
          treeAge: { type: Number, optional: false },
          pricePerShare: { type: Number, optional: false },
          sharesLeft: { type: Number, optional: false },
          totalShares: { type: Number, optional: false },
          cover: { type: String, optional: false },
          location: { type: String, optional: false },
          sales: { type: Number, optional: false },
          benefits: { type: common_vendor.UTS.UTSType.withGenerics(Array, [String]), optional: false },
          batch: { type: String, optional: false }
        };
      },
      name: "Garden"
    };
  }
  constructor(options, metadata = Garden.get$UTSMetadata$(), isJSONParse = false) {
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
    this.totalShares = this.__props__.totalShares;
    this.cover = this.__props__.cover;
    this.location = this.__props__.location;
    this.sales = this.__props__.sales;
    this.benefits = this.__props__.benefits;
    this.batch = this.__props__.batch;
    delete this.__props__;
  }
}
class Product extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          id: { type: String, optional: false },
          name: { type: String, optional: false },
          desc: { type: String, optional: false },
          price: { type: Number, optional: false },
          type: { type: String, optional: false },
          cover: { type: String, optional: false }
        };
      },
      name: "Product"
    };
  }
  constructor(options, metadata = Product.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.id = this.__props__.id;
    this.name = this.__props__.name;
    this.desc = this.__props__.desc;
    this.price = this.__props__.price;
    this.type = this.__props__.type;
    this.cover = this.__props__.cover;
    delete this.__props__;
  }
}
class AromaCard extends common_vendor.UTS.UTSType {
  static get$UTSMetadata$() {
    return {
      kind: 2,
      get fields() {
        return {
          name: { type: String, optional: false },
          subtitle: { type: String, optional: false },
          cover: { type: String, optional: false }
        };
      },
      name: "AromaCard"
    };
  }
  constructor(options, metadata = AromaCard.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.name = this.__props__.name;
    this.subtitle = this.__props__.subtitle;
    this.cover = this.__props__.cover;
    delete this.__props__;
  }
}
class TraceEvent extends common_vendor.UTS.UTSType {
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
      name: "TraceEvent"
    };
  }
  constructor(options, metadata = TraceEvent.get$UTSMetadata$(), isJSONParse = false) {
    super();
    this.__props__ = common_vendor.UTS.UTSType.initProps(options, metadata, isJSONParse);
    this.date = this.__props__.date;
    this.title = this.__props__.title;
    this.desc = this.__props__.desc;
    delete this.__props__;
  }
}
[
  new Garden({
    id: "tianfengshan",
    name: "饶平田峰山茶园",
    variety: "台地",
    altitude: 800,
    area: 30,
    treeAge: 25,
    pricePerShare: 1280,
    sharesLeft: 36,
    totalShares: 120,
    cover: "/static/slices/category-origin/card-tianfengshan.jpg",
    location: "潮州·饶平",
    sales: 320,
    benefits: ["年产约500g干茶", "专属木牌挂牌", "数字认养证", "全程溯源档案"],
    batch: "YN-2024-TF-0188"
  }),
  new Garden({
    id: "pingkengkou",
    name: "凤凰坪坑口古树园",
    variety: "古树",
    altitude: 980,
    area: 18,
    treeAge: 120,
    pricePerShare: 2680,
    sharesLeft: 12,
    totalShares: 80,
    cover: "/static/slices/category-origin/card-pingkengkou.jpg",
    location: "潮州·凤凰山",
    sales: 540,
    benefits: ["年产约500g干茶", "鸭屎香母树", "专属木牌挂牌", "数字认养证", "全程溯源档案"],
    batch: "YN-2024-PK-0061"
  }),
  new Garden({
    id: "fengxi-shiwen",
    name: "凤凰凤溪石瓮群落",
    variety: "古树",
    altitude: 1e3,
    area: 22,
    treeAge: 100,
    pricePerShare: 2980,
    sharesLeft: 0,
    totalShares: 60,
    cover: "/static/slices/category-origin/card-fengxi-shiwen.jpg",
    location: "潮州·凤凰山",
    sales: 600,
    benefits: ["年产约500g干茶", "百年古树群落", "专属木牌挂牌", "数字认养证", "全程溯源档案"],
    batch: "YN-2024-FX-0023"
  }),
  new Garden({
    id: "jianrao-organic",
    name: "饶平建饶有机茶园",
    variety: "台地",
    altitude: 600,
    area: 45,
    treeAge: 18,
    pricePerShare: 980,
    sharesLeft: 88,
    totalShares: 200,
    cover: "/static/slices/category-origin/card-jianrao-organic.jpg",
    location: "潮州·饶平",
    sales: 210,
    benefits: ["年产约500g干茶", "有机认证", "数字认养证", "全程溯源档案"],
    batch: "YN-2024-JR-0142"
  }),
  new Garden({
    id: "wudong",
    name: "凤凰乌栋传统山场",
    variety: "古树",
    altitude: 1100,
    area: 15,
    treeAge: 150,
    pricePerShare: 3680,
    sharesLeft: 5,
    totalShares: 40,
    cover: "/static/slices/category-origin/card-wudong.jpg",
    location: "潮州·凤凰山",
    sales: 720,
    benefits: ["年产约500g干茶", "150年老枞", "专属木牌挂牌", "数字认养证", "全程溯源档案"],
    batch: "YN-2024-WD-0009"
  })
];
[
  new Product({ id: "p1", name: "鸭屎香单丛 · 礼盒装", desc: "古树春茶 · 250g", price: 880, type: "实物", cover: "/static/slices/category-aroma/card-yashixiang.jpg" }),
  new Product({ id: "p2", name: "蜜兰香单丛 · 品鉴装", desc: "高香耐泡 · 100g", price: 360, type: "实物", cover: "/static/slices/category-aroma/card-milanxiang.jpg" }),
  new Product({ id: "p3", name: "凤凰山泉水票", desc: "12 期配送 · 虚拟权益", price: 240, type: "虚拟", cover: "/static/slices/category-aroma/card-shuixian.jpg" }),
  new Product({ id: "p4", name: "宋种水仙 · 珍藏装", desc: "老枞水仙 · 200g", price: 1280, type: "实物", cover: "/static/slices/category-aroma/card-zhilanxiang.jpg" }),
  new Product({ id: "p5", name: "芝兰香单丛 · 口粮装", desc: "日常自饮 · 500g", price: 480, type: "实物", cover: "/static/slices/category-aroma/card-huazhixiang.jpg" }),
  new Product({ id: "p6", name: "品鉴装兑换券", desc: "可兑换 15g 小样 · 支持转赠", price: 0, type: "虚拟", cover: "/static/slices/category-aroma/card-guihuaxiang.jpg" })
];
[
  new AromaCard({ name: "柚花香", subtitle: "清雅如柚花初绽", cover: "/static/slices/category-aroma/card-youhuaxiang.jpg" }),
  new AromaCard({ name: "鸭屎香", subtitle: "浓郁持久·回甘力强", cover: "/static/slices/category-aroma/card-yashixiang.jpg" }),
  new AromaCard({ name: "黄枝香", subtitle: "黄栀古韵·花蜜香显", cover: "/static/slices/category-aroma/card-huazhixiang.jpg" }),
  new AromaCard({ name: "蜜兰香", subtitle: "甜蜜幽兰·韵味悠长", cover: "/static/slices/category-aroma/card-milanxiang.jpg" }),
  new AromaCard({ name: "破头香", subtitle: "独特山场·韵深气足", cover: "/static/slices/category-aroma/card-potouxiang.jpg" }),
  new AromaCard({ name: "芝兰香", subtitle: "芝兰幽香·清雅高远", cover: "/static/slices/category-aroma/card-zhilanxiang.jpg" }),
  new AromaCard({ name: "大乌叶", subtitle: "叶大香醇·茶气强劲", cover: "/static/slices/category-aroma/card-daye.jpg" }),
  new AromaCard({ name: "水仙", subtitle: "老枞水仙·甘醇耐泡", cover: "/static/slices/category-aroma/card-shuixian.jpg" }),
  new AromaCard({ name: "锯朵仔", subtitle: "山野气韵·独特风味", cover: "/static/slices/category-aroma/card-juduozai.jpg" }),
  new AromaCard({ name: "凹富后", subtitle: "稀有名丛·韵味不凡", cover: "/static/slices/category-aroma/card-aofuhou.jpg" }),
  new AromaCard({ name: "桂花香", subtitle: "桂花清甜·馥郁怡人", cover: "/static/slices/category-aroma/card-guihuaxiang.jpg" })
];
[
  new TraceEvent({ date: "2024.03.12", title: "春茶萌芽", desc: "古树新芽初展，基地记录长势" }),
  new TraceEvent({ date: "2024.04.05", title: "明前采摘", desc: "人工手采一芽二叶，当日付制" }),
  new TraceEvent({ date: "2024.04.06", title: "传统做青", desc: "晒青·晾青·碰青，十道工艺古法制作" }),
  new TraceEvent({ date: "2024.04.10", title: "炭焙提香", desc: "荔枝炭文火慢焙，定型提香" }),
  new TraceEvent({ date: "2024.04.20", title: "SGS 质检", desc: "农残检测合格，质检报告已上传" })
];
const craftSteps = [
  new TraceEvent({ date: "01", title: "采青", desc: "明前手采一芽二三叶，当日付制" }),
  new TraceEvent({ date: "02", title: "晒青", desc: "日光萎凋，散失部分水分、激发香气" }),
  new TraceEvent({ date: "03", title: "晾青", desc: "移入室内摊晾，叶片回软、走水均匀" }),
  new TraceEvent({ date: "04", title: "做青", desc: "碰青·浪青反复数轮，形成花果香" }),
  new TraceEvent({ date: "05", title: "杀青", desc: "高温钝化酶活，定格香气与滋味" }),
  new TraceEvent({ date: "06", title: "揉捻", desc: "塑形成条，揉出茶汁、利于冲泡" }),
  new TraceEvent({ date: "07", title: "初烘", desc: "毛火初焙，固定外形、去除多余水分" }),
  new TraceEvent({ date: "08", title: "摊凉", desc: "回潮摊放，让内含物质重新平衡" }),
  new TraceEvent({ date: "09", title: "复焙", desc: "荔枝炭文火慢焙，去杂提香" }),
  new TraceEvent({ date: "10", title: "拣剔", desc: "人工拣剔归堆，分级窨藏待发" })
];
exports.craftSteps = craftSteps;
