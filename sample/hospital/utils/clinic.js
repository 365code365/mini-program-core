// 诊疗模板（离线演示）：就诊结束后医生开的检查与药、检查报告的内容、问诊回复、住院每日费用
// 处方按科室挑；没有专门模板的科室按所在分组挑
const BILL_BY_DEPT = {
  hx: [{ name: '血常规', cat: '检验', price: 28, qty: 1, report: 'cbc' }, { name: '胸部 CT 平扫', cat: '检查', price: 260, qty: 1, report: 'ct' }, { name: '复方甘草口服溶液 100ml', cat: '药品', price: 12.5, qty: 2 }],
  xxg: [{ name: '心电图', cat: '检查', price: 35, qty: 1, report: 'ecg' }, { name: '血脂四项', cat: '检验', price: 48, qty: 1, report: 'lipid' }, { name: '阿托伐他汀钙片 20mg×7', cat: '药品', price: 18.6, qty: 2 }],
  yk: [{ name: '视力 + 眼压检查', cat: '检查', price: 40, qty: 1, report: 'eye' }, { name: '玻璃酸钠滴眼液 5ml', cat: '药品', price: 38.6, qty: 1 }],
  ym: [{ name: '四价流感疫苗', cat: '药品', price: 128, qty: 1 }, { name: '接种服务费', cat: '治疗', price: 10, qty: 1 }],
  tj: [{ name: '血常规', cat: '检验', price: 28, qty: 1, report: 'cbc' }, { name: '血脂四项', cat: '检验', price: 48, qty: 1, report: 'lipid' }, { name: '心电图', cat: '检查', price: 35, qty: 1, report: 'ecg' }]
};
const BILL_BY_GROUP = {
  nk: [{ name: '血常规', cat: '检验', price: 28, qty: 1, report: 'cbc' }, { name: '奥美拉唑肠溶胶囊 20mg×14', cat: '药品', price: 24.8, qty: 1 }],
  wk: [{ name: '患处 X 线摄片', cat: '检查', price: 120, qty: 1, report: 'xray' }, { name: '双氯芬酸钠缓释片', cat: '药品', price: 23.5, qty: 1 }, { name: '换药治疗', cat: '治疗', price: 15, qty: 1 }],
  fc: [{ name: '妇科彩超', cat: '检查', price: 150, qty: 1, report: 'us' }, { name: '血常规', cat: '检验', price: 28, qty: 1, report: 'cbc' }],
  ek: [{ name: '血常规 + C 反应蛋白', cat: '检验', price: 45, qty: 1, report: 'cbc' }, { name: '布洛芬混悬液 100ml', cat: '药品', price: 26.8, qty: 1 }],
  wg: [{ name: '电子鼻咽喉镜', cat: '检查', price: 180, qty: 1, report: 'scope' }, { name: '糠酸莫米松鼻喷雾剂', cat: '药品', price: 66.5, qty: 1 }],
  zy: [{ name: '中药饮片 7 剂', cat: '药品', price: 168, qty: 1 }, { name: '针灸治疗', cat: '治疗', price: 60, qty: 1 }],
  pf: [{ name: '皮肤镜检查', cat: '检查', price: 80, qty: 1, report: 'derm' }, { name: '丁酸氢化可的松乳膏', cat: '药品', price: 21.5, qty: 1 }],
  yf: [{ name: '健康咨询', cat: '治疗', price: 10, qty: 1 }]
};
function billItems(deptId, groupId) {
  return (BILL_BY_DEPT[deptId] || BILL_BY_GROUP[groupId] || BILL_BY_GROUP.nk).map(x => Object.assign({}, x));
}
// 复诊续方：医生审核通过后开的药
const RENEW_ITEMS = [{ name: '续方药品（按上次处方）', cat: '药品', price: 56.4, qty: 1 }, { name: '药事服务费', cat: '治疗', price: 8, qty: 1 }];
// 检查报告：检验类给结果表（flag：高 / 低），检查类给所见与结论
const REPORTS = {
  cbc: { name: '血常规', kind: '检验', items: [
    { n: '白细胞计数 WBC', v: '6.8', u: '10^9/L', r: '3.5-9.5', f: '' },
    { n: '中性粒细胞百分比', v: '72.4', u: '%', r: '40-75', f: '' },
    { n: '淋巴细胞百分比', v: '18.2', u: '%', r: '20-50', f: '低' },
    { n: '红细胞计数 RBC', v: '4.62', u: '10^12/L', r: '4.3-5.8', f: '' },
    { n: '血红蛋白 HGB', v: '142', u: 'g/L', r: '130-175', f: '' },
    { n: '血小板计数 PLT', v: '268', u: '10^9/L', r: '125-350', f: '' },
    { n: 'C 反应蛋白 CRP', v: '12.6', u: 'mg/L', r: '0-10', f: '高' }
  ], conclusion: '淋巴细胞百分比略低、C 反应蛋白轻度升高，提示近期可能存在感染，请结合症状复诊。' },
  lipid: { name: '血脂四项', kind: '检验', items: [
    { n: '总胆固醇 TC', v: '5.92', u: 'mmol/L', r: '0-5.2', f: '高' },
    { n: '甘油三酯 TG', v: '1.46', u: 'mmol/L', r: '0-1.7', f: '' },
    { n: '高密度脂蛋白 HDL-C', v: '1.12', u: 'mmol/L', r: '1.0-1.9', f: '' },
    { n: '低密度脂蛋白 LDL-C', v: '3.86', u: 'mmol/L', r: '0-3.4', f: '高' }
  ], conclusion: '总胆固醇与低密度脂蛋白偏高，建议低脂饮食、规律运动，遵医嘱服用降脂药，3 个月后复查。' },
  ct: { name: '胸部 CT 平扫', kind: '检查', findings: '两肺纹理稍增多，右肺下叶见一枚直径约 4mm 磨玻璃小结节，边界清晰；纵隔淋巴结未见肿大，双侧胸腔未见积液。', conclusion: '右肺下叶微小结节（考虑良性可能大），建议 12 个月后复查胸部 CT。' },
  ecg: { name: '心电图', kind: '检查', findings: '窦性心律，心率 72 次/分，电轴不偏，各导联 ST-T 未见明显异常。', conclusion: '正常心电图。' },
  eye: { name: '视力 + 眼压检查', kind: '检查', findings: '裸眼视力 右 0.6 左 0.5，矫正视力 右 1.0 左 1.0；眼压 右 15mmHg 左 16mmHg。', conclusion: '双眼屈光不正（近视），眼压正常。建议规范配镜，每半年复查。' },
  xray: { name: '患处 X 线摄片', kind: '检查', findings: '所摄骨质结构完整，未见明确骨折线，关节间隙正常，周围软组织稍肿胀。', conclusion: '未见明显骨折，软组织损伤可能，建议制动休息。' },
  us: { name: '妇科彩超', kind: '检查', findings: '子宫大小形态正常，内膜厚约 8mm，回声均匀；双侧附件区未见明显包块。', conclusion: '子宫及双附件未见明显异常。' },
  scope: { name: '电子鼻咽喉镜', kind: '检查', findings: '双侧下鼻甲肿大，黏膜充血，鼻道见少量黏性分泌物；咽部轻度充血，声带运动正常。', conclusion: '慢性鼻炎急性发作。' },
  derm: { name: '皮肤镜检查', kind: '检查', findings: '皮损区见红色背景下点状血管，散在少许白色鳞屑，未见色素网异常。', conclusion: '符合湿疹样皮炎表现，建议外用药治疗 2 周后复诊。' }
};
// 问诊回复：按问诊方式与科室分组
const REPLY = {
  text: {
    nk: '您好，根据您的描述，建议先规律作息、清淡饮食，近期避免劳累。如症状持续超过一周或加重，请到院做血常规等检查，我可以帮您安排复诊。',
    wk: '您好，从描述看以软组织劳损可能性大，建议局部制动、冷敷 48 小时后改热敷。若出现明显肿胀或活动受限，请尽快到院拍片。',
    ek: '您好，孩子体温超过 38.5 度可按体重服用退烧药，多补充水分。若精神差、持续高热超过 3 天或呼吸急促，请立即到儿科急诊。',
    other: '您好，已收到您的情况描述。建议您先观察症状变化，按说明规律用药；如有加重请到院面诊，我已为您备注就诊建议。'
  },
  video: '您好，我是今天的接诊医生。已查看您提交的资料，请保持网络通畅，我将在 10 分钟内发起视频问诊。',
  renew: '您好，已核对您上次的处方与近期用药情况，可以按原方案续方。处方已开具，请在「门诊缴费」支付后到门诊 1 楼药房取药。'
};
function consultReply(type, groupId) {
  if (type === 'text') return REPLY.text[groupId] || REPLY.text.other;
  return REPLY[type] || REPLY.text.other;
}
// 住院每日费用：第 k 天（入院当天 k = 0）的明细，按天固定生成
const DRUGS = [286.4, 245.8, 198.6, 156.2, 142.0, 128.5, 118.3];
function dailyItems(k) {
  const items = [
    { name: '床位费（三人间）', amount: 75 },
    { name: '一级护理', amount: 36 },
    { name: '诊查费', amount: 18 },
    { name: '西药费', amount: DRUGS[k % DRUGS.length] }
  ];
  if (k < 4) items.push({ name: '静脉输液', amount: 42 });
  if (k === 0) items.push({ name: '心电图 + 血常规', amount: 63 }, { name: '心脏彩超', amount: 280 });
  if (k === 1) items.push({ name: '冠脉 CTA', amount: 980 });
  if (k === 2) items.push({ name: '血脂四项', amount: 48 });
  return items;
}
module.exports = { billItems, RENEW_ITEMS, REPORTS, consultReply, dailyItems };
