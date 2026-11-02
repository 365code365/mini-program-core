// 投保页用的静态数据：职业分类（两级，括号里是 1~6 类职业风险等级）与健康告知问题。
const OCCUPATIONS = [
  { name: '机关/事业单位', jobs: [['公务员', 1], ['教师', 1], ['医生', 2], ['护士', 2], ['警察（内勤）', 2], ['消防救援人员', 6]] },
  { name: '企业/办公', jobs: [['企业管理人员', 1], ['软件工程师', 1], ['财务人员', 1], ['行政/人事', 1], ['外勤销售', 2], ['客服专员', 1]] },
  { name: '交通运输', jobs: [['网约车司机', 3], ['出租车司机', 3], ['货车司机', 4], ['快递员', 4], ['外卖骑手', 4], ['远洋船员', 5]] },
  { name: '建筑/制造', jobs: [['建筑设计师', 1], ['工地管理人员', 3], ['车间操作工', 3], ['电工', 4], ['高空作业人员', 6], ['井下矿工', 6]] },
  { name: '其他', jobs: [['在校学生', 1], ['家庭主妇/主夫', 1], ['退休人员', 1], ['自由职业', 2], ['农民', 2], ['个体经营', 2]] }
];

function occColumns(i) {
  return [OCCUPATIONS.map(o => o.name), OCCUPATIONS[i].jobs.map(j => j[0] + '（' + j[1] + '类）')];
}
function occOf(idx) {
  const o = OCCUPATIONS[idx[0]], j = o.jobs[idx[1]] || o.jobs[0];
  return { text: o.name + ' / ' + j[0], level: j[1] };
}

// 健康告知：意外险、旅行险不需要；防癌险对三高宽松（不问慢性病）
const HEALTH = [
  { v: 'hosp', t: '过去 2 年内住院，或被医生建议住院 / 手术' },
  { v: 'chronic', t: '患有或曾患高血压、糖尿病、冠心病等慢性病', skip: ['cancer'] },
  { v: 'severe', t: '患有或曾患恶性肿瘤、脑卒中、肝硬化、肾功能不全' },
  { v: 'exam', t: '近 1 年体检有异常（结节、肿块、肿瘤标志物升高等）' },
  { v: 'none', t: '以上情况均没有' }
];
function healthFor(model) {
  if (model === 'accident' || model === 'travel') return [];
  return HEALTH.filter(h => !(h.skip && h.skip.indexOf(model) >= 0)).map(h => ({ v: h.v, t: h.t }));
}

module.exports = { OCCUPATIONS, occColumns, occOf, healthFor };
