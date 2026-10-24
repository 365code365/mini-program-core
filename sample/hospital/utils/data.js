// 健康挂号：离线演示数据（医院、科室、医生、排班）
const hospital = {
  name: '华康大学附属仁和医院',
  level: '三级甲等',
  branch: '总院 · 徐汇院区',
  address: '上海市徐汇区宛平南路 600 号',
  phone: '021-6438 2200',
  hours: '门诊 07:30-17:00 · 急诊 24 小时'
};

const groups = [
  { id: 'nk', name: '内科', depts: [
    { id: 'xxg', name: '心血管内科', desc: '高血压、冠心病、心律失常' },
    { id: 'hx', name: '呼吸内科', desc: '慢性咳嗽、哮喘、肺结节' },
    { id: 'xh', name: '消化内科', desc: '胃炎、胃肠镜、肝病' },
    { id: 'sj', name: '神经内科', desc: '头痛、眩晕、睡眠障碍' },
    { id: 'nfm', name: '内分泌科', desc: '糖尿病、甲状腺疾病' },
    { id: 'sn', name: '肾内科', desc: '肾炎、尿蛋白、透析' }
  ] },
  { id: 'wk', name: '外科', depts: [
    { id: 'gk', name: '骨科', desc: '骨折、颈椎病、关节置换' },
    { id: 'pw', name: '普外科', desc: '甲状腺结节、疝气、胆囊' },
    { id: 'mnwk', name: '泌尿外科', desc: '结石、前列腺疾病' },
    { id: 'xw', name: '胸外科', desc: '肺部肿瘤、纵隔疾病' }
  ] },
  { id: 'fc', name: '妇产科', depts: [
    { id: 'fk', name: '妇科', desc: '月经不调、妇科炎症' },
    { id: 'ck', name: '产科', desc: '孕期检查、高危妊娠' },
    { id: 'sz', name: '生殖医学科', desc: '不孕不育、试管咨询' }
  ] },
  { id: 'ek', name: '儿科', depts: [
    { id: 'xer', name: '小儿内科', desc: '发热、咳嗽、腹泻' },
    { id: 'ebh', name: '儿童保健科', desc: '生长发育、疫苗咨询' },
    { id: 'xew', name: '小儿外科', desc: '包茎、疝气、骨折' }
  ] },
  { id: 'wg', name: '五官科', depts: [
    { id: 'yk', name: '眼科', desc: '近视、白内障、干眼' },
    { id: 'ebh2', name: '耳鼻喉科', desc: '鼻炎、中耳炎、咽炎' },
    { id: 'kq', name: '口腔科', desc: '补牙、种植牙、正畸' }
  ] },
  { id: 'zy', name: '中医科', depts: [
    { id: 'zynk', name: '中医内科', desc: '调理体质、失眠、脾胃' },
    { id: 'zj', name: '针灸推拿科', desc: '颈肩腰腿痛、面瘫' }
  ] },
  { id: 'pf', name: '皮肤科', depts: [
    { id: 'pfk', name: '皮肤科', desc: '湿疹、痤疮、荨麻疹' },
    { id: 'yxmr', name: '医学美容科', desc: '祛斑、光子嫩肤' }
  ] }
];

