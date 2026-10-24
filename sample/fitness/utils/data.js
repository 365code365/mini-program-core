// 燃动健身：离线演示数据
const coaches = {
  lin: { name: 'Lina 林娜', tag: '国家一级健身指导员', color: '#ff7a3d' },
  zhou: { name: 'Kevin 周凯', tag: 'ACE 认证私教', color: '#3d7bff' },
  su: { name: 'Mia 苏蔓', tag: 'RYT-500 瑜伽导师', color: '#2fb57a' },
  he: { name: 'Leo 何磊', tag: 'Les Mills 认证', color: '#8a5cff' }
};

const courses = [
  { id: 'c1', name: '燃脂搏击 BodyCombat', type: '燃脂', coach: 'he', time: '07:30', dur: 45, room: '1 号操房', cap: 25, booked: 19, kcal: 520, level: 2, bg: 'linear-gradient(135deg, #ff7a3d, #ff3d6e)', intro: '融合空手道、拳击、泰拳动作的高强度有氧课，跟着节拍出拳踢腿，45 分钟高效燃脂。' },
  { id: 'c2', name: '晨间流瑜伽', type: '瑜伽', coach: 'su', time: '09:00', dur: 60, room: '瑜伽室', cap: 18, booked: 18, kcal: 220, level: 1, bg: 'linear-gradient(135deg, #34d399, #0ea5a4)', intro: '以拜日式串联体式，配合呼吸唤醒身体，改善久坐带来的肩颈紧张，零基础友好。' },
  { id: 'c3', name: '动感单车 RPM', type: '单车', coach: 'zhou', time: '12:15', dur: 45, room: '单车房', cap: 30, booked: 22, kcal: 600, level: 3, bg: 'linear-gradient(135deg, #6a8bff, #3d5afe)', intro: '模拟爬坡、冲刺与平路骑行，间歇强度带来持续燃脂效果，适合有一定基础的会员。' },
  { id: 'c4', name: '核心塑形 Pilates', type: '普拉提', coach: 'lin', time: '18:30', dur: 50, room: '2 号操房', cap: 16, booked: 9, kcal: 280, level: 1, bg: 'linear-gradient(135deg, #f472b6, #a855f7)', intro: '强化深层核心与骨盆稳定，改善体态、缓解腰背不适，器械与垫上动作结合。' },
  { id: 'c5', name: '杠铃力量 BodyPump', type: '力量', coach: 'he', time: '19:30', dur: 55, room: '1 号操房', cap: 25, booked: 24, kcal: 450, level: 2, bg: 'linear-gradient(135deg, #fbbf24, #f97316)', intro: '轻重量多次数的杠铃训练，覆盖全身主要肌群，提升肌耐力与基础代谢。' },
  { id: 'c6', name: '夜间舒缓拉伸', type: '瑜伽', coach: 'su', time: '20:45', dur: 40, room: '瑜伽室', cap: 18, booked: 6, kcal: 120, level: 1, bg: 'linear-gradient(135deg, #60a5fa, #818cf8)', intro: '针对训练后紧张的肌群做深度拉伸与放松，帮助恢复、改善睡眠质量。' }
];
courses.forEach(c => { c.coachInfo = coaches[c.coach]; c.levelText = ['', '入门', '进阶', '挑战'][c.level]; });

const WEEK = ['日', '一', '二', '三', '四', '五', '六'];
function nextDays(n) {
  const out = [];
  const b = new Date();
  for (let i = 0; i < n; i++) {
    const d = new Date(b.getFullYear(), b.getMonth(), b.getDate() + i);
    out.push({ idx: i, day: d.getDate(), week: i === 0 ? '今天' : '周' + WEEK[d.getDay()], key: (d.getMonth() + 1) + '月' + d.getDate() + '日' });
  }
  return out;
}

// 每天的课表：同一批课按日期轮换，余位按日期变化（离线可复现）
function scheduleFor(dayIdx) {
  return courses.map((c, i) => {
    const booked = Math.min(c.cap, Math.max(2, c.booked - dayIdx * 3 + ((i * 5 + dayIdx) % 4)));
    return Object.assign({}, c, { booked, left: c.cap - booked, pct: Math.round(booked / c.cap * 100), dayIdx });
  });
}

function findCourse(id) { return courses.find(c => c.id === id); }

const weekRecords = [
  { d: '一', min: 45, kcal: 380 }, { d: '二', min: 0, kcal: 0 }, { d: '三', min: 62, kcal: 540 },
  { d: '四', min: 30, kcal: 210 }, { d: '五', min: 55, kcal: 460 }, { d: '六', min: 80, kcal: 690 }, { d: '日', min: 38, kcal: 300 }
];

const history = [
  { id: 'h1', name: '燃脂搏击 BodyCombat', date: '周六 07:30', min: 45, kcal: 512, hr: 152, type: '团课' },
  { id: 'h2', name: '跑步机 · 间歇跑', date: '周六 19:10', min: 35, kcal: 178, hr: 141, type: '自主' },
  { id: 'h3', name: '杠铃力量 BodyPump', date: '周五 19:30', min: 55, kcal: 460, hr: 138, type: '团课' },
  { id: 'h4', name: '核心塑形 Pilates', date: '周四 18:30', min: 30, kcal: 210, hr: 118, type: '团课' },
  { id: 'h5', name: '器械 · 胸背训练', date: '周三 20:00', min: 62, kcal: 540, hr: 132, type: '自主' }
];

module.exports = { coaches, courses, nextDays, scheduleFor, findCourse, weekRecords, history };
