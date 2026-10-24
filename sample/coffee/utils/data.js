// 拾光咖啡 · 离线菜单数据（价格单位：元，price 为中杯价）
const IMG = '/images/drinks/';

const categories = [
  { id: 'new', name: '当季新品' },
  { id: 'latte', name: '拿铁家族' },
  { id: 'americano', name: '美式' },
  { id: 'fruit', name: '果咖·轻乳' },
  { id: 'classic', name: '经典意式' }
];

const drinks = [
  { id: 'k1', cat: 'new', name: '橙C美式', en: 'Orange C Americano', img: IMG + 'orange.png', price: 19, origin: 29, sales: '月售 2.1 万', desc: '鲜榨橙汁 + 埃塞俄比亚日晒豆，酸甜清爽', temps: ['冰', '少冰', '去冰'], tag: '新品' },
  { id: 'k2', cat: 'new', name: '桃桃乌龙拿铁', en: 'Peach Oolong Latte', img: IMG + 'peach.png', price: 22, origin: 32, sales: '月售 9800', desc: '白桃果酱 + 冻顶乌龙 + 鲜牛乳', temps: ['冰', '少冰'], tag: '限定' },
  { id: 'k3', cat: 'new', name: '芝士奶盖拿铁', en: 'Cheese Foam Latte', img: IMG + 'cheese.png', price: 23, origin: 33, sales: '月售 6500', desc: '咸甜芝士奶盖，口口绵密', temps: ['冰', '少冰'], tag: '' },
  { id: 'k4', cat: 'latte', name: '生椰拿铁', en: 'Coconut Latte', img: IMG + 'coconut.png', price: 18, origin: 29, sales: '月售 5.3 万', desc: '厚椰乳 + 双份浓缩，招牌 No.1', temps: ['冰', '少冰', '去冰', '热'], tag: '招牌' },
  { id: 'k5', cat: 'latte', name: '丝绒拿铁', en: 'Velvet Latte', img: IMG + 'latte.png', price: 17, origin: 27, sales: '月售 3.2 万', desc: '鲜牛乳与浓缩的经典比例', temps: ['冰', '少冰', '热'], tag: '' },
  { id: 'k6', cat: 'latte', name: '焦糖玛奇朵', en: 'Caramel Macchiato', img: IMG + 'caramel.png', price: 21, origin: 31, sales: '月售 1.4 万', desc: '香草糖浆打底，焦糖淋面', temps: ['冰', '热'], tag: '' },
  { id: 'k7', cat: 'latte', name: '冰吸拿铁', en: 'Iced Latte', img: IMG + 'iced_latte.png', price: 17, origin: 27, sales: '月售 2.6 万', desc: '冰块慢慢化开，越喝越顺', temps: ['冰', '少冰'], tag: '' },
  { id: 'k8', cat: 'americano', name: '冰美式', en: 'Iced Americano', img: IMG + 'iced_americano.png', price: 13, origin: 22, sales: '月售 6.8 万', desc: '拾光拼配豆，坚果与可可尾韵', temps: ['冰', '少冰', '去冰'], tag: '提神' },
  { id: 'k9', cat: 'americano', name: '热美式', en: 'Americano', img: IMG + 'americano.png', price: 13, origin: 22, sales: '月售 2.2 万', desc: '一杯下去，早八不慌', temps: ['热'], tag: '' },
  { id: 'k10', cat: 'fruit', name: '抹茶轻乳', en: 'Matcha Milk', img: IMG + 'matcha.png', price: 19, origin: 28, sales: '月售 8700', desc: '宇治抹茶 + 鲜牛乳，不含咖啡', temps: ['冰', '少冰', '热'], tag: '无咖' },
  { id: 'k11', cat: 'classic', name: '澳白', en: 'Flat White', img: IMG + 'flatwhite.png', price: 20, origin: 30, sales: '月售 1.1 万', desc: '双份 ristretto，奶泡更薄', temps: ['热', '冰'], tag: '' },
  { id: 'k12', cat: 'classic', name: '摩卡', en: 'Mocha', img: IMG + 'mocha.png', price: 21, origin: 31, sales: '月售 7600', desc: '比利时黑巧 + 浓缩 + 奶油顶', temps: ['热', '冰'], tag: '' }
];

const sizes = [
  { name: '中杯', ml: '360ml', add: 0 },
  { name: '大杯', ml: '480ml', add: 3 },
  { name: '超大杯', ml: '600ml', add: 5 }
];

const sugars = ['标准糖', '少糖', '半糖', '不另外加糖'];

const stores = [
  { id: 'st1', name: '拾光咖啡（望京SOHO店）', addr: '望京SOHO T1 首层 L1-08', distance: '320m', hours: '07:30-21:00' },
  { id: 'st2', name: '拾光咖啡（方恒购物中心店）', addr: '阜通东大街 6 号方恒购物中心 B1', distance: '860m', hours: '08:00-22:00' },
  { id: 'st3', name: '拾光咖啡（花家地店）', addr: '花家地北里 2 号楼底商', distance: '1.4km', hours: '07:00-20:00' }
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

function findDrink(id) {
  return drinks.find(d => d.id === id) || drinks[0];
}

module.exports = { categories, drinks, sizes, sugars, stores, coupons, findDrink };
