// 滴达出行：离线演示数据（车型计价、地点、司机、优惠券、常见问题）
// 地点坐标 x / y 是以望京SOHO 为原点的公里数（东、北为正），两点距离 = 直线 × 1.25（路网绕行系数）
const CAR_TYPES = [
  { key: 'express', name: '快车', desc: '经济实惠', base: 10, perKm: 1.8, perMin: 0.45, seats: 4 },
  { key: 'premier', name: '专车', desc: '舒适商务', base: 16, perKm: 3.2, perMin: 0.8, seats: 4 },
  { key: 'pool', name: '拼车', desc: '一口价 省钱', base: 6, perKm: 1.2, perMin: 0.3, seats: 2, rate: 0.82 }
];
const FAR_KM = 15;
const FAR_PER_KM = 0.8;
const CANCEL_FEE = 5;
const SERVICE_PHONE = '400-0000-1010';

const ORIGIN = { name: '望京SOHO T1-东门', addr: '北京市朝阳区阜通东大街6号', x: 0, y: 0 };
const POIS = [
  { name: '北京首都国际机场T3航站楼', addr: '顺义区首都机场路', x: 14, y: 9, airport: true },
  { name: '北京大兴国际机场', addr: '大兴区大兴机场高速', x: -8, y: -45, airport: true },
  { name: '北京南站', addr: '丰台区永外大街车站路12号', x: -8, y: -15, station: true },
  { name: '北京西站', addr: '丰台区莲花池东路118号', x: -15, y: -10, station: true },
  { name: '三里屯太古里', addr: '朝阳区三里屯路19号', x: -1.5, y: -6.5 },
  { name: '国贸商城', addr: '朝阳区建国门外大街1号', x: -0.5, y: -9.5 },
  { name: '颐和园', addr: '海淀区新建宫门路19号', x: -17, y: -1 },
  { name: '五道口购物中心', addr: '海淀区成府路28号', x: -11, y: -1 },
  { name: '望京凯德MALL', addr: '朝阳区广顺北大街33号', x: -0.6, y: 1.1 },
  { name: '798艺术区', addr: '朝阳区酒仙桥路4号', x: 1.5, y: -2.3 },
  { name: '中关村软件园二期', addr: '海淀区西北旺东路10号', x: -15, y: 6 },
  { name: '朝阳大悦城', addr: '朝阳区朝阳北路101号', x: 2, y: -7.5 },
  { name: '北京协和医院（东单院区）', addr: '东城区帅府园1号', x: -5.5, y: -11 },
  { name: '望京西园三区', addr: '朝阳区望京西园三区312号楼', x: -1.2, y: 1 },
  { name: '国贸三期', addr: '朝阳区建国门外大街1号', x: -0.3, y: -9.6 },
  { name: '西二旗地铁站', addr: '海淀区西二旗大街', x: -13, y: 5 }
];
// 接送机 / 站：首页「接送机」一键填终点
const HUBS = POIS.filter(p => p.airport || p.station);

const DRIVERS = [
  { name: '王师傅', surname: '王', plate: '京A·D7K29', model: '比亚迪 秦PLUS DM-i', color: '白色', rating: '4.96', orders: 2316, phone: '171 8056 2391', dist: 1.2, eta: 3 },
  { name: '刘师傅', surname: '刘', plate: '京N·5T8E2', model: '丰田 凯美瑞', color: '黑色', rating: '4.92', orders: 4108, phone: '171 8056 4472', dist: 0.8, eta: 2 },
  { name: '张师傅', surname: '张', plate: '京Q·M2R67', model: '大众 帕萨特', color: '灰色', rating: '4.98', orders: 1587, phone: '171 8056 7713', dist: 1.6, eta: 4 }
];

// 优惠券：types 限车型；airport 限起终点含机场；night 限 22:00-05:00 下单；min 满多少可用
const COUPONS = [
  { id: 'c1', amount: 15, min: 40, cond: '满 40 可用', name: '快车立减券', date: '2026.10.24 - 2026.11.07', scope: '快车', types: ['express'] },
  { id: 'c2', amount: 8, min: 0, cond: '无门槛', name: '新客专享券', date: '2026.10.20 - 2026.11.20', scope: '全部车型' },
  { id: 'c3', amount: 20, min: 80, cond: '满 80 可用', name: '专车商务券', date: '2026.10.01 - 2026.12.31', scope: '专车', types: ['premier'] },
  { id: 'c4', amount: 5, min: 0, cond: '无门槛', name: '拼车出行券', date: '2026.10.24 - 2026.11.30', scope: '拼车', types: ['pool'] },
  { id: 'c5', amount: 10, min: 30, cond: '满 30 可用', name: '机场接送券', date: '2026.10.15 - 2026.11.15', scope: '机场订单', airport: true },
  { id: 'c6', amount: 6, min: 20, cond: '满 20 可用', name: '夜间出行券', date: '2026.10.24 - 2026.11.24', scope: '22:00 后', night: true }
];
const EXPIRED = [
  { id: 'e1', amount: 12, min: 35, cond: '满 35 可用', name: '国庆出行券', date: '2026.10.01 - 2026.10.07', scope: '快车' },
  { id: 'e2', amount: 3, min: 0, cond: '无门槛', name: '早高峰券', date: '2026.09.01 - 2026.09.30', scope: '全部车型' }
];
// 首页 banner 一次领 3 张
const WEEKEND_PACK = [
  { amount: 5, min: 0, cond: '无门槛', name: '周末快车券', scope: '快车', types: ['express'] },
  { amount: 5, min: 15, cond: '满 15 可用', name: '周末快车券', scope: '快车', types: ['express'] },
  { amount: 5, min: 25, cond: '满 25 可用', name: '周末快车券', scope: '快车', types: ['express'] }
];
// 兑换码（每个码只能兑一次）
const CODES = {
  DIDA2026: { amount: 10, min: 0, cond: '无门槛', name: '兑换码专享券', scope: '全部车型' },
  AIRPORT: { amount: 20, min: 60, cond: '满 60 可用', name: '接送机兑换券', scope: '机场订单', airport: true }
};
const RECHARGES = [{ pay: 50, bonus: 0 }, { pay: 100, bonus: 5 }, { pay: 200, bonus: 15 }];

