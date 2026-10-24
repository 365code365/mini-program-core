"use strict";
/**
 * 离线演示数据（config/env.js 的 MOCK = true 时启用）。
 *
 * 每个接口返回的就是后端 `{ code: 200, data }` 里的 `data` 那一层，字段名与
 * api/mappers.js、各页面读取的键一一对应（storeName / image / otPrice ……），
 * 所以页面代码不用区分真假接口。图片全部指向包内 /static/mock 与 /static/gen。
 */
const IMG = "/static/mock/";

const PRODUCTS = [
  { id: 1001, storeName: "鸭屎香单丛 · 古树春茶", storeInfo: "坪坑口核心产区 · 银花香高扬", image: IMG + "p-yashixiang.jpg", price: 368, otPrice: 428, vipPrice: 338, sales: 2318, unitName: "罐", stock: 186, tasteTags: "银花清香,蜜韵回甘,耐冲泡", kw: "鸭屎香 口粮 日常 品鉴" },
  { id: 1002, storeName: "宋种茶王 · 老丛珍藏", storeInfo: "乌岽宋种后代 · 限量 88 罐", image: IMG + "p-songzhong.jpg", price: 1280, otPrice: 1580, vipPrice: 1180, sales: 386, unitName: "罐", stock: 42, tasteTags: "山韵悠长,老丛木质香,喉韵深", kw: "宋种 茶王 老丛 珍藏 送礼" },
  { id: 1003, storeName: "蜜兰香单丛 · 品鉴装", storeInfo: "甜蜜幽兰 · 新手首选", image: IMG + "p-milanxiang.jpg", price: 198, otPrice: 238, vipPrice: 0, sales: 4126, unitName: "罐", stock: 520, tasteTags: "蜜甜兰韵,柔和顺滑", kw: "蜜兰香 品鉴 新手 入门" },
  { id: 1004, storeName: "芝兰香单丛 · 高山清韵", storeInfo: "海拔 900 米 · 兰香幽远", image: IMG + "p-zhilanxiang.jpg", price: 268, otPrice: 318, vipPrice: 248, sales: 1752, unitName: "罐", stock: 260, tasteTags: "幽兰清芬,汤感清透", kw: "芝兰香 高山 日常" },
  { id: 1005, storeName: "黄枝香单丛 · 栀子花韵", storeInfo: "凤凰十大香型之首", image: IMG + "p-huangzhi.jpg", price: 238, otPrice: 288, vipPrice: 0, sales: 1290, unitName: "罐", stock: 300, tasteTags: "黄栀花香,甘润", kw: "黄枝香 日常 品鉴" },
  { id: 1006, storeName: "老丛水仙 · 茶王赛金奖", storeInfo: "斗茶赛金奖同批次", image: IMG + "p-laocong.jpg", price: 880, otPrice: 980, vipPrice: 820, sales: 612, unitName: "罐", stock: 68, tasteTags: "丛韵浓厚,木香,回甘持久", kw: "水仙 老丛 茶王 金奖" },
  { id: 1007, storeName: "云岫礼盒 · 双罐典藏", storeInfo: "鸭屎香 + 蜜兰香 · 送礼体面", image: IMG + "p-giftbox.jpg", price: 598, otPrice: 698, vipPrice: 568, sales: 980, unitName: "盒", stock: 150, tasteTags: "双香组合,礼盒包装", kw: "礼盒 送礼 茶礼 企业" },
  { id: 1008, storeName: "六香品鉴装 · 一次喝懂单丛", storeInfo: "六大香型 · 每款 8g", image: IMG + "p-tasting.jpg", price: 128, otPrice: 168, vipPrice: 0, sales: 5630, unitName: "盒", stock: 800, tasteTags: "六香对比,入门友好", kw: "品鉴 小样 新手 入门" }
];

