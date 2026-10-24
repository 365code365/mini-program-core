// 问卷星球：离线演示数据（问卷、投票、基础统计）
// 统计 base：模拟其他人已经提交的分布，用户提交后在此基础上累加

const surveys = [
  {
    id: 'q1', kind: '问卷', title: '2026 年轻人咖啡消费习惯调查', desc: '了解你每天喝咖啡的频率、口味与花费，约 2 分钟', author: '饮品研究所', glyph: '咖',
    bg: 'linear-gradient(135deg, #C79A6B, #8A5A3B)', joined: 12863, deadline: '2026-11-30', hot: true,
    questions: [
      { id: 'a', type: 'single', title: '你平均每周喝几杯咖啡？', required: true, options: ['几乎不喝', '1-3 杯', '4-7 杯', '每天 2 杯以上'], base: [1320, 4630, 4210, 2703] },
      { id: 'b', type: 'multi', title: '你常喝哪些品类？（可多选）', required: true, options: ['美式', '拿铁', '生椰/风味拿铁', '手冲', '冷萃', '速溶/挂耳'], base: [5120, 7240, 6630, 1920, 2410, 3380] },
      { id: 'c', type: 'rate', title: '你对目前常去咖啡店的满意度打几分？', required: true, min: 0, max: 10, base: { sum: 92640, n: 12863 } },
      { id: 'd', type: 'single', title: '单杯咖啡你能接受的价格？', required: false, options: ['10 元以内', '10-20 元', '20-30 元', '30 元以上'], base: [2210, 6420, 3360, 873] },
      { id: 'e', type: 'text', title: '你希望咖啡店增加什么服务？', required: false, placeholder: '例如：自带杯减价、更多座位…', samples: ['工作日早上能更快出杯', '希望有低糖选项', '自带杯减 2 元就会常去'] }
    ]
  },
  {
    id: 'q2', kind: '投票', title: '公司年会去哪儿？最终投票', desc: '行政部发起，投票结果决定 12 月团建目的地', author: '行政部·张敏', glyph: '游',
    bg: 'linear-gradient(135deg, #5DB8FF, #3A7BFF)', joined: 486, deadline: '2026-11-08', hot: true,
    questions: [
      { id: 'a', type: 'single', title: '你最想去的地方', required: true, options: ['千岛湖·湖景度假', '莫干山·民宿两日', '三亚·海边三日', '本地·温泉 + 剧本杀'], base: [132, 118, 171, 65] }
    ]
  },
  {
    id: 'q3', kind: '问卷', title: '居家健身器材使用情况调研', desc: '帮我们做出更好用的家用器材，填写送 20 元券', author: '燃动运动实验室', glyph: '健',
    bg: 'linear-gradient(135deg, #FF9A6B, #FF5E62)', joined: 3420, deadline: '2026-12-15', hot: false,
    questions: [
      { id: 'a', type: 'multi', title: '你家里有哪些健身器材？', required: true, options: ['瑜伽垫', '哑铃/壶铃', '弹力带', '跑步机', '动感单车', '都没有'], base: [2860, 1720, 1530, 640, 410, 280] },
      { id: 'b', type: 'single', title: '每周居家锻炼几次？', required: true, options: ['基本不练', '1-2 次', '3-4 次', '5 次及以上'], base: [520, 1460, 1080, 360] },
      { id: 'c', type: 'rate', title: '向朋友推荐居家锻炼的意愿（0-10）', required: true, min: 0, max: 10, base: { sum: 24600, n: 3420 } }
    ]
  },
  {
    id: 'q4', kind: '投票', title: '小区周末市集，你最期待哪个摊位？', desc: '翠苑一区业委会发起', author: '翠苑一区业委会', glyph: '集',
    bg: 'linear-gradient(135deg, #5FD99A, #19BE6B)', joined: 1208, deadline: '2026-11-02', hot: false,
    questions: [
      { id: 'a', type: 'multi', title: '可选 2 项', required: true, options: ['手作甜品', '旧书交换', '宠物美容', '亲子手工', '鲜花绿植'], base: [612, 388, 296, 524, 402] }
    ]
  }
];

const MY_SEED = [
  { id: 'm1', kind: '投票', title: '周五部门聚餐吃什么？', options: ['火锅', '日料', '烧烤', '粤菜'], counts: [8, 5, 6, 3], deadline: '2026-10-31', anonymous: true, multi: false, created: '10-21' },
  { id: 'm2', kind: '投票', title: '读书会 11 月书单投票', options: ['《置身事内》', '《长安的荔枝》', '《人类简史》'], counts: [14, 21, 9], deadline: '2026-10-20', anonymous: false, multi: true, created: '10-12' }
];

function getMine() {
  const closed = wx.getStorageSync('sv_closed') || [];
  const seed = MY_SEED.map(m => (closed.indexOf(m.id) >= 0 ? Object.assign({}, m, { closed: true }) : m));
  return (wx.getStorageSync('sv_mine') || []).concat(seed);
}

// 用户发布的投票也能被作答：转成和内置问卷一样的结构
function toSurvey(m) {
  return {
    id: m.id, kind: '投票', title: m.title, desc: m.anonymous ? '匿名投票，发起人看不到你的选择' : '实名投票', author: '我', glyph: '投',
    bg: 'linear-gradient(135deg, #A08CFF, #6C5CE7)', joined: m.counts.reduce((a, b) => a + b, 0), deadline: m.deadline,
    questions: [{ id: 'a', type: m.multi ? 'multi' : 'single', title: m.multi ? '可多选' : '请选择一项', required: true, options: m.options, base: m.counts }]
  };
}

function getSurvey(id) {
  const s = surveys.find(x => x.id === id);
  if (s) return s;
  const m = getMine().find(x => x.id === id);
  return m ? toSurvey(m) : surveys[0];
}

function getAnswers() { return wx.getStorageSync('sv_answers') || {}; }

function today() {
  const d = new Date();
  const p = n => (n < 10 ? '0' + n : '' + n);
  return d.getFullYear() + '-' + p(d.getMonth() + 1) + '-' + p(d.getDate());
}

function fmtNum(n) { return n >= 10000 ? (n / 10000).toFixed(1) + '万' : String(n); }

module.exports = { today, surveys, getMine, getSurvey, getAnswers, fmtNum, toSurvey };
