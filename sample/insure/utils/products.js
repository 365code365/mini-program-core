// 产品库：险种、保障责任、测算参数。保费算法见 premium.js，条款与常见问题见 terms.js。
const terms = require('./terms.js');

const CATS = [
  { key: 'medical', name: '医疗险', icon: '/images/cat/medical.png', desc: '住院大病报销' },
  { key: 'critical', name: '重疾险', icon: '/images/cat/critical.png', desc: '确诊即赔' },
  { key: 'accident', name: '意外险', icon: '/images/cat/accident.png', desc: '出行无忧' },
  { key: 'life', name: '寿险', icon: '/images/cat/life.png', desc: '家庭责任' },
  { key: 'child', name: '少儿险', icon: '/images/cat/child.png', desc: '宝宝专属' },
  { key: 'travel', name: '旅行险', icon: '/images/cat/travel.png', desc: '境外出游' }
];

const PRODUCTS = [
  {
    id: 'p01', name: '安心百万医疗险（2026版）', cat: 'medical', model: 'medical',
    tagline: '一般医疗 300 万，重疾医疗 600 万，0 免赔', tags: ['保证续保20年', '1万免赔', '外购药可报'],
    insurer: '安心财产保险股份有限公司', ages: [0, 65], wait: 30, sales: 128643, hot: true,
    amounts: [2000000, 4000000, 6000000], terms: ['1年'], defAmount: 6000000, defTerm: '1年',
    coverage: [
      { name: '一般医疗保险金', amount: '300万', brief: '住院、特殊门诊、门诊手术', detail: '被保险人因意外或在等待期后因疾病，在二级及以上公立医院普通部住院，或接受特殊门诊、门诊手术、住院前后各 30 天门急诊治疗的合理且必要的医疗费用，扣除 1 万元年免赔额后按 100% 赔付（未经社保结算的按 60% 赔付）。' },
      { name: '重大疾病医疗保险金', amount: '600万', brief: '120 种重疾，0 免赔', detail: '被保险人初次确诊合同约定的 120 种重大疾病，因该重疾产生的住院、特殊门诊、门诊手术等医疗费用不设免赔额，按 100% 赔付，与一般医疗保险金共享 600 万年度限额。' },
      { name: '质子重离子医疗保险金', amount: '600万', brief: '恶性肿瘤先进疗法', detail: '被保险人确诊恶性肿瘤——重度，在上海市质子重离子医院等指定医疗机构接受质子、重离子放射治疗产生的费用，0 免赔，100% 赔付。' },
      { name: '院外特定药品费用', amount: '100万', brief: '覆盖 120 种特药', detail: '确诊恶性肿瘤后，凭处方在指定药店购买清单内的靶向药、免疫药等特定药品，0 免赔，100% 赔付（未经社保结算 60%）。' },
      { name: '重疾住院津贴', amount: '200元/天', brief: '每年最多 90 天', detail: '因重大疾病住院，按每天 200 元给付住院津贴，单次最多 30 天，年度累计最多 90 天。' }
    ]
  },
  {
    id: 'p02', name: '守护多倍重疾险', cat: 'critical', model: 'critical',
    tagline: '120 种重疾，不同组最高赔 3 次', tags: ['重疾多次赔', '轻中症豁免', '可选保终身'],
    insurer: '恒信人寿保险股份有限公司', ages: [0, 55], wait: 90, sales: 56210, hot: true,
    amounts: [300000, 500000, 800000], terms: ['保至70岁', '保终身'], defAmount: 500000, defTerm: '保终身',
    coverage: [
      { name: '重大疾病保险金', amount: '100%基本保额', brief: '120 种，分 5 组，最多赔 3 次', detail: '等待期后初次确诊合同约定的重大疾病，按基本保额给付；不同组别重疾间隔 180 天后可再次赔付，最多 3 次。' },
      { name: '中症疾病保险金', amount: '60%基本保额', brief: '25 种，最多赔 2 次', detail: '确诊合同约定的中症疾病，按基本保额的 60% 给付，每种限 1 次，累计最多 2 次。' },
      { name: '轻症疾病保险金', amount: '30%基本保额', brief: '40 种，最多赔 3 次', detail: '确诊合同约定的轻症疾病，按基本保额的 30% 给付，每种限 1 次，累计最多 3 次。' },
      { name: '身故保险金', amount: '已交保费', brief: '18 周岁后为基本保额', detail: '18 周岁前身故返还已交保费；18 周岁后身故按基本保额给付，合同终止。' },
      { name: '轻中症豁免', amount: '豁免后续保费', brief: '确诊轻中症后无需再交', detail: '确诊轻症或中症疾病后，豁免剩余各期保险费，合同继续有效。' }
    ]
  },
  {
    id: 'p03', name: '畅行综合意外险', cat: 'accident', model: 'accident',
    tagline: '意外身故伤残最高 200 万，意外医疗 0 免赔', tags: ['意外医疗0免赔', '含猝死保障', '交通意外加倍'],
    insurer: '安心财产保险股份有限公司', ages: [18, 65], wait: 0, sales: 98321, hot: true,
    amounts: [500000, 1000000, 2000000], terms: ['1年'], defAmount: 1000000, defTerm: '1年',
    coverage: [
      { name: '意外身故 / 伤残', amount: '100%保额', brief: '伤残按 1~10 级比例赔', detail: '因意外伤害事故身故的按保额给付；致残的按《人身保险伤残评定标准》对应等级比例（10%~100%）给付。' },
      { name: '交通意外额外保障', amount: '100%保额', brief: '乘坐公共交通工具', detail: '以乘客身份乘坐客运飞机、火车、轮船、公共汽车期间遭受意外身故或伤残，额外再赔一倍保额。' },
      { name: '意外医疗', amount: '5万', brief: '0 免赔，含门急诊', detail: '因意外产生的门急诊与住院医疗费用，0 免赔，社保范围内 100% 赔付，社保外 80% 赔付。' },
      { name: '猝死保障', amount: '50万', brief: '24 小时内', detail: '外表健康的被保险人突发急性疾病，在 24 小时内身故的，给付猝死保险金 50 万元。' },
      { name: '意外住院津贴', amount: '100元/天', brief: '最长 180 天', detail: '因意外住院，每天给付 100 元津贴，单次最长 180 天。' }
    ]
  },
  {
    id: 'p04', name: '安家定期寿险', cat: 'life', model: 'life',
    tagline: '身故 / 全残赔付，房贷车贷家庭保障', tags: ['健康告知宽松', '最高 300 万', '可选保至70岁'],
    insurer: '恒信人寿保险股份有限公司', ages: [18, 55], wait: 90, sales: 23517, hot: false,
    amounts: [500000, 1000000, 2000000, 3000000], terms: ['保至60岁', '保至70岁'], defAmount: 1000000, defTerm: '保至60岁',
    coverage: [
      { name: '身故保险金', amount: '100%基本保额', brief: '疾病或意外身故', detail: '等待期后因疾病身故或因意外身故的，按基本保额给付身故保险金，合同终止。' },
      { name: '全残保险金', amount: '100%基本保额', brief: '与身故责任二选一', detail: '等待期后因疾病或意外导致合同约定的全残，按基本保额给付全残保险金，合同终止。' },
      { name: '等待期内责任', amount: '已交保费', brief: '因疾病身故返还保费', detail: '等待期内因疾病身故或全残的，无息返还已交保险费，合同终止；因意外则不受等待期限制。' }
    ]
  },
  {
    id: 'p05', name: '小雨伞少儿医疗险', cat: 'child', model: 'child',
    tagline: '0~17 岁专属，住院门诊都能报', tags: ['出生满28天可保', '含门诊手术', '先进疗法'],
    insurer: '安心财产保险股份有限公司', ages: [0, 17], wait: 30, sales: 41288, hot: false,
    amounts: [2000000, 4000000], terms: ['1年'], defAmount: 4000000, defTerm: '1年',
    coverage: [
      { name: '住院医疗保险金', amount: '400万', brief: '1 万免赔，100% 赔付', detail: '因意外或等待期后患病住院产生的合理必要医疗费用，年免赔 1 万元，按 100% 赔付。' },
      { name: '少儿特定疾病医疗', amount: '400万', brief: '白血病等 20 种，0 免赔', detail: '确诊白血病、川崎病、重型再生障碍性贫血等 20 种少儿高发疾病，相关医疗费用 0 免赔。' },
      { name: '住院垫付', amount: '最高 50 万', brief: '三级医院直接垫付', detail: '在合作的三级公立医院住院，可申请保险公司直接向医院垫付押金与医疗费用。' }
    ]
  },
  {
    id: 'p06', name: '全球通境外旅行险', cat: 'travel', model: 'travel',
    tagline: '申根签证认可，境外医疗与紧急救援', tags: ['申根签证适用', '境外医疗直付', '航班延误'],
    insurer: '安心财产保险股份有限公司', ages: [1, 80], wait: 0, sales: 66903, hot: true,
    amounts: [300000, 500000, 1000000], terms: ['7天', '15天', '30天', '90天'], defAmount: 500000, defTerm: '15天',
    coverage: [
      { name: '境外意外与急性病医疗', amount: '100%保额', brief: '含住院、门急诊', detail: '旅行期间在境外因意外或突发急性病就医产生的医疗费用，0 免赔，100% 赔付，合作医院可直付。' },
      { name: '紧急医疗转运', amount: '50万', brief: '24 小时全球救援', detail: '由救援机构安排的紧急医疗转运、送返回国费用，全球 24 小时中文救援热线。' },
      { name: '航班延误', amount: '300元/4小时', brief: '最高 1200 元', detail: '所乘航班延误连续满 4 小时，每满 4 小时赔付 300 元，最高 1200 元。' },
      { name: '行李及证件损失', amount: '3000元', brief: '含护照补办费用', detail: '随身行李被盗、被抢或托运行李丢失损坏，以及护照遗失后的补办费用。' }
    ]
  },
  {
    id: 'p07', name: '银发防癌医疗险', cat: 'critical', model: 'cancer',
    tagline: '50~80 岁可投，有高血压糖尿病也能保', tags: ['三高可投', '恶性肿瘤0免赔', '质子重离子'],
    insurer: '恒信人寿保险股份有限公司', ages: [50, 80], wait: 90, sales: 18654, hot: false,
    amounts: [100000, 200000, 300000], terms: ['1年'], defAmount: 200000, defTerm: '1年',
    coverage: [
      { name: '恶性肿瘤确诊金', amount: '100%保额', brief: '确诊一次性给付', detail: '等待期后初次确诊恶性肿瘤——重度，按保额一次性给付确诊保险金。' },
      { name: '恶性肿瘤医疗', amount: '200万', brief: '住院、门诊、特药，0 免赔', detail: '恶性肿瘤相关的住院、特殊门诊、外购特药费用，0 免赔，社保内 100% 赔付。' },
      { name: '原位癌保险金', amount: '20%保额', brief: '限 1 次', detail: '初次确诊原位癌按保额的 20% 给付，给付后合同继续有效。' }
    ]
  },
  {
    id: 'p08', name: '小病无忧门急诊险', cat: 'medical', model: 'outpatient',
    tagline: '感冒发烧看门诊也能报，年度限额 1 万', tags: ['0免赔', '门急诊报销', '含购药'],
    insurer: '安心财产保险股份有限公司', ages: [0, 60], wait: 30, sales: 35072, hot: false,
    amounts: [10000], terms: ['1年'], defAmount: 10000, defTerm: '1年',
    coverage: [
      { name: '门急诊医疗费用', amount: '1万/年', brief: '单次限额 1000 元', detail: '在二级及以上公立医院普通部门急诊的检查、治疗、药品费用，经社保结算后 0 免赔，按 70% 赔付，单次最高 1000 元。' },
      { name: '住院医疗费用', amount: '2万/年', brief: '补充百万医疗免赔额', detail: '住院医疗费用经社保结算后 0 免赔，按 90% 赔付，可用于补足百万医疗险的 1 万免赔额。' }
    ]
  }
];

PRODUCTS.forEach(p => {
  const c = CATS.find(x => x.key === p.cat);
  p.catName = c.name;
  p.cover = '/images/cover/' + p.cat + '.jpg';
  p.clauses = terms.clauses(p);
  p.faqs = terms.faqs(p);
});

function getProduct(id) { return PRODUCTS.find(p => p.id === id) || null; }

module.exports = { CATS, PRODUCTS, getProduct };
