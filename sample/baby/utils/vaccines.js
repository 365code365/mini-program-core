// 国家免疫规划疫苗儿童免疫程序（2021 年版）0~4 岁部分 + 常见的非免疫规划（自费）疫苗。
// month：建议接种月龄；free：是否免费（免疫规划）
const LIST = [
  { code: 'hepb1', name: '乙肝疫苗', dose: '第1剂', month: 0, free: true, disease: '乙型病毒性肝炎', site: '上臂三角肌 肌内注射', tip: '出生后 24 小时内接种，越早越好。' },
  { code: 'bcg', name: '卡介苗', dose: '1剂', month: 0, free: true, disease: '结核病（重症结核性脑膜炎、粟粒性肺结核）', site: '上臂三角肌外下缘 皮内注射', tip: '接种 2~3 周后局部会出现红肿、化脓、结痂，属正常反应，不要挤压。' },
  { code: 'hepb2', name: '乙肝疫苗', dose: '第2剂', month: 1, free: true, disease: '乙型病毒性肝炎', site: '上臂三角肌 肌内注射', tip: '与第 1 剂间隔不少于 28 天。' },
  { code: 'ipv1', name: '脊灰灭活疫苗（IPV）', dose: '第1剂', month: 2, free: true, disease: '脊髓灰质炎（小儿麻痹症）', site: '大腿前外侧 肌内注射', tip: '注射后观察 30 分钟再离开接种点。' },
  { code: 'pcv1', name: '13价肺炎球菌结合疫苗', dose: '第1剂', month: 2, free: false, disease: '肺炎球菌引起的肺炎、脑膜炎、中耳炎', site: '大腿前外侧 肌内注射', tip: '自费疫苗，2/4/6 月龄基础免疫 3 剂，12~15 月龄加强 1 剂。' },
  { code: 'rota1', name: '口服五价轮状病毒疫苗', dose: '第1剂', month: 2, free: false, disease: '轮状病毒腹泻（秋季腹泻）', site: '口服', tip: '自费疫苗，6~12 周龄开始口服，每剂间隔 4~10 周，共 3 剂。服苗前后半小时不要喂奶。' },
  { code: 'ipv2', name: '脊灰灭活疫苗（IPV）', dose: '第2剂', month: 3, free: true, disease: '脊髓灰质炎', site: '大腿前外侧 肌内注射', tip: '' },
  { code: 'dtap1', name: '百白破疫苗', dose: '第1剂', month: 3, free: true, disease: '百日咳、白喉、破伤风', site: '上臂外侧三角肌或大腿前外侧 肌内注射', tip: '少数宝宝会有低热、局部红肿，一般 1~2 天消退。' },
  { code: 'bopv3', name: '脊灰减毒活疫苗（bOPV）', dose: '第3剂', month: 4, free: true, disease: '脊髓灰质炎', site: '口服', tip: '服苗后半小时内不要喝热水、吃奶。' },
  { code: 'dtap2', name: '百白破疫苗', dose: '第2剂', month: 4, free: true, disease: '百日咳、白喉、破伤风', site: '肌内注射', tip: '' },
  { code: 'pcv2', name: '13价肺炎球菌结合疫苗', dose: '第2剂', month: 4, free: false, disease: '肺炎球菌感染', site: '大腿前外侧 肌内注射', tip: '' },
  { code: 'dtap3', name: '百白破疫苗', dose: '第3剂', month: 5, free: true, disease: '百日咳、白喉、破伤风', site: '肌内注射', tip: '' },
  { code: 'hepb3', name: '乙肝疫苗', dose: '第3剂', month: 6, free: true, disease: '乙型病毒性肝炎', site: '上臂三角肌 肌内注射', tip: '' },
  { code: 'mpsva1', name: 'A群流脑多糖疫苗', dose: '第1剂', month: 6, free: true, disease: '流行性脑脊髓膜炎', site: '上臂外侧三角肌 皮下注射', tip: '' },
  { code: 'pcv3', name: '13价肺炎球菌结合疫苗', dose: '第3剂', month: 6, free: false, disease: '肺炎球菌感染', site: '大腿前外侧 肌内注射', tip: '' },
  { code: 'ev71', name: '肠道病毒71型灭活疫苗', dose: '第1剂', month: 6, free: false, disease: 'EV71 引起的手足口病', site: '上臂三角肌 肌内注射', tip: '自费疫苗，共 2 剂，间隔 1 个月。' },
  { code: 'flu', name: '流感疫苗（儿童型）', dose: '首年第1剂', month: 6, free: false, disease: '季节性流感', site: '上臂三角肌 肌内注射', tip: '6 月龄以上首次接种需 2 剂，间隔 4 周；每年秋季前接种。' },
  { code: 'mmr1', name: '麻腮风疫苗', dose: '第1剂', month: 8, free: true, disease: '麻疹、流行性腮腺炎、风疹', site: '上臂外侧三角肌下缘 皮下注射', tip: '接种后 5~12 天可能出现一过性发热或皮疹。' },
  { code: 'je1', name: '乙脑减毒活疫苗', dose: '第1剂', month: 8, free: true, disease: '流行性乙型脑炎', site: '上臂外侧三角肌下缘 皮下注射', tip: '' },
  { code: 'mpsva2', name: 'A群流脑多糖疫苗', dose: '第2剂', month: 9, free: true, disease: '流行性脑脊髓膜炎', site: '皮下注射', tip: '与第 1 剂间隔 3 个月。' },
  { code: 'var1', name: '水痘减毒活疫苗', dose: '第1剂', month: 12, free: false, disease: '水痘', site: '上臂三角肌 皮下注射', tip: '部分省市已纳入免费程序，以当地接种门诊为准。' },
  { code: 'dtap4', name: '百白破疫苗', dose: '第4剂', month: 18, free: true, disease: '百日咳、白喉、破伤风', site: '肌内注射', tip: '' },
  { code: 'mmr2', name: '麻腮风疫苗', dose: '第2剂', month: 18, free: true, disease: '麻疹、流行性腮腺炎、风疹', site: '皮下注射', tip: '' },
  { code: 'hepa', name: '甲肝减毒活疫苗', dose: '1剂', month: 18, free: true, disease: '甲型病毒性肝炎', site: '上臂外侧三角肌 皮下注射', tip: '' },
  { code: 'je2', name: '乙脑减毒活疫苗', dose: '第2剂', month: 24, free: true, disease: '流行性乙型脑炎', site: '皮下注射', tip: '' },
  { code: 'mpsvac1', name: 'A群C群流脑多糖疫苗', dose: '第1剂', month: 36, free: true, disease: '流行性脑脊髓膜炎', site: '皮下注射', tip: '' },
  { code: 'bopv4', name: '脊灰减毒活疫苗（bOPV）', dose: '第4剂', month: 48, free: true, disease: '脊髓灰质炎', site: '口服', tip: '' }
];

function ageLabel(m) {
  if (m === 0) return '出生时';
  if (m < 24) return m + '月龄';
  return (m / 12) + '岁';
}

module.exports = { LIST, ageLabel };
