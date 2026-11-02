// 政务服务事项目录（虚构的「云川市」，部门名称按地级市常见设置，不对应任何真实机构）。
// 字段：promise 承诺时限（工作日）、legal 法定时限、visits 跑动次数、level 网办深度、fee 收费说明、
// materials[].copies「原件 N / 复印件 N」、steps 办理流程（详情页画成流程图）。
const THEMES = [
  { id: 'hukou', name: '户籍证件', color: '#1d64d8' },
  { id: 'social', name: '社保就业', color: '#16a34a' },
  { id: 'house', name: '住房不动产', color: '#ea580c' },
  { id: 'biz', name: '市场主体', color: '#7c3aed' },
  { id: 'traffic', name: '交通出行', color: '#0891b2' },
  { id: 'medical', name: '医保生育', color: '#db2777' },
  { id: 'civil', name: '民政养老', color: '#ca8a04' }
];

const DEPTS = [
  '市公安局', '市人力资源和社会保障局', '市住房公积金管理中心', '市自然资源和规划局',
  '市市场监督管理局', '市医疗保障局', '市民政局'
];

const STD_STEPS = ['网上申请', '窗口受理', '部门审核', '办结送达'];

const ITEMS = [
  {
    id: 'G0101', name: '居民身份证换领', theme: 'hukou', dept: '市公安局', hot: 1, online: false,
    promise: 15, legal: 60, visits: 1, level: '最多跑一次', fee: '工本费 20 元（按国家标准收取）',
    window: '各区公安分局户政服务大厅、云川市政务服务中心 B 区 12~15 号窗口',
    conditions: ['具有云川市常住户口的中国公民', '居民身份证有效期满前 3 个月内或已过期', '证件信息需变更（如姓名、出生日期更正后）'],
    materials: [
      { name: '居民户口簿', copies: '原件 1', required: true },
      { name: '原居民身份证', copies: '原件 1', required: true },
      { name: '符合标准的数码照片回执', copies: '原件 1', required: false, note: '可在窗口现场免费采集' }
    ],
    steps: ['预约取号', '现场采集照片指纹', '窗口受理', '制证', '领取或邮寄']
  },
  {
    id: 'G0102', name: '居住证办理', theme: 'hukou', dept: '市公安局', hot: 1, online: true,
    promise: 7, legal: 15, visits: 0, level: '全程网办', fee: '首次申领免费',
    window: '居住地派出所、社区警务室',
    conditions: ['离开常住户口所在地，到云川市居住半年以上', '有合法稳定就业、合法稳定住所或连续就读条件之一'],
    materials: [
      { name: '申请人居民身份证', copies: '扫描件 1', required: true },
      { name: '房屋租赁合同或房产证明', copies: '扫描件 1', required: true },
      { name: '劳动合同或营业执照', copies: '扫描件 1', required: false, note: '就业类申请提供' }
    ],
    steps: STD_STEPS
  },
  {
    id: 'G0103', name: '新生儿出生登记', theme: 'hukou', dept: '市公安局', hot: 0, online: true,
    promise: 3, legal: 10, visits: 0, level: '全程网办', fee: '不收费',
    window: '父母一方户口所在地派出所',
    conditions: ['新生儿出生后 1 个月内申报', '随父或随母落户，申报人为父母一方'],
    materials: [
      { name: '出生医学证明', copies: '扫描件 1', required: true },
      { name: '父母双方居民身份证', copies: '扫描件 2', required: true },
      { name: '父母结婚证', copies: '扫描件 1', required: true },
      { name: '落户方居民户口簿', copies: '扫描件 1', required: true }
    ],
    steps: STD_STEPS
  },
  {
    id: 'G0201', name: '灵活就业人员参保登记', theme: 'social', dept: '市人力资源和社会保障局', hot: 1, online: true,
    promise: 1, legal: 5, visits: 0, level: '全程网办', fee: '不收费（缴费基数按上年度社平工资 60%~300% 自选）',
    window: '市社会保险服务中心 1~6 号窗口',
    conditions: ['年满 16 周岁未达法定退休年龄', '无雇工的个体工商户、未在用人单位参保的非全日制从业人员'],
    materials: [
      { name: '居民身份证', copies: '扫描件 1', required: true },
      { name: '灵活就业人员参保登记表', copies: '扫描件 1', required: true }
    ],
    steps: ['网上申请', '系统核验', '登记办结']
  },
  {
    id: 'G0202', name: '失业保险金申领', theme: 'social', dept: '市人力资源和社会保障局', hot: 0, online: true,
    promise: 5, legal: 10, visits: 0, level: '全程网办', fee: '不收费',
    window: '各区公共就业服务中心',
    conditions: ['失业前用人单位和本人已缴纳失业保险费满 1 年', '非因本人意愿中断就业', '已办理失业登记并有求职要求'],
    materials: [
      { name: '居民身份证', copies: '扫描件 1', required: true },
      { name: '解除或终止劳动关系证明', copies: '扫描件 1', required: true },
      { name: '本人社会保障卡（金融账户）', copies: '扫描件 1', required: true }
    ],
    steps: STD_STEPS
  },
  {
    id: 'G0301', name: '住房公积金提取（租房）', theme: 'house', dept: '市住房公积金管理中心', hot: 1, online: true,
    promise: 1, legal: 3, visits: 0, level: '全程网办', fee: '不收费',
    window: '市住房公积金管理中心各管理部',
    conditions: ['本人及配偶在云川市无自有住房且租赁住房', '连续足额缴存住房公积金满 3 个月', '每年可提取一次，单次提取额度不超过 1.5 万元'],
    materials: [
      { name: '居民身份证', copies: '扫描件 1', required: true },
      { name: '房屋租赁合同', copies: '扫描件 1', required: true },
      { name: '本人一类银行卡', copies: '扫描件 1', required: true }
    ],
    steps: ['网上申请', '系统核验', '资金到账']
  },
  {
    id: 'G0302', name: '不动产登记资料查询', theme: 'house', dept: '市自然资源和规划局', hot: 1, online: true,
    promise: 0, legal: 1, visits: 0, level: '即时办结', fee: '不收费',
    window: '市不动产登记中心查询窗口',
    conditions: ['权利人查询本人名下不动产登记结果', '利害关系人凭相关证明查询'],
    materials: [
      { name: '查询人居民身份证', copies: '扫描件 1', required: true },
      { name: '不动产登记资料查询申请书', copies: '扫描件 1', required: true }
    ],
    steps: ['网上申请', '人脸核验', '即时出具查询结果']
  },
  {
    id: 'G0401', name: '个体工商户设立登记', theme: 'biz', dept: '市市场监督管理局', hot: 1, online: true,
    promise: 1, legal: 15, visits: 0, level: '全程网办', fee: '不收费',
    window: '云川市政务服务中心 A 区商事登记窗口',
    conditions: ['有经营能力的自然人', '有固定的经营场所', '经营范围不属于禁止类'],
    materials: [
      { name: '经营者居民身份证', copies: '扫描件 1', required: true },
      { name: '经营场所使用证明（租赁合同或产权证明）', copies: '扫描件 1', required: true },
      { name: '个体工商户登记（备案）申请书', copies: '扫描件 1', required: true },
      { name: '委托代理人证明', copies: '扫描件 1', required: false, note: '委托他人办理时提供' }
    ],
    steps: STD_STEPS
  },
  {
    id: 'G0402', name: '食品经营许可（小餐饮）', theme: 'biz', dept: '市市场监督管理局', hot: 0, online: true,
    promise: 7, legal: 20, visits: 0, level: '全程网办', fee: '不收费',
    window: '各区市场监督管理局行政审批窗口',
    conditions: ['已取得营业执照', '经营场所面积 50 平方米以下', '具有与经营品种相适应的设备设施'],
    materials: [
      { name: '营业执照', copies: '扫描件 1', required: true },
      { name: '经营场所平面布局图', copies: '扫描件 1', required: true },
      { name: '食品安全管理制度文本', copies: '扫描件 1', required: true },
      { name: '从业人员健康证明', copies: '扫描件 1', required: true }
    ],
    steps: ['网上申请', '材料审查', '现场核查', '作出许可决定', '发证']
  },
  {
    id: 'G0501', name: '机动车驾驶证有效期满换证', theme: 'traffic', dept: '市公安局', hot: 1, online: true,
    promise: 3, legal: 5, visits: 0, level: '全程网办', fee: '工本费 10 元，邮寄费用由寄递企业收取',
    window: '市公安局交通警察支队车管所及各区车管分所',
    conditions: ['驾驶证有效期满前 90 日内', '无未处理的交通违法记录和交通事故', '已提交有效期内的体检证明'],
    materials: [
      { name: '居民身份证', copies: '扫描件 1', required: true },
      { name: '机动车驾驶人身体条件证明', copies: '扫描件 1', required: true },
      { name: '近期免冠彩色照片', copies: '电子版 1', required: true }
    ],
    steps: STD_STEPS
  },
  {
    id: 'G0601', name: '生育津贴申领', theme: 'medical', dept: '市医疗保障局', hot: 1, online: true,
    promise: 10, legal: 20, visits: 0, level: '全程网办', fee: '不收费',
    window: '市医疗保障服务中心 3~5 号窗口',
    conditions: ['职工生育时用人单位已为其连续缴纳生育保险满 12 个月', '符合法律法规规定的生育条件'],
    materials: [
      { name: '居民身份证', copies: '扫描件 1', required: true },
      { name: '出生医学证明', copies: '扫描件 1', required: true },
      { name: '出院小结', copies: '扫描件 1', required: true }
    ],
    steps: STD_STEPS
  },
  {
    id: 'G0602', name: '城乡居民医保参保登记', theme: 'medical', dept: '市医疗保障局', hot: 0, online: true,
    promise: 0, legal: 1, visits: 0, level: '即时办结', fee: '2026 年度个人缴费 400 元',
    window: '各街道（乡镇）便民服务中心',
    conditions: ['具有云川市户籍或持有云川市居住证', '未参加职工基本医疗保险'],
    materials: [{ name: '居民身份证或户口簿', copies: '扫描件 1', required: true }],
    steps: ['网上申请', '系统核验', '登记办结']
  },
  {
    id: 'G0701', name: '高龄津贴申领', theme: 'civil', dept: '市民政局', hot: 0, online: true,
    promise: 10, legal: 20, visits: 0, level: '全程网办', fee: '不收费（80~89 周岁每月 100 元，90 周岁以上每月 300 元）',
    window: '户籍所在地街道（乡镇）便民服务中心',
    conditions: ['具有云川市户籍', '年满 80 周岁'],
    materials: [
      { name: '老年人居民身份证', copies: '扫描件 1', required: true },
      { name: '本人社会保障卡', copies: '扫描件 1', required: true },
      { name: '代办人身份证', copies: '扫描件 1', required: false, note: '他人代办时提供' }
    ],
    steps: STD_STEPS
  },
  {
    id: 'G0702', name: '婚姻登记预约', theme: 'civil', dept: '市民政局', hot: 0, online: false,
    promise: 0, legal: 0, visits: 1, level: '最多跑一次', fee: '不收费',
    window: '云川市婚姻登记处（新城区府前路 18 号）',
    conditions: ['男方年满 22 周岁、女方年满 20 周岁', '双方自愿且均无配偶', '双方共同到场'],
    materials: [
      { name: '双方居民身份证', copies: '原件 2', required: true },
      { name: '双方居民户口簿', copies: '原件 2', required: true },
      { name: '近期 2 寸半身免冠合影照片', copies: '3 张', required: true }
    ],
    steps: ['预约取号', '现场填表声明', '审查', '颁证']
  }
];

function themeOf(id) { return THEMES.find(t => t.id === id) || THEMES[0]; }
function getItem(id) { return ITEMS.find(i => i.id === id); }

// 网点：预约取号用
const HALLS = [
  { id: 'H1', name: '云川市政务服务中心', addr: '新城区府前路 99 号', phone: '0871-6288 1234', img: '/images/hall1.jpg', time: '工作日 09:00-17:00，周六 09:00-12:00' },
  { id: 'H2', name: '滨江区政务服务中心', addr: '滨江区江湾大道 216 号', phone: '0871-6288 2235', img: '/images/hall2.jpg', time: '工作日 09:00-17:00' },
  { id: 'H3', name: '青山区政务服务中心', addr: '青山区青岚路 37 号', phone: '0871-6288 3326', img: '/images/hall3.jpg', time: '工作日 09:00-17:00' }
];

module.exports = { THEMES, DEPTS, ITEMS, HALLS, themeOf, getItem };