const GARDENS = [
  { id: 1, name: "坪坑口 · 鸭屎香古树园", variety: "鸭屎香", altitude: 860, area: "凤凰山核心产区", treeAge: 120, price: 1980, totalShare: 60, soldShare: 43, coverImage: IMG + "g-pingkeng.jpg", location: "广东潮州 · 凤凰镇坪坑口", benefit: "每年 500g 春茶\n专属认养挂牌\n茶园实时直播\n采摘季邀约探园", weatherEui: "WS-PKK-01", cameraEui: "CAM-PKK-01", traceCode: "YN-2024-PK-0061" },
  { id: 2, name: "乌岽 · 宋种老丛园", variety: "宋种", altitude: 1150, area: "乌岽山顶", treeAge: 260, price: 4880, totalShare: 30, soldShare: 26, coverImage: IMG + "g-wudong.jpg", location: "广东潮州 · 乌岽村", benefit: "每年 300g 老丛春茶\n古树档案与证书\n茶王赛专属席位", weatherEui: "WS-WD-02", cameraEui: "CAM-WD-02", traceCode: "YN-2024-WD-0012" },
  { id: 3, name: "凤溪石瓮 · 蜜兰香园", variety: "蜜兰香", altitude: 1000, area: "凤溪石瓮", treeAge: 90, price: 1280, totalShare: 80, soldShare: 37, coverImage: IMG + "g-fengxi.jpg", location: "广东潮州 · 石古坪", benefit: "每年 500g 春茶\n专属挂牌\n节气茶礼", weatherEui: "WS-FX-03", cameraEui: "CAM-FX-03", traceCode: "YN-2024-FX-0033" },
  { id: 4, name: "田峰山 · 芝兰香生态园", variety: "芝兰香", altitude: 800, area: "饶平田峰山", treeAge: 60, price: 980, totalShare: 100, soldShare: 58, coverImage: IMG + "g-tianfeng.jpg", location: "广东饶平 · 田峰山", benefit: "每年 500g 春茶\n生态种植报告", weatherEui: "WS-TF-04", cameraEui: "CAM-TF-04", traceCode: "YN-2024-TF-0047" }
];

const ADDRESS = [
  { id: 11, realName: "林清和", phone: "138****6612", province: "广东省", city: "广州市", district: "天河区", detail: "珠江新城花城大道 88 号 1206", isDefault: true, cityId: 440106 },
  { id: 12, realName: "陈若溪", phone: "139****2380", province: "广东省", city: "潮州市", district: "湘桥区", detail: "凤凰镇茶韵路 18 号", isDefault: false, cityId: 445102 }
];

const USER = { uid: 8801, nickname: "云岫茶友 · 清和", avatar: IMG + "avatar.jpg", phone: "13800006612", nowMoney: 286.5, integral: 3260, couponCount: 4, vipName: "金叶会员", token: "mock-token" };

function page(list, pageNo, limit) {
  const p = pageNo > 0 ? pageNo : 1;
  const l = limit > 0 ? limit : 20;
  return { list: list.slice((p - 1) * l, p * l), total: list.length, page: p, limit: l, totalPage: Math.ceil(list.length / l) };
}
function prodByKeyword(kw) {
  if (!kw) return PRODUCTS;
  const hit = PRODUCTS.filter((p) => (p.storeName + p.kw + p.storeInfo).indexOf(kw) >= 0);
  return hit.length > 0 ? hit : PRODUCTS;
}
function prod(id) {
  return PRODUCTS.find((p) => p.id == id) || PRODUCTS[0];
}
function garden(id) {
  return GARDENS.find((g) => g.id == id) || GARDENS[0];
}
function orderItem(p, num) {
  return { productId: p.id, payNum: num, price: p.price, productInfo: { storeName: p.storeName, image: p.image }, storeName: p.storeName, image: p.image };
}
const ORDER_STATE = [
  { paid: false, status: 0 },
  { paid: true, status: 0 },
  { paid: true, status: 1 },
  { paid: true, status: 2 },
  { paid: true, status: 3 }
];
function order(id, type, items, day) {
  let total = 0, num = 0;
  items.forEach((it) => { total += it.price * it.payNum; num += it.payNum; });
  const st = ORDER_STATE[type] || ORDER_STATE[4];
  return {
    id, orderId: "YX2024" + (310000 + id), paid: st.paid, status: st.status, refundStatus: 0,
    payPrice: total, totalNum: num, createTime: "2024-03-" + day + " 10:2" + (id % 10),
    orderInfoList: items, realName: ADDRESS[0].realName, userPhone: ADDRESS[0].phone,
    userAddress: "广东省 广州市 天河区 " + ADDRESS[0].detail
  };
}
function ordersOf(type) {
  const base = [
    [orderItem(PRODUCTS[0], 2)],
    [orderItem(PRODUCTS[6], 1), orderItem(PRODUCTS[7], 1)],
    [orderItem(PRODUCTS[1], 1)]
  ];
  return base.map((items, i) => order(type * 10 + i + 1, type, items, 14 - i - type));
}