const QUICK_MSGS = ['我在路边等你', '请稍等，马上到', '我在上车点了', '麻烦开到小区门口'];
const DRIVER_REPLY = ['好的，马上到', '好的，您稍等', '收到，我到了给您打电话', '好的，我开到门口'];
const CANCEL_REASONS = ['行程有变，暂不需要用车', '司机太远 / 迟迟不来', '司机要求我取消', '联系不上司机', '其他原因'];
// 司机原因（要求取消、联系不上）不收取消费
const DRIVER_FAULT = [2, 3];
const RATE_TAGS = ['车内整洁', '驾驶平稳', '准时到达', '路线熟悉', '服务热情', '没有异味'];
// 三星及以下换成「哪里不满意」
const RATE_BAD = ['车内有异味', '绕路', '驾驶不平稳', '迟到', '态度不好', '开车打电话'];
const COMPLAIN_REASONS = ['司机绕路', '多收费用', '车内有异味', '危险驾驶', '服务态度差', '其他问题'];
const FAQS = [
  { q: '取消订单会扣费吗？', a: '司机接单前取消免费；接单 2 分钟内取消免费；接单超过 2 分钟或司机已到达后取消，收取 5 元取消费。司机要求取消、联系不上司机不收费。' },
  { q: '对行程费用有疑问怎么办？', a: '在「行程」里打开那一单，点「投诉与反馈」选「多收费用」，客服核实后差额原路退回。' },
  { q: '物品遗落在车上怎么办？', a: '行程结束 24 小时内可以在行程详情里通过隐私号联系司机。' },
  { q: '怎么开电子发票？', a: '「我的 → 电子发票」勾选已支付的行程，填写抬头与邮箱即可，多个行程可以合开一张。' }
];

function carType(key) {
  return CAR_TYPES.find(c => c.key === key) || CAR_TYPES[0];
}
function round1(n) {
  return Math.round(n * 10) / 10;
}
function km(a, b) {
  const dx = (a.x || 0) - (b.x || 0);
  const dy = (a.y || 0) - (b.y || 0);
  return Math.max(1.5, round1(Math.sqrt(dx * dx + dy * dy) * 1.25));
}
function minutes(k) {
  return Math.round(k * 2.2 + 6);
}
// 车费：起步价 + 里程费 + 时长费 + 远途费（超 15 公里部分 0.8 元/公里）；拼车拼成再打 82 折
function fare(key, k, min) {
  const t = carType(key);
  const mileage = round1(t.perKm * k);
  const time = round1(t.perMin * min);
  const far = k > FAR_KM ? round1((k - FAR_KM) * FAR_PER_KM) : 0;
  const sub = round1(t.base + mileage + time + far);
  const discount = t.rate ? round1(sub * (1 - t.rate)) : 0;
  return { base: t.base, mileage, time, far, discount, total: round1(sub - discount) };
}
function ruleText(key) {
  const t = carType(key);
  return `起步价 ${t.base} 元（含 0 公里）；里程费 ${t.perKm} 元/公里；时长费 ${t.perMin} 元/分钟；` +
    `单程超过 ${FAR_KM} 公里，超出部分加收远途费 ${FAR_PER_KM} 元/公里` + (t.rate ? '；拼成后整单 82 折' : '') + '。';
}
function findPoi(name) {
  return POIS.find(p => p.name === name) || null;
}

module.exports = {
  CAR_TYPES, FAR_KM, CANCEL_FEE, SERVICE_PHONE, ORIGIN, POIS, HUBS, DRIVERS,
  COUPONS, EXPIRED, WEEKEND_PACK, CODES, RECHARGES,
  QUICK_MSGS, DRIVER_REPLY, CANCEL_REASONS, DRIVER_FAULT, RATE_TAGS, RATE_BAD, COMPLAIN_REASONS, FAQS,
  carType, round1, km, minutes, fare, ruleText, findPoi
};
