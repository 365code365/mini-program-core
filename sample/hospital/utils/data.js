// 健康挂号：离线演示数据（医院、科室、医生、排班、资讯、导诊）
const hospital = {
  name: '华康大学附属仁和医院',
  short: '华康仁和医院',
  level: '三级甲等',
  branch: '总院 · 徐汇院区',
  address: '上海市徐汇区宛平南路 600 号',
  phone: '021-6438 2200',
  emergency: '021-6438 2120',
  hours: '门诊 07:30-17:00 · 急诊 24 小时',
  lat: 31.1849,
  lng: 121.4473,
  traffic: [
    { k: '地铁', v: '12 号线龙华中路站 2 号口，步行约 6 分钟' },
    { k: '公交', v: '44 路、104 路、864 路「宛平南路龙华西路」站' },
    { k: '停车', v: '院内停车场改造中，请停放宛平南路 588 号社会停车场（¥10/小时）' }
  ]
};
// room：诊区位置（门诊楼几楼几区），院内导航按楼层汇总
const groups = [
  { id: 'nk', name: '内科', depts: [
    { id: 'xxg', name: '心血管内科', desc: '高血压、冠心病、心律失常', room: '门诊 3 楼 A 区 301' },
    { id: 'hx', name: '呼吸内科', desc: '慢性咳嗽、哮喘、肺结节', room: '门诊 3 楼 B 区 307' },
    { id: 'xh', name: '消化内科', desc: '胃炎、胃肠镜、肝病', room: '门诊 3 楼 C 区 312' },
    { id: 'sj', name: '神经内科', desc: '头痛、眩晕、睡眠障碍', room: '门诊 4 楼 A 区 401' },
    { id: 'nfm', name: '内分泌科', desc: '糖尿病、甲状腺疾病', room: '门诊 4 楼 B 区 406' },
    { id: 'sn', name: '肾内科', desc: '肾炎、尿蛋白、透析', room: '门诊 4 楼 C 区 410' }
  ] },
  { id: 'wk', name: '外科', depts: [
    { id: 'gk', name: '骨科', desc: '骨折、颈椎病、关节置换', room: '门诊 5 楼 A 区 501' },
    { id: 'pw', name: '普外科', desc: '甲状腺结节、疝气、胆囊', room: '门诊 5 楼 B 区 506' },
    { id: 'mnwk', name: '泌尿外科', desc: '结石、前列腺疾病', room: '门诊 5 楼 C 区 512' },
    { id: 'xw', name: '胸外科', desc: '肺部肿瘤、纵隔疾病', room: '门诊 5 楼 C 区 515' }
  ] },
  { id: 'fc', name: '妇产科', depts: [
    { id: 'fk', name: '妇科', desc: '月经不调、妇科炎症', room: '门诊 6 楼 A 区 601' },
    { id: 'ck', name: '产科', desc: '孕期检查、高危妊娠', room: '门诊 6 楼 B 区 606' },
    { id: 'sz', name: '生殖医学科', desc: '不孕不育、试管咨询', room: '门诊 6 楼 C 区 612' }
  ] },
  { id: 'ek', name: '儿科', depts: [
    { id: 'xer', name: '小儿内科', desc: '发热、咳嗽、腹泻', room: '门诊 1 楼 B 区 105' },
    { id: 'ebh', name: '儿童保健科', desc: '生长发育、疫苗咨询', room: '门诊 1 楼 B 区 108' },
    { id: 'xew', name: '小儿外科', desc: '包茎、疝气、骨折', room: '门诊 1 楼 C 区 112' }
  ] },
  { id: 'wg', name: '五官科', depts: [
    { id: 'yk', name: '眼科', desc: '近视、白内障、干眼', room: '门诊 2 楼 A 区 205' },
    { id: 'ebh2', name: '耳鼻喉科', desc: '鼻炎、中耳炎、咽炎', room: '门诊 2 楼 B 区 210' },
    { id: 'kq', name: '口腔科', desc: '补牙、种植牙、正畸', room: '门诊 2 楼 C 区 216' }
  ] },
  { id: 'zy', name: '中医科', depts: [
    { id: 'zynk', name: '中医内科', desc: '调理体质、失眠、脾胃', room: '门诊 7 楼 A 区 701' },
    { id: 'zj', name: '针灸推拿科', desc: '颈肩腰腿痛、面瘫', room: '门诊 7 楼 B 区 706' }
  ] },
  { id: 'pf', name: '皮肤科', depts: [
    { id: 'pfk', name: '皮肤科', desc: '湿疹、痤疮、荨麻疹', room: '门诊 4 楼 D 区 418' },
    { id: 'yxmr', name: '医学美容科', desc: '祛斑、光子嫩肤', room: '门诊 4 楼 D 区 422' }
  ] },
  { id: 'yf', name: '预防保健', depts: [
    { id: 'ym', name: '预防接种门诊', desc: '流感疫苗、HPV、带状疱疹疫苗', room: '门诊 1 楼 A 区 101' },
    { id: 'tj', name: '健康体检中心', desc: '入职体检、年度体检套餐', room: '门诊 1 楼 A 区 103' }
  ] }
];
// 门诊楼各层的公共设施（院内导航用；科室诊区按 groups 里的 room 汇总到楼层）
const facilities = {
  1: ['挂号收费处', '自助机', '门诊药房', '急诊（东侧 24 小时）'],
  2: ['检验科采血处'],
  3: ['放射科 CT / X 光'],
  4: ['超声科'],
  5: ['日间手术中心'],
  6: ['母婴室'],
  7: ['中药房']
};
const doctors = [
  { id: 'd1', dept: 'xxg', name: '陈立华', title: '主任医师', tags: ['教授', '博士生导师'], good: '冠心病介入治疗、难治性高血压、心力衰竭的综合管理', rate: 99.2, visits: 12860, fee: 50, color: '#3b8cff', star: true },
  { id: 'd2', dept: 'xxg', name: '王晓雯', title: '副主任医师', tags: ['副教授'], good: '心律失常射频消融、房颤、早搏的诊治', rate: 98.7, visits: 8420, fee: 30, color: '#20b2aa' },
  // left: 0 = 一周号源都已约满（演示「已约满」）
  { id: 'd3', dept: 'xxg', name: '刘铭', title: '主治医师', tags: ['青年骨干'], good: '高血压、高血脂规范化诊疗，心血管病预防', rate: 97.9, visits: 5210, fee: 20, left: 0, color: '#7b6cf6' },
  { id: 'd4', dept: 'xxg', name: '赵静怡', title: '主任医师', tags: ['特需专家'], good: '结构性心脏病、心脏瓣膜病、肺动脉高压', rate: 99.5, visits: 15330, fee: 300, color: '#f5794a', star: true },
  { id: 'd5', dept: 'hx', name: '孙建国', title: '主任医师', tags: ['教授'], good: '慢阻肺、支气管哮喘、肺结节随访评估', rate: 98.9, visits: 10230, fee: 50, color: '#3b8cff', star: true },
  { id: 'd6', dept: 'hx', name: '周雨桐', title: '主治医师', tags: [], good: '慢性咳嗽、呼吸道感染、睡眠呼吸暂停', rate: 97.6, visits: 3860, fee: 20, color: '#20b2aa' },
  { id: 'd7', dept: 'xh', name: '吴海涛', title: '副主任医师', tags: ['内镜中心'], good: '无痛胃肠镜、消化道早癌筛查、幽门螺杆菌根治', rate: 98.4, visits: 9120, fee: 30, color: '#7b6cf6' },
  { id: 'd8', dept: 'gk', name: '郑凯', title: '主任医师', tags: ['教授', '运动医学'], good: '膝关节镜微创、半月板损伤、前交叉韧带重建', rate: 99.1, visits: 11040, fee: 50, color: '#f5794a', star: true },
  { id: 'd9', dept: 'yk', name: '林佳', title: '副主任医师', tags: [], good: '青少年近视防控、白内障超声乳化、干眼症', rate: 98.3, visits: 7300, fee: 30, color: '#20b2aa' },
  { id: 'd10', dept: 'xer', name: '黄思远', title: '主治医师', tags: ['儿童友好'], good: '小儿发热、反复呼吸道感染、过敏性疾病', rate: 99.0, visits: 6680, fee: 20, color: '#3b8cff' }
];
// 其余科室每个排两位出诊医生（固定生成，离线可复现）：从前借用心内科医生，挂号单上的科室就对不上了
const NAMES = ['高文博', '许嘉怡', '冯振宇', '邓慧敏', '韩立新', '曹雪琴', '彭志远', '蒋若楠', '谢永康', '潘晓燕', '杜明哲', '叶秋婷',
  '程浩然', '袁静雯', '何俊峰', '吕思琪', '丁国庆', '沈佳音', '任泽宇', '姚梦洁', '卢振东', '钟雅琳', '汪海波', '田欣然',
  '方建伟', '石婉清', '崔明亮', '康雨薇', '贾志诚', '夏若曦', '万鹏飞', '秦书瑶', '戴晨阳', '陆婷婷', '孔德明', '邱雅文'];