const PLANS = [
  { id: 1, name: "节气茶 · 年度订阅", intro: "二十四节气，每月寄一款当季单丛，附冲泡卡片", coverImage: IMG + "p-tasting.jpg", price: 1680, periods: 12, intervalDays: 30, specPerDelivery: "每期 2 罐 · 共 100g", stock: 88 },
  { id: 2, name: "鸭屎香 · 口粮季卡", intro: "坪坑口口粮茶按月到家，喝完不断档", coverImage: IMG + "p-yashixiang.jpg", price: 598, periods: 3, intervalDays: 30, specPerDelivery: "每期 250g", stock: 200 },
  { id: 3, name: "老丛珍藏 · 半年计划", intro: "宋种、老丛水仙轮换，适合老茶客", coverImage: IMG + "p-songzhong.jpg", price: 2880, periods: 6, intervalDays: 30, specPerDelivery: "每期 1 罐 · 50g", stock: 30 }
];

const CITY = [
  { name: "广东省", cityId: 44, child: [
    { name: "广州市", cityId: 4401, child: [{ name: "天河区", cityId: 440106 }, { name: "越秀区", cityId: 440104 }] },
    { name: "潮州市", cityId: 4451, child: [{ name: "湘桥区", cityId: 445102 }, { name: "潮安区", cityId: 445103 }] }
  ] },
  { name: "北京市", cityId: 11, child: [{ name: "北京市", cityId: 1101, child: [{ name: "朝阳区", cityId: 110105 }, { name: "海淀区", cityId: 110108 }] }] }
];

function orderInfoVo() {
  const items = [PRODUCTS[0], PRODUCTS[2]];
  const list = items.map((p, i) => ({ productId: p.id, productName: p.storeName, image: p.image, price: p.price, payNum: i + 1, sku: i == 0 ? "250g · 礼罐" : "100g · 品鉴罐" }));
  const pro = items[0].price + items[1].price * 2;
  const a = ADDRESS[0];
  return { proTotalFee: pro, couponFee: 30, freightFee: 0, payFee: pro - 30, userBalance: USER.nowMoney, userCouponId: 0, addressId: a.id, realName: a.realName, phone: a.phone, province: a.province, city: a.city, district: a.district, detail: a.detail, orderDetailList: list };
}

