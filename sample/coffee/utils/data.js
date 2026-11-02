// 拾光咖啡 · 离线菜单数据（价格单位：元，price 为中杯价）
const IMG = '/images/drinks/';
const categories = [
  { id: 'new', name: '当季新品' },
  { id: 'latte', name: '拿铁家族' },
  { id: 'americano', name: '美式' },
  { id: 'fruit', name: '果咖·轻乳' },
  { id: 'classic', name: '经典意式' }
];
// milk: 含奶，才能「换燕麦奶」
const drinks = [
  { id: 'k1', cat: 'new', name: '橙C美式', en: 'Orange C Americano', img: IMG + 'orange.png', price: 19, origin: 29, sales: '月售 2.1 万', desc: '鲜榨橙汁 + 埃塞俄比亚日晒豆，酸甜清爽', temps: ['冰', '少冰', '去冰'], tag: '新品', milk: false },
  { id: 'k2', cat: 'new', name: '桃桃乌龙拿铁', en: 'Peach Oolong Latte', img: IMG + 'peach.png', price: 22, origin: 32, sales: '月售 9800', desc: '白桃果酱 + 冻顶乌龙 + 鲜牛乳', temps: ['冰', '少冰'], tag: '限定', milk: true },
  { id: 'k3', cat: 'new', name: '芝士奶盖拿铁', en: 'Cheese Foam Latte', img: IMG + 'cheese.png', price: 23, origin: 33, sales: '月售 6500', desc: '咸甜芝士奶盖，口口绵密', temps: ['冰', '少冰'], tag: '', milk: true },
  { id: 'k4', cat: 'latte', name: '生椰拿铁', en: 'Coconut Latte', img: IMG + 'coconut.png', price: 18, origin: 29, sales: '月售 5.3 万', desc: '厚椰乳 + 双份浓缩，招牌 No.1', temps: ['冰', '少冰', '去冰', '热'], tag: '招牌', milk: true },
  { id: 'k5', cat: 'latte', name: '丝绒拿铁', en: 'Velvet Latte', img: IMG + 'latte.png', price: 17, origin: 27, sales: '月售 3.2 万', desc: '鲜牛乳与浓缩的经典比例', temps: ['冰', '少冰', '热'], tag: '', milk: true },
  { id: 'k6', cat: 'latte', name: '焦糖玛奇朵', en: 'Caramel Macchiato', img: IMG + 'caramel.png', price: 21, origin: 31, sales: '月售 1.4 万', desc: '香草糖浆打底，焦糖淋面', temps: ['冰', '热'], tag: '', milk: true },
  { id: 'k7', cat: 'latte', name: '冰吸拿铁', en: 'Iced Latte', img: IMG + 'iced_latte.png', price: 17, origin: 27, sales: '月售 2.6 万', desc: '冰块慢慢化开，越喝越顺', temps: ['冰', '少冰'], tag: '', milk: true },
  { id: 'k8', cat: 'americano', name: '冰美式', en: 'Iced Americano', img: IMG + 'iced_americano.png', price: 13, origin: 22, sales: '月售 6.8 万', desc: '拾光拼配豆，坚果与可可尾韵', temps: ['冰', '少冰', '去冰'], tag: '提神', milk: false },
  { id: 'k9', cat: 'americano', name: '热美式', en: 'Americano', img: IMG + 'americano.png', price: 13, origin: 22, sales: '月售 2.2 万', desc: '一杯下去，早八不慌', temps: ['热'], tag: '', milk: false },
  { id: 'k10', cat: 'fruit', name: '抹茶轻乳', en: 'Matcha Milk', img: IMG + 'matcha.png', price: 19, origin: 28, sales: '月售 8700', desc: '宇治抹茶 + 鲜牛乳，不含咖啡', temps: ['冰', '少冰', '热'], tag: '无咖', milk: true },
  { id: 'k11', cat: 'classic', name: '澳白', en: 'Flat White', img: IMG + 'flatwhite.png', price: 20, origin: 30, sales: '月售 1.1 万', desc: '双份 ristretto，奶泡更薄', temps: ['热', '冰'], tag: '', milk: true },
  { id: 'k12', cat: 'classic', name: '摩卡', en: 'Mocha', img: IMG + 'mocha.png', price: 21, origin: 31, sales: '月售 7600', desc: '比利时黑巧 + 浓缩 + 奶油顶', temps: ['热', '冰'], tag: '', milk: true }
];
const sizes = [
  { name: '中杯', ml: '360ml', add: 0 },
  { name: '大杯', ml: '480ml', add: 3 },
  { name: '超大杯', ml: '600ml', add: 5 }
];
const sugars = ['标准糖', '少糖', '半糖', '不另外加糖'];
// 加料：可多选；milkOnly 的只给含奶饮品
const addons = [
  { id: 'shot', name: '浓缩 +1 份', add: 3 },
  { id: 'oat', name: '换燕麦奶', add: 4, milkOnly: true },
  { id: 'vanilla', name: '香草糖浆', add: 2 }
];
const stores = [
  { id: 'st1', name: '拾光咖啡（望京SOHO店）', addr: '望京SOHO T1 首层 L1-08', distance: '320m', hours: '07:30-21:00', phone: '010-5718 6620', lat: 39.9962, lng: 116.4807 },
  { id: 'st2', name: '拾光咖啡（方恒购物中心店）', addr: '阜通东大街 6 号方恒购物中心 B1', distance: '860m', hours: '08:00-22:00', phone: '010-5718 6621', lat: 39.9893, lng: 116.4826 },
  { id: 'st3', name: '拾光咖啡（花家地店）', addr: '花家地北里 2 号楼底商', distance: '1.4km', hours: '07:00-20:00', phone: '010-5718 6622', lat: 39.9808, lng: 116.4712 }
];
const coupons = [
  { id: 'q1', amount: 5, unit: '元', title: '新品立减券', cond: '满 20 元可用', min: 20, expire: '2024-03-31', status: 'valid' },
  { id: 'q2', amount: 9.9, unit: '元', title: '任意饮品 9.9 元购', cond: '限中杯，不含新品', min: 0, expire: '2024-03-20', status: 'valid', fixed: true },
  { id: 'q3', amount: 3, unit: '元', title: '无门槛券', cond: '全场通用', min: 0, expire: '2024-04-15', status: 'valid' },
  { id: 'q4', amount: 8, unit: '折', title: '外送 8 折券', cond: '外送订单满 30 元', min: 30, expire: '2024-04-01', status: 'valid', rate: 0.8 },
  { id: 'q5', amount: 5, unit: '元', title: '早餐咖啡券', cond: '09:30 前下单可用', min: 15, expire: '2024-03-10', status: 'used' },
  { id: 'q6', amount: 10, unit: '元', title: '会员生日券', cond: '满 30 元可用', min: 30, expire: '2024-02-28', status: 'expired' },
  { id: 'q7', amount: 6, unit: '元', title: '春日拿铁券', cond: '拿铁家族可用', min: 18, expire: '2024-03-01', status: 'expired' }
];
// 咖啡钱包充值档位
const recharges = [
  { pay: 100, bonus: 20 },
  { pay: 200, bonus: 50 },
  { pay: 500, bonus: 150 }
];
// 积分商城：coupon 兑换后进「我的优惠券」，goods 兑换后凭兑换码到店领取
const mall = [
  { id: 'm1', type: 'coupon', name: '3 元无门槛券', points: 300, glyph: '3', c1: '#e7a15b', c2: '#b5562a', coupon: { amount: 3, unit: '元', title: '积分兑换 · 3 元无门槛券', cond: '全场通用', min: 0 } },
  { id: 'm2', type: 'coupon', name: '满 25 减 8 券', points: 600, glyph: '8', c1: '#d98a6a', c2: '#a2462c', coupon: { amount: 8, unit: '元', title: '积分兑换 · 满 25 减 8', cond: '满 25 元可用', min: 25 } },
  { id: 'm3', type: 'coupon', name: '任意饮品 9.9 元券', points: 900, glyph: '9.9', c1: '#b98a5e', c2: '#6b4226', coupon: { amount: 9.9, unit: '元', title: '积分兑换 · 9.9 元购', cond: '限中杯，不含新品', min: 0, fixed: true } },
  { id: 'm4', type: 'goods', name: '挂耳咖啡 10 片装', points: 1500, glyph: '挂', c1: '#a07b5c', c2: '#5b3a29', stock: 36 },
  { id: 'm5', type: 'goods', name: '帆布托特包', points: 2600, glyph: '包', c1: '#8fae7a', c2: '#4f7a4a', stock: 12 },
  { id: 'm6', type: 'goods', name: '拾光随行杯', points: 3800, glyph: '杯', c1: '#6b8fb5', c2: '#3f5f86', stock: 5 }
];
const rateTags = ['咖啡香浓', '出餐快', '包装用心', '甜度刚好', '店员热情', '性价比高'];
const faqs = [
  { q: '下单后多久可以取餐？', a: '立即制作的订单一般 3~5 分钟完成，高峰期会稍长；预约订单按你选的时间开始制作。' },
  { q: '订单还能取消吗？', a: '咖啡师开始制作前（下单后 1 分钟内）可以在订单详情里取消，款项与优惠券原路退回。' },
  { q: '积分怎么获得、怎么用？', a: '每消费 1 元得 1 积分，每日签到 +10。100 积分可抵 1 元，单笔最多抵 5 元，也能在积分商城兑换券和周边。' },
  { q: '咖啡钱包的钱能退吗？', a: '充值金额可在 App 内付款时直接使用，赠送金额不可提现；如需退款请联系客服。' }
];
function findDrink(id) {
  return drinks.find(d => d.id === id) || drinks[0];
}
function findStore(id) {
  return stores.find(s => s.id === id) || stores[0];
}
module.exports = { categories, drinks, sizes, sugars, addons, stores, coupons, recharges, mall, rateTags, faqs, findDrink, findStore };