const COLORS = ['#3b8cff', '#20b2aa', '#7b6cf6', '#f5794a'];
let gen = 0;
groups.forEach(g => g.depts.forEach(dp => {
  if (doctors.some(d => d.dept === dp.id)) return;
  [['主任医师', 50, ['教授']], ['主治医师', 20, []]].forEach(t => {
    gen++;
    doctors.push({
      id: 'g' + gen, dept: dp.id, name: NAMES[(gen - 1) % NAMES.length], title: t[0], tags: t[2],
      good: dp.desc + '的规范化诊治与长期随访', rate: Math.round((97.2 + (gen % 23) / 10) * 10) / 10,
      visits: 2600 + gen * 397, fee: t[1], color: COLORS[gen % 4]
    });
  });
}));
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
// left：余号数；0 约满；-1 停诊（「已过」与扣掉自己已挂的号在 store.slotsFor 里算）
function slotsFor(doctor, dayIdx) {
  const seed = parseInt(doctor.id.slice(1), 10);
  return SLOTS.map((s, i) => {
    const v = (seed * 7 + dayIdx * 5 + i * 3) % 9;
    const off = (seed + dayIdx) % 4 === 3 && i < 4; // 某些上午停诊
    const full = doctor.left === 0 || v > 6;
    return { time: s.time, half: s.half, left: off ? -1 : (full ? 0 : v + 1) };
  });
}
// 资讯：action 是文末按钮（去挂号 / 去问诊 / 去导航）
const news = [
  { id: 1, tag: '科普', title: '秋冬季流感高发，这几类人群建议尽早接种疫苗', read: '1.2 万', date: '10-21', color: '#3b8cff',
    body: ['每年 10 月至次年 3 月是流感高发季。流感病毒变异快，往年接种产生的抗体会随时间减弱，建议每年接种一次。',
      '60 岁以上老人、6 月龄至 5 岁儿童、慢性病患者、孕妇及医务人员属于重点人群，接种后约 2 周产生保护性抗体。',
      '本院预防接种门诊（门诊 1 楼 A 区）周一至周六开放，四价流感疫苗可在线预约，接种当天请携带身份证或医保卡。'],
    action: { name: '预约接种', type: 'dept', id: 'ym' } },
  { id: 2, tag: '义诊', title: '本周六上午门诊大厅开展「护心行动」免费义诊', read: '8,640', date: '10-20', color: '#20b2aa',
    body: ['本周六 08:30-11:30，心血管内科陈立华主任团队将在门诊大厅开展免费义诊，现场提供血压、血糖测量与心电图检查。',
      '高血压、冠心病、心律失常患者可携带近期检查报告与正在服用的药物，专家现场解读并调整用药建议。',
      '义诊无需挂号；如需进一步检查，可现场预约心血管内科门诊。'],
    action: { name: '预约心血管内科', type: 'dept', id: 'xxg' } },
  { id: 3, tag: '指南', title: '体检报告上的箭头，哪些需要立刻就医？', read: '2.6 万', date: '10-18', color: '#f5794a',
    body: ['体检报告上的「高 / 低」标记只表示结果超出参考范围，不一定就是疾病。轻度异常多与饮食、作息、采血前状态有关。',
      '需要尽快就医的情况：血红蛋白明显降低、空腹血糖达到 7.0 mmol/L 及以上、肝功能转氨酶超过正常上限 3 倍、肿瘤标志物明显升高、影像提示占位。',
      '看不懂报告时，可以使用「在线问诊」，全科医生 30 分钟内回复解读。'],
    action: { name: '在线问诊解读报告', type: 'consult' } },
  { id: 4, tag: '通知', title: '徐汇院区停车场改造期间，请优先选择地铁 12 号线', read: '5,120', date: '10-16', color: '#7b6cf6',
    body: ['即日起至 12 月 31 日，徐汇院区地面停车场进行改造，院内车位暂停对外开放。',
      '自驾就诊请停放宛平南路 588 号社会停车场，凭当日挂号单可享 2 小时停车优惠。',
      '建议优先乘坐地铁 12 号线至龙华中路站 2 号口，步行约 6 分钟到达门诊楼。'],
    action: { name: '查看院内导航', type: 'navi' } }
];
// 智能导诊：部位 → 症状 → 推荐科室（urgent：提示急诊）
const symptomAreas = [
  { key: 'head', name: '头颈部', items: [{ n: '头痛', d: 'sj' }, { n: '头晕眩晕', d: 'sj' }, { n: '失眠多梦', d: 'zynk' }, { n: '颈肩酸痛', d: 'zj' }, { n: '甲状腺肿大', d: 'pw' }] },
  { key: 'chest', name: '胸部', items: [{ n: '咳嗽咳痰', d: 'hx' }, { n: '胸闷心慌', d: 'xxg' }, { n: '胸痛', d: 'xxg', urgent: true }, { n: '气短喘憋', d: 'hx' }, { n: '乳房肿块', d: 'pw' }] },
  { key: 'belly', name: '腹部', items: [{ n: '胃痛反酸', d: 'xh' }, { n: '腹泻便秘', d: 'xh' }, { n: '腰痛尿频', d: 'mnwk' }, { n: '月经不调', d: 'fk' }, { n: '剧烈腹痛', d: 'pw', urgent: true }] },
  { key: 'limb', name: '四肢关节', items: [{ n: '膝盖疼痛', d: 'gk' }, { n: '腰腿痛', d: 'zj' }, { n: '手脚麻木', d: 'sj' }, { n: '扭伤骨折', d: 'gk' }] },
  { key: 'face', name: '五官', items: [{ n: '视力下降', d: 'yk' }, { n: '眼睛干涩', d: 'yk' }, { n: '鼻塞流涕', d: 'ebh2' }, { n: '咽喉痛', d: 'ebh2' }, { n: '牙痛', d: 'kq' }] },
  { key: 'skin', name: '皮肤', items: [{ n: '皮疹瘙痒', d: 'pfk' }, { n: '痤疮', d: 'pfk' }, { n: '色斑', d: 'yxmr' }] },
  { key: 'kid', name: '儿童', items: [{ n: '发热', d: 'xer' }, { n: '咳嗽', d: 'xer' }, { n: '生长发育', d: 'ebh' }, { n: '疫苗咨询', d: 'ebh' }] },
  { key: 'whole', name: '全身', items: [{ n: '发热乏力', d: 'hx' }, { n: '口渴多尿', d: 'nfm' }, { n: '体重骤变', d: 'nfm' }, { n: '浮肿', d: 'sn' }, { n: '想做体检', d: 'tj' }] }
];
function findDoctor(id) { return doctors.find(d => d.id === id); }
function findDept(id) {
  for (const g of groups) {
    const d = g.depts.find(x => x.id === id);
    if (d) return d;
  }
  return null;
}
function groupOfDept(id) { return groups.find(g => g.depts.some(d => d.id === id)) || groups[0]; }
function findNews(id) { return news.find(n => n.id === Number(id)); }
module.exports = { hospital, groups, facilities, doctors, news, symptomAreas, nextDays, slotsFor, findDoctor, findDept, groupOfDept, findNews };