/** 路由表：[方法, 路径前缀, (query, 路径尾段) => data] —— 按书写顺序匹配，长前缀写在前面 */
const ROUTES = [
  ["GET", "/tea/ui/assets", () => ({})],
  ["GET", "/products", (q) => page(prodByKeyword(q.keyword), q.page, q.limit)],
  ["GET", "/product/detail/", (q, id) => {
    const p = prod(id);
    return {
      productInfo: Object.assign({}, p, {
        sliderImage: JSON.stringify([p.image, "/static/gen/detail-3.jpg", "/static/gen/origin-master.jpg"]),
        suited: "喜欢花香、追求回甘的茶友；日常自饮与待客皆宜",
        reason: "核心产区单株采制，十道传统工艺，香高水甜，七泡有余香",
        notFor: "空腹或睡前不宜多饮；肠胃敏感者建议淡泡",
        brew: "盖碗 110ml · 投茶 7g · 沸水快出汤，前三泡 5 秒内",
        content: "<p>凤凰单丛产自潮州凤凰山，一株一香。本品选自海拔 800 米以上古树，春茶头采，炭焙三遍，香气层层递进。</p>"
      }),
      productAttr: [{ attrName: "规格", attrValues: "250g · 礼罐,100g · 品鉴罐" }],
      productValue: {
        "100g · 品鉴罐": { id: p.id * 10 + 1, price: "" + Math.round(p.price * 0.45), otPrice: "" + Math.round(p.otPrice * 0.45), image: p.image, stock: p.stock },
        "250g · 礼罐": { id: p.id * 10 + 2, price: "" + p.price, otPrice: "" + p.otPrice, image: p.image, stock: p.stock }
      }
    };
  }],
  ["GET", "/search/keyword", () => ["鸭屎香", "宋种", "蜜兰香", "古树认养", "礼盒", "品鉴装"].map((k) => ({ keyword: k }))],
  ["GET", "/adopt/garden/list", (q) => page(q.variety ? GARDENS.filter((g) => g.variety == q.variety) : GARDENS, q.page, q.limit)],
  ["GET", "/adopt/garden/detail/", (q, id) => garden(id)],
  ["GET", "/adopt/my/list", (q) => page([
    { id: 501, gardenName: GARDENS[0].name, garden: GARDENS[0], shareNum: 2, periodStart: "2024-03-01", periodEnd: "2025-02-28", certNo: "YXC-2024-000501", orderNo: "AD2024030101", status: 1, plateName: "清和的茶树", plateMessage: "愿岁岁春茶，与家人同饮" },
    { id: 502, gardenName: GARDENS[1].name, garden: GARDENS[1], shareNum: 1, periodStart: "2023-03-01", periodEnd: "2024-02-29", certNo: "YXC-2023-000214", orderNo: "AD2023030188", status: 2, plateName: "宋种 · 一叶知秋", plateMessage: "" }
  ], q.page, q.limit)],
  ["GET", "/adopt/plate/info/", () => ({ nickname: "清和的茶树", message: "愿岁岁春茶，与家人同饮", status: 0 })],
  ["GET", "/adopt/cert/verify/", (q, no) => ({ certNo: "YXC-" + no, blockHash: "0x9f3a6c1e27b84d05a1c3" })],
  ["POST", "/adopt/create", () => ({ id: 503, orderNo: "AD" + Date.now() })],
  ["POST", "/adopt/pay", () => ({ status: "SUCCESS" })],
  ["GET", "/iot/weather/latest", () => ({ temperature: 18.6, humidity: 76, light: "适中", altitude: 980, updateTime: "今天 09:30" })],
  ["GET", "/iot/camera/play", () => ({})],
  ["GET", "/trace/archive/", () => ({
    gardenName: GARDENS[0].name, variety: "鸭屎香单丛",
    env: { temperature: "18.6°C", humidity: "76%", light: "适中" },
    timeline: [
      { date: "2024.03.18", title: "春茶开采", desc: "古树树叶人工采摘，一芽两叶" },
      { date: "2024.03.19", title: "传统工艺制作", desc: "十道工序古法制茶，炭焙三遍" },
      { date: "2024.04.20", title: "SGS 送检合格", desc: "农残 / 重金属检测通过" },
      { date: "2024.05.06", title: "入库封存", desc: "恒温恒湿仓储，一罐一码" }
    ]
  })],
  ["GET", "/cart/list", (q) => page([PRODUCTS[0], PRODUCTS[2], PRODUCTS[6]].map((p, i) => ({
    id: 300 + i, productId: p.id, cartNum: i == 1 ? 2 : 1, productAttrUnique: "" + (p.id * 10 + 2), attrStatus: true, stock: p.stock,
    productInfo: { storeName: p.storeName, image: p.image, price: p.price, attrInfo: { image: p.image, price: p.price, stock: p.stock } }
  })), q.page, q.limit)],
  ["POST", "/order/pre/order", () => ({ preOrderNo: "PRE20240315001", orderInfoVo: orderInfoVo() })],
  ["GET", "/order/load/pre/", () => ({ orderInfoVo: orderInfoVo() })],
  ["POST", "/order/computed/price", () => { const v = orderInfoVo(); return { proTotalFee: v.proTotalFee, couponFee: v.couponFee, freightFee: 0, payFee: v.payFee }; }],
  ["POST", "/order/create", () => ({ orderNo: "YX" + Date.now() })],
  ["POST", "/pay/payment", () => ({ status: "SUCCESS" })],
  ["GET", "/coupons/order/", () => [
    { id: 71, name: "新客满减券", money: 30, minPrice: 199, useEndTimeStr: "2024-06-30" },
    { id: 72, name: "春茶尝鲜券", money: 50, minPrice: 499, useEndTimeStr: "2024-05-31" }
  ]],
  ["GET", "/order/list", (q) => page(ordersOf(Number(q.type) || 0), q.page, q.limit)],
  ["GET", "/order/detail/", (q, id) => order(Number(id) || 41, 4, [orderItem(PRODUCTS[0], 2), orderItem(PRODUCTS[7], 1)], 12)],
  ["GET", "/order/refund/reason", () => ["不想要了", "商品缺货", "信息填写错误", "商品质量问题", "其他"]],
  ["GET", "/order/express/", () => ({
    expressName: "顺丰速运", expressNo: "SF1428 6630 2291",
    list: [
      { context: "已签收，感谢使用顺丰", time: "2024-03-14 15:20" },
      { context: "快件已到达【广州天河珠江新城营业点】", time: "2024-03-14 08:02" },
      { context: "顺丰速运已收取快件（潮州凤凰镇）", time: "2024-03-12 18:40" }
    ]
  })],
  ["GET", "/user/balance", () => ({ nowMoney: USER.nowMoney })],
  ["GET", "/user/sign/get", () => ({ sumSignDay: 5, isSign: false })],
  ["GET", "/user/sign/integral", () => ({ integral: 10 })],
  ["GET", "/user", () => USER],
  ["GET", "/integral/user", () => ({ integral: USER.integral })],
  ["GET", "/recharge/index", () => ({ rechargeQuota: [
    { id: 1, price: 100, giveMoney: 5 }, { id: 2, price: 300, giveMoney: 20 }, { id: 3, price: 500, giveMoney: 40 },
    { id: 4, price: 1000, giveMoney: 100 }, { id: 5, price: 2000, giveMoney: 240 }, { id: 6, price: 5000, giveMoney: 700 }
  ] })],
  ["GET", "/wechat/work/kefu", () => ({ title: "专属选茶顾问 · 阿岫", subtitle: "扫码添加，一对一推荐好茶", qrcodeImage: IMG + "kefu-qr.png" })],
  ["GET", "/coupon/list", (q) => page(Number(q.type) == 3 ? [
    { id: 61, money: 20, minPrice: 199, couponTitle: "春茶满减券（已过期）", useEndTime: "2024-02-29" }
  ] : [
    { id: 51, money: 30, minPrice: 199, couponTitle: "新客满减券", useEndTime: "2024-06-30" },
    { id: 52, money: 50, minPrice: 499, couponTitle: "春茶尝鲜券", useEndTime: "2024-05-31" },
    { id: 53, money: 10, minPrice: 0, couponTitle: "无门槛品鉴券", useEndTime: "2024-04-30" },
    { id: 54, money: 100, minPrice: 999, couponTitle: "茶王礼盒专享", useEndTime: "2024-12-31" }
  ], q.page, q.limit)],
  ["GET", "/tasting/voucher/list", (q) => page([
    { id: 81, name: "鸭屎香 · 15g 品鉴装", desc: "可到店兑换或邮寄到家", status: 0 },
    { id: 82, name: "蜜兰香 · 15g 品鉴装", desc: "支持转赠好友", status: 0 },
    { id: 83, name: "宋种 · 8g 小样", desc: "已于 2024-03-02 核销", status: 1 }
  ], q.page, q.limit)],
  ["GET", "/tasting/share/code", () => "YX8F2C"],
  ["GET", "/tasting/share/list", (q) => page([
    { receiveUid: 9012, createTime: "2024-03-10 20:16", status: 1, rewardStatus: 1, rewardIntegral: 50 }
  ], q.page, q.limit)],
  ["GET", "/tasting/", () => ({})],
  ["GET", "/address/list", (q) => page(ADDRESS, q.page, q.limit)],
  ["GET", "/address/default", () => ADDRESS[0]],
  ["GET", "/city/list", () => CITY],
  ["GET", "/subscribe/plan/list", () => PLANS],
  ["GET", "/subscribe/my/list", (q) => page([
    { id: 901, planName: PLANS[0].name, orderNo: "SB2024010101", status: 1, totalPeriods: 12, deliveredPeriods: 3, nextDeliveryDate: "2024-04-04" }
  ], q.page, q.limit)],
  ["POST", "/subscribe/create", () => ({ payType: "yue" })],
  ["GET", "/agreement/template/", () => ({ id: 1, name: "古树认养托管协议", content: "<p>甲方（认养人）认养乙方茶园内指定茶树份额，乙方负责日常管护、采摘与制作，并按约定交付成品茶。</p>" })],
  ["POST", "/ai/tea/recommend", () => ({
    summary: "按你「自己喝、偏花香、预算 300 内」的需求，挑了三款",
    consultantText: "第一次喝单丛，先从蜜兰香入门；想要香气更足，再试鸭屎香。",
    skillName: "选茶师 · 香型匹配",
    intent: "自饮", level: "入门", taste: "花香", budget: "300 元内",
    products: [PRODUCTS[2], PRODUCTS[0], PRODUCTS[3]].map((p, i) => ({
      id: p.id, name: p.storeName, subtitle: p.storeInfo, image: p.image, price: p.price,
      tasteTags: p.tasteTags.split(","),
      suited: ["初次接触单丛", "喜欢花香高扬", "偏爱清雅兰香"][i],
      reason: ["香气柔和、几乎不苦涩", "核心产区，回甘力强", "汤感清透，适合日常"][i],
      notFor: ["偏爱浓烈口感的老茶客", "睡前饮用", "喜欢厚重焙火"][i]
    }))
  })],
  ["POST", "/tea/consult/submit", () => "提交成功"],
  ["POST", "/login", () => USER],
  ["POST", "/sendCode", () => ({})],
  ["POST", "/wechat/", () => Object.assign({ type: "login" }, USER)],
  ["GET", "/logout", () => ({})]
];

/**
 * 返回 data（纯 JS 对象）；未登记的接口按「操作成功」处理（POST 返回 {}），
 * 演示模式里不出现「网络异常」。
 */
function mockData(method, url, query) {
  const qi = url.indexOf("?");
  const path = qi >= 0 ? url.substring(0, qi) : url;
  const q = Object.assign({}, query || {});
  if (qi >= 0) {
    url.substring(qi + 1).split("&").forEach((kv) => {
      const [k, v] = kv.split("=");
      if (k) q[k] = decodeURIComponent(v || "");
    });
  }
  for (let i = 0; i < ROUTES.length; i++) {
    const [m, prefix, fn] = ROUTES[i];
    if (m != method) continue;
    if (prefix.endsWith("/") ? path.startsWith(prefix) : (path == prefix || path.startsWith(prefix + "/"))) {
      return fn(q, path.substring(prefix.length));
    }
  }
  return {};
}
exports.mockData = mockData;
exports.MOCK_USER = USER;
