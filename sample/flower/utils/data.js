// 花束商品：纯离线数据。price 是 11 枝 + 基础包装的价格，其余规格按差价加
const uses = [
  { key: 'love', name: '爱情', sub: '表白纪念', bg: 'linear-gradient(135deg, #ff9fb4, #ef4f76)', color: '#ef4f76' },
  { key: 'birthday', name: '生日', sub: '祝福惊喜', bg: 'linear-gradient(135deg, #ffd27a, #ff9a3d)', color: '#ff9a3d' },
  { key: 'business', name: '商务', sub: '开业庆典', bg: 'linear-gradient(135deg, #8fb8ff, #4f7ff0)', color: '#4f7ff0' },
  { key: 'visit', name: '探望', sub: '慰问长辈', bg: 'linear-gradient(135deg, #8de0bf, #2fb487)', color: '#2fb487' }
];
const stems = [
  { k: 11, name: '11 枝', sub: '一心一意', add: 0 },
  { k: 19, name: '19 枝', sub: '长长久久', add: 60 },
  { k: 33, name: '33 枝', sub: '三生三世', add: 150 },
  { k: 52, name: '52 枝', sub: '我只爱你', add: 280 }
];
const packs = [
  { k: 'mist', name: '韩式雾面纸', add: 0 },
  { k: 'kraft', name: '复古牛皮纸', add: 10 },
  { k: 'box', name: '高定礼盒', add: 68 }
];
const slots = ['09:00-12:00', '12:00-15:00', '15:00-18:00', '18:00-21:00'];
const products = [
  { id: 'f1', name: '热恋 · 卡罗拉红玫瑰', img: '/images/b1.png', price: 199, origin: 259, sold: 8623, use: 'love', tag: '店长推荐',
    flowers: '卡罗拉红玫瑰 11 枝、满天星点缀', words: '你是我平淡生活里的浪漫。' },
  { id: 'f2', name: '初见 · 香槟玫瑰', img: '/images/b2.png', price: 229, origin: 289, sold: 4210, use: 'love', tag: '',
    flowers: '香槟玫瑰 11 枝、尤加利叶', words: '与你初见，便是心动的开始。' },
  { id: 'f3', name: '紫色梦境 · 桔梗花束', img: '/images/b3.png', price: 169, origin: 219, sold: 3108, use: 'birthday', tag: '新品',
    flowers: '紫桔梗 11 枝、白色满天星', words: '愿你的每一个梦都温柔明亮。' },
  { id: 'f4', name: '向阳而生 · 向日葵', img: '/images/b4.png', price: 159, origin: 199, sold: 6741, use: 'birthday', tag: '第二束半价',
    flowers: '向日葵 11 枝、黄莺草', words: '生日快乐，愿你永远热烈灿烂。' },
  { id: 'f5', name: '甜心 · 粉佳人玫瑰', img: '/images/b5.png', price: 189, origin: 239, sold: 5326, use: 'love', tag: '',
    flowers: '粉佳人玫瑰 11 枝、粉色石竹', words: '把所有的甜都送给你。' },
  { id: 'f6', name: '安康 · 白百合康乃馨', img: '/images/b6.png', price: 209, origin: 269, sold: 2894, use: 'visit', tag: '',
    flowers: '白百合 3 枝、康乃馨 8 枝', words: '祝您早日康复，平安顺遂。' },
  { id: 'f7', name: '鸿运 · 珊瑚红开业花束', img: '/images/b7.png', price: 299, origin: 368, sold: 1527, use: 'business', tag: '商务',
    flowers: '珊瑚红玫瑰 11 枝、金色叶材', words: '开业大吉，生意兴隆。' },
  { id: 'f8', name: '蓝色海洋 · 绣球花束', img: '/images/b8.png', price: 239, origin: 299, sold: 1983, use: 'business', tag: '',
    flowers: '蓝绣球 2 头、蓝色满天星', words: '祝合作顺利，前程似海。' }
];
// 活动：向日葵（首页第二张 banner）同一单第 2、4、6… 束半价
const HALF_ID = 'f4';
// 优惠券模板：days 是从今天算起的有效天数（负数 = 已过期）；use 有值时只算该用途的花束金额
const coupons = [
  { id: 'c1', amount: 20, min: 199, title: '满 199 减 20', cond: '全场花束通用', days: 29 },
  { id: 'c2', amount: 40, min: 299, title: '满 299 减 40', cond: '全场花束通用', days: 58 },
  { id: 'c3', amount: 30, min: 199, use: 'visit', title: '探望花礼满 199 减 30', cond: '限探望鲜花', days: 20 },
  { id: 'c0', amount: 52, min: 299, use: 'love', title: '七夕告白券', cond: '限爱情鲜花', days: -3 }
];
const rateTags = ['花材新鲜', '包装精美', '准时送达', '和图片一致', '贺卡用心', '配送员热情'];
const SERVICE_PHONE = '400-660-5520';
const COURIER = { name: '李师傅', phone: '170 9998 6617' };
const faqs = [
  { q: '最快多久能送到？', a: '同城下单后最快 3 小时送达；指定时段的订单在所选时段内送到，节日高峰建议提前一天预订。' },
  { q: '订单还能取消吗？', a: '花艺师在送达时段开始前 2 小时开始制作，制作前（或支付后 10 分钟内）可在订单详情里取消，款项与优惠券原路退回。' },
  { q: '收到的花不满意怎么办？', a: '签收后 24 小时内拍照联系客服，花材问题免费重做或全额退款。' },
  { q: '可以匿名送花吗？', a: '下单时打开「匿名送花」，贺卡不署名，配送员也不会透露订花人信息。' }
];
// 买家评价（离线示例），按用途分组；自己评价过的订单会排在前面
const reviewPool = {
  love: [{ user: '陈**', stars: 5, text: '花很新鲜，女朋友收到超级开心，包装也很精致' }, { user: '周**', stars: 5, text: '比图片还好看，配送员很准时，还帮忙保密了' }],
  birthday: [{ user: '林**', stars: 5, text: '妈妈生日送的，花开得很灿烂，贺卡是手写的，很用心' }, { user: '许**', stars: 4, text: '准时送到公司，同事都说好看，就是包装纸稍微有点皱' }],
  visit: [{ user: '王**', stars: 5, text: '探望长辈送的，花材很新鲜，香味淡淡的' }, { user: '赵**', stars: 5, text: '送到医院病房很顺利，配送员电话沟通很耐心' }],
  business: [{ user: '孙**', stars: 5, text: '朋友新店开业送的，花束很大气，很有面子' }, { user: '吴**', stars: 4, text: '客户很喜欢，下次还会再来订' }]
};
function productOf(id) { return products.find(p => p.id === id); }
function stemOf(k) { return stems.find(s => s.k === k) || stems[0]; }
function packOf(k) { return packs.find(p => p.k === k) || packs[0]; }
function useOf(k) { return uses.find(u => u.key === k) || uses[0]; }
function unitPrice(p, stemK, packK) { return p.price + stemOf(stemK).add + packOf(packK).add; }
module.exports = {
  uses, stems, packs, slots, products, HALF_ID, coupons, rateTags, SERVICE_PHONE, COURIER, faqs, reviewPool,
  productOf, stemOf, packOf, useOf, unitPrice
};
