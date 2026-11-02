// 燃动健身：离线演示数据（门店、教练、团课、训练计划、训练类型）
const stores = [
  { id: 'kjy', name: '燃动 · 科技园店', short: '科技园店', dist: '0.8km', addr: '南山区科苑南路 3099 号中国储能大厦 2F', phone: '0755-8666 2031', hours: '06:30-23:00', lat: 22.5309, lng: 113.9531 },
  { id: 'hac', name: '燃动 · 海岸城店', short: '海岸城店', dist: '2.3km', addr: '南山区文心五路 33 号海岸城购物中心 5F', phone: '0755-8666 2032', hours: '07:00-22:30', lat: 22.5172, lng: 113.9361 },
  { id: 'hhh', name: '燃动 · 后海汇店', short: '后海汇店', dist: '3.6km', addr: '南山区中心路 3333 号后海汇 B1', phone: '0755-8666 2033', hours: '06:00-23:30', lat: 22.5139, lng: 113.9446 }
];
const coaches = {
  lin: { name: 'Lina 林娜', tag: '国家一级健身指导员', color: '#ff7a3d' },
  zhou: { name: 'Kevin 周凯', tag: 'ACE 认证私教', color: '#3d7bff' },
  su: { name: 'Mia 苏蔓', tag: 'RYT-500 瑜伽导师', color: '#2fb57a' },
  he: { name: 'Leo 何磊', tag: 'Les Mills 认证', color: '#8a5cff' }
};
const courses = [
  { id: 'c1', name: '燃脂搏击 BodyCombat', type: '燃脂', coach: 'he', time: '07:30', dur: 45, room: '1 号操房', cap: 25, booked: 19, kcal: 520, level: 2, bg: 'linear-gradient(135deg, #ff7a3d, #ff3d6e)', intro: '融合空手道、拳击、泰拳动作的高强度有氧课，跟着节拍出拳踢腿，45 分钟高效燃脂。' },
  // booked 比 cap 还多：晨间瑜伽最抢手，近两天都约满（演示候补）
  { id: 'c2', name: '晨间流瑜伽', type: '瑜伽', coach: 'su', time: '09:00', dur: 60, room: '瑜伽室', cap: 18, booked: 21, kcal: 220, level: 1, bg: 'linear-gradient(135deg, #34d399, #0ea5a4)', intro: '以拜日式串联体式，配合呼吸唤醒身体，改善久坐带来的肩颈紧张，零基础友好。' },
  { id: 'c3', name: '动感单车 RPM', type: '单车', coach: 'zhou', time: '12:15', dur: 45, room: '单车房', cap: 30, booked: 22, kcal: 600, level: 3, bg: 'linear-gradient(135deg, #6a8bff, #3d5afe)', intro: '模拟爬坡、冲刺与平路骑行，间歇强度带来持续燃脂效果，适合有一定基础的会员。' },
  { id: 'c4', name: '核心塑形 Pilates', type: '普拉提', coach: 'lin', time: '18:30', dur: 50, room: '2 号操房', cap: 16, booked: 9, kcal: 280, level: 1, bg: 'linear-gradient(135deg, #f472b6, #a855f7)', intro: '强化深层核心与骨盆稳定，改善体态、缓解腰背不适，器械与垫上动作结合。' },
  { id: 'c5', name: '杠铃力量 BodyPump', type: '力量', coach: 'he', time: '19:30', dur: 55, room: '1 号操房', cap: 25, booked: 24, kcal: 450, level: 2, bg: 'linear-gradient(135deg, #fbbf24, #f97316)', intro: '轻重量多次数的杠铃训练，覆盖全身主要肌群，提升肌耐力与基础代谢。' },
  { id: 'c6', name: '夜间舒缓拉伸', type: '瑜伽', coach: 'su', time: '20:45', dur: 40, room: '瑜伽室', cap: 18, booked: 6, kcal: 120, level: 1, bg: 'linear-gradient(135deg, #60a5fa, #818cf8)', intro: '针对训练后紧张的肌群做深度拉伸与放松，帮助恢复、改善睡眠质量。' }
];
courses.forEach(c => { c.coachInfo = coaches[c.coach]; c.levelText = ['', '入门', '进阶', '挑战'][c.level]; });
const WEEK = ['日', '一', '二', '三', '四', '五', '六'];
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function nextDays(n) {
  const out = [];
  const b = new Date();
  for (let i = 0; i < n; i++) {
    const d = new Date(b.getFullYear(), b.getMonth(), b.getDate() + i);
    out.push({ idx: i, day: d.getDate(), week: i === 0 ? '今天' : '周' + WEEK[d.getDay()], key: (d.getMonth() + 1) + '月' + d.getDate() + '日', date: d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) });
  }
  return out;
}
// 每天的课表：同一批课按日期轮换，余位按日期变化（离线可复现）。别人约的 booked 不含自己
function scheduleFor(dayIdx) {
  const day = nextDays(dayIdx + 1)[dayIdx];
  return courses.map((c, i) => {
    const booked = Math.min(c.cap, Math.max(2, c.booked - dayIdx * 3 + ((i * 5 + dayIdx) % 4)));
    return Object.assign({}, c, { booked, left: c.cap - booked, pct: Math.round(booked / c.cap * 100), dayIdx, date: day.date, dateText: day.key });
  });
}
function findCourse(id) { return courses.find(c => c.id === id); }
// 训练计划：按 7 天一轮排课，rest 为休息日
const plans = [
  { id: 'fat21', name: '21 天燃脂计划', level: '入门', days: 21, goal: '减脂', desc: '每周练 5 天：有氧 + 核心，每次 30~40 分钟', color: 'linear-gradient(135deg, #ff7a3d, #ff3d6e)',
    week: [{ name: '快走 + 核心激活', min: 30, kcal: 220 }, { name: 'HIIT 间歇训练', min: 25, kcal: 300 }, { rest: true }, { name: '慢跑 3 公里', min: 30, kcal: 280 },
      { name: '全身循环训练', min: 35, kcal: 320 }, { name: '单车 + 拉伸', min: 40, kcal: 330 }, { rest: true }] },
  { id: 'muscle28', name: '增肌塑形 4 周', level: '进阶', days: 28, goal: '增肌', desc: '上下肢分化，每周 4 练，大重量复合动作', color: 'linear-gradient(135deg, #6a8bff, #3d5afe)',
    week: [{ name: '胸 + 三头', min: 55, kcal: 360 }, { name: '背 + 二头', min: 55, kcal: 360 }, { rest: true }, { name: '腿 + 臀', min: 60, kcal: 420 },
      { name: '肩 + 核心', min: 50, kcal: 320 }, { rest: true }, { rest: true }] },
  { id: 'begin14', name: '新手 14 天入门', level: '入门', days: 14, goal: '保持', desc: '从动作模式学起，每次 20 分钟，零基础友好', color: 'linear-gradient(135deg, #34d399, #0ea5a4)',
    week: [{ name: '自重深蹲 + 平板支撑', min: 20, kcal: 120 }, { name: '瑜伽拉伸', min: 20, kcal: 80 }, { name: '椭圆机有氧', min: 20, kcal: 160 }, { rest: true },
      { name: '弹力带上肢', min: 20, kcal: 110 }, { name: '快走 30 分钟', min: 30, kcal: 150 }, { rest: true }] }
];
function findPlan(id) { return plans.find(p => p.id === id) || null; }
// 记一笔训练：类型 → 每分钟大约消耗多少千卡（填时长自动估算，可改）
const workoutTypes = [
  { key: 'run', name: '跑步', perMin: 10 }, { key: 'strength', name: '力量', perMin: 7 }, { key: 'cycle', name: '单车', perMin: 9 },
  { key: 'swim', name: '游泳', perMin: 9 }, { key: 'yoga', name: '瑜伽', perMin: 4 }, { key: 'other', name: '其他', perMin: 6 }
];
// 会员卡套餐（days：顺延天数）；单次课价格给没有有效会员卡的人
const cardPlans = [
  { name: '月卡', price: 299, days: 30, per: '¥9.9/天', orig: 399, badge: '' },
  { name: '季卡', price: 799, days: 90, per: '¥8.8/天', orig: 1197, badge: '热销' },
  { name: '黑金年卡', price: 2580, days: 365, per: '¥7.0/天', orig: 3999, badge: '最划算' }
];
const SINGLE_PRICE = 79;
module.exports = { stores, coaches, courses, nextDays, scheduleFor, findCourse, plans, findPlan, workoutTypes, cardPlans, SINGLE_PRICE };