const doctors = [
  { id: 'd1', dept: 'xxg', name: '陈立华', title: '主任医师', tags: ['教授', '博士生导师'], good: '冠心病介入治疗、难治性高血压、心力衰竭的综合管理', rate: 99.2, visits: 12860, fee: 50, left: 3, color: '#3b8cff' },
  { id: 'd2', dept: 'xxg', name: '王晓雯', title: '副主任医师', tags: ['副教授'], good: '心律失常射频消融、房颤、早搏的诊治', rate: 98.7, visits: 8420, fee: 30, left: 12, color: '#20b2aa' },
  { id: 'd3', dept: 'xxg', name: '刘铭', title: '主治医师', tags: ['青年骨干'], good: '高血压、高血脂规范化诊疗，心血管病预防', rate: 97.9, visits: 5210, fee: 20, left: 0, color: '#7b6cf6' },
  { id: 'd4', dept: 'xxg', name: '赵静怡', title: '主任医师', tags: ['特需专家'], good: '结构性心脏病、心脏瓣膜病、肺动脉高压', rate: 99.5, visits: 15330, fee: 300, left: 1, color: '#f5794a' },
  { id: 'd5', dept: 'hx', name: '孙建国', title: '主任医师', tags: ['教授'], good: '慢阻肺、支气管哮喘、肺结节随访评估', rate: 98.9, visits: 10230, fee: 50, left: 6, color: '#3b8cff' },
  { id: 'd6', dept: 'hx', name: '周雨桐', title: '主治医师', tags: [], good: '慢性咳嗽、呼吸道感染、睡眠呼吸暂停', rate: 97.6, visits: 3860, fee: 20, left: 18, color: '#20b2aa' },
  { id: 'd7', dept: 'xh', name: '吴海涛', title: '副主任医师', tags: ['内镜中心'], good: '无痛胃肠镜、消化道早癌筛查、幽门螺杆菌根治', rate: 98.4, visits: 9120, fee: 30, left: 4, color: '#7b6cf6' },
  { id: 'd8', dept: 'gk', name: '郑凯', title: '主任医师', tags: ['教授', '运动医学'], good: '膝关节镜微创、半月板损伤、前交叉韧带重建', rate: 99.1, visits: 11040, fee: 50, left: 2, color: '#f5794a' },
  { id: 'd9', dept: 'yk', name: '林佳', title: '副主任医师', tags: [], good: '青少年近视防控、白内障超声乳化、干眼症', rate: 98.3, visits: 7300, fee: 30, left: 9, color: '#20b2aa' },
  { id: 'd10', dept: 'xer', name: '黄思远', title: '主治医师', tags: ['儿童友好'], good: '小儿发热、反复呼吸道感染、过敏性疾病', rate: 99.0, visits: 6680, fee: 20, left: 15, color: '#3b8cff' }
];

// WXML 里 {{name[0]}} 取字符串下标在引擎里取不到，姓氏预先算好
doctors.forEach(d => { d.sx = d.name.charAt(0); });

// 未来 7 天排班：按医生 id 生成固定号源（离线、可复现）
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function nextDays(n) {
  const out = [];
  const base = new Date();
  for (let i = 0; i < n; i++) {
    const d = new Date(base.getFullYear(), base.getMonth(), base.getDate() + i);
    out.push({
      key: d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()),
      md: pad(d.getMonth() + 1) + '-' + pad(d.getDate()),
      week: i === 0 ? '今天' : (i === 1 ? '明天' : WEEK[d.getDay()]),
      idx: i
    });
  }
  return out;
}
const SLOTS = [
  { time: '08:00-09:00', half: '上午' }, { time: '09:00-10:00', half: '上午' },
  { time: '10:00-11:00', half: '上午' }, { time: '11:00-11:30', half: '上午' },
  { time: '13:30-14:30', half: '下午' }, { time: '14:30-15:30', half: '下午' },
  { time: '15:30-16:30', half: '下午' }
];
function slotsFor(doctor, dayIdx) {
  const seed = parseInt(doctor.id.slice(1), 10);
  return SLOTS.map((s, i) => {
    const v = (seed * 7 + dayIdx * 5 + i * 3) % 9;
    const off = (seed + dayIdx) % 4 === 3 && i < 4; // 某些上午停诊
    return { time: s.time, half: s.half, left: off ? -1 : (v > 6 ? 0 : v + 1) };
  });
}

const news = [
  { id: 1, tag: '科普', title: '秋冬季流感高发，这几类人群建议尽早接种疫苗', read: '1.2 万', date: '10-21', color: '#3b8cff' },
  { id: 2, tag: '义诊', title: '本周六上午门诊大厅开展「护心行动」免费义诊', read: '8,640', date: '10-20', color: '#20b2aa' },
  { id: 3, tag: '指南', title: '体检报告上的箭头，哪些需要立刻就医？', read: '2.6 万', date: '10-18', color: '#f5794a' },
  { id: 4, tag: '通知', title: '徐汇院区停车场改造期间，请优先选择地铁 12 号线', read: '5,120', date: '10-16', color: '#7b6cf6' }
];

function findDoctor(id) { return doctors.find(d => d.id === id); }
function findDept(id) {
  for (const g of groups) {
    const d = g.depts.find(x => x.id === id);
    if (d) return d;
  }
  return null;
}

module.exports = { hospital, groups, doctors, news, nextDays, slotsFor, findDoctor, findDept };
