// 律安心的静态数据：咨询分类、律师、热门问答。律师与律所均为虚构，价格参照常见线上法律咨询平台的区间。
const categories = [
  { id: 'labor', name: '劳动纠纷', desc: '欠薪·辞退·工伤', ic: '劳', color: '#2f6fe0' },
  { id: 'marriage', name: '婚姻家事', desc: '离婚·抚养·继承', ic: '婚', color: '#d6587f' },
  { id: 'debt', name: '借贷债务', desc: '欠款·担保·催收', ic: '债', color: '#c08a1e' },
  { id: 'house', name: '房产租赁', desc: '买卖·租房·物业', ic: '房', color: '#2c9a7a' },
  { id: 'traffic', name: '交通事故', desc: '责任·理赔·伤残', ic: '车', color: '#e0702f' },
  { id: 'contract', name: '合同纠纷', desc: '违约·审查·解除', ic: '约', color: '#6b5bd6' },
  { id: 'consumer', name: '消费维权', desc: '退款·欺诈·假货', ic: '消', color: '#d64545' },
  { id: 'company', name: '公司经营', desc: '股权·合规·用工', ic: '企', color: '#3c4658' }
];

const lawyers = [
  {
    id: 'l1', name: '周明远', avatar: '/images/lawyer/l1.jpg', firm: '浙江衡言律师事务所', city: '杭州', years: 12,
    fields: ['labor', 'company'], rating: 4.9, consults: 3862, reply: '平均 6 分钟回复',
    price: { text: 39, phone: 99, doc: 299 },
    intro: '执业 12 年，专注劳动争议与企业用工合规，代理劳动仲裁及诉讼案件 600 余件，曾任多家制造业企业常年法律顾问。',
    cases: ['某电子厂 42 名员工集体欠薪仲裁，全部支持', '高管违法解除争议，获赔 2N 共计 31.6 万元', '协助连锁餐饮企业搭建用工合规制度'],
    reviews: [
      { user: '陈**', star: 5, text: '把仲裁流程和需要准备的证据一条条列出来了，很清楚。', date: '2026-10-28' },
      { user: '王**', star: 5, text: '回复快，算的 N+1 金额和公司最后给的一致。', date: '2026-10-19' }
    ]
  },
  {
    id: 'l2', name: '林婉清', avatar: '/images/lawyer/l2.jpg', firm: '上海申和律师事务所', city: '上海', years: 9,
    fields: ['marriage', 'house'], rating: 4.8, consults: 2741, reply: '平均 10 分钟回复',
    price: { text: 49, phone: 129, doc: 399 },
    intro: '婚姻家事律师，擅长离婚财产分割、子女抚养权与遗产继承，注重调解，帮助当事人体面解决纠纷。',
    cases: ['婚前按揭房增值部分分割，争取到 186 万元补偿', '跨省抚养权变更诉讼胜诉', '三代人遗嘱继承纠纷调解结案'],
    reviews: [
      { user: '刘**', star: 5, text: '很耐心，没有一上来就让我打官司。', date: '2026-10-30' },
      { user: '赵**', star: 4, text: '分析得很细，就是周末回复稍慢。', date: '2026-10-12' }
    ]
  },
  {
    id: 'l3', name: '陈国栋', avatar: '/images/lawyer/l3.jpg', firm: '广东粤信律师事务所', city: '深圳', years: 15,
    fields: ['debt', 'contract', 'company'], rating: 4.9, consults: 5120, reply: '平均 8 分钟回复',
    price: { text: 59, phone: 159, doc: 499 },
    intro: '高级合伙人，主办民商事合同与债权债务纠纷，熟悉保全与执行程序，累计为客户追回欠款逾 2 亿元。',
    cases: ['建设工程款纠纷，申请财产保全后 3 个月执行到位', '民间借贷 80 万元本息全额支持', '股权转让合同违约金调整案'],
    reviews: [{ user: '孙**', star: 5, text: '一针见血，告诉我先做保全，后来果然追回来了。', date: '2026-10-25' }]
  },
  {
    id: 'l4', name: '许静怡', avatar: '/images/lawyer/l4.jpg', firm: '北京京和律师事务所', city: '北京', years: 7,
    fields: ['consumer', 'contract'], rating: 4.7, consults: 1893, reply: '平均 12 分钟回复',
    price: { text: 39, phone: 99, doc: 299 },
    intro: '专注消费者权益保护与电商纠纷，熟悉《消费者权益保护法》《电子商务法》，擅长退一赔三、预付卡退费类案件。',
    cases: ['健身房预付卡跑路，协助 37 名会员集体维权', '二手车隐瞒事故，判决退一赔三', '教培机构退费纠纷调解'],
    reviews: [{ user: '李**', star: 5, text: '给了投诉模板，12315 很快就受理了。', date: '2026-10-21' }]
  },
  {
    id: 'l5', name: '何振华', avatar: '/images/lawyer/l5.jpg', firm: '四川蜀鼎律师事务所', city: '成都', years: 18,
    fields: ['traffic', 'labor'], rating: 4.8, consults: 4307, reply: '平均 9 分钟回复',
    price: { text: 49, phone: 119, doc: 359 },
    intro: '处理交通事故与人身损害赔偿案件近 20 年，熟悉伤残鉴定与保险理赔，帮助伤者争取合理赔偿。',
    cases: ['外卖骑手交通事故，认定工伤并获赔 46 万元', '十级伤残理赔，赔偿额较保险公司初报提高 2.3 倍'],
    reviews: [{ user: '吴**', star: 5, text: '讲明白了交强险和商业险分别赔哪些。', date: '2026-10-27' }]
  },
  {
    id: 'l6', name: '宋雨桐', avatar: '/images/lawyer/l6.jpg', firm: '江苏金陵律师事务所', city: '南京', years: 6,
    fields: ['house', 'marriage', 'consumer'], rating: 4.7, consults: 1420, reply: '平均 15 分钟回复',
    price: { text: 29, phone: 89, doc: 259 },
    intro: '房产与租赁纠纷律师，处理过大量押金不退、群租、二房东转租、物业服务纠纷，收费透明。',
    cases: ['长租公寓暴雷，协助租客追回押金与预付租金', '商品房逾期交付违约金诉讼'],
    reviews: [{ user: '郑**', star: 4, text: '解答到位，推荐的取证方法很实用。', date: '2026-10-16' }]
  }
];

const qa = [
  {
    id: 'q1', cat: 'labor', title: '公司没签劳动合同，能要求双倍工资吗？', views: 18234,
    answer: '<p>可以。根据《劳动合同法》第八十二条，用人单位自用工之日起<b>超过一个月不满一年</b>未与劳动者订立书面劳动合同的，应当向劳动者每月支付<b>二倍的工资</b>。</p><p style="margin-top:8px">注意：二倍工资最多支付 11 个月，申请仲裁的时效为一年。建议保存工资流水、考勤记录、工作群聊天等证据。</p>'
  },
  {
    id: 'q2', cat: 'labor', title: '被公司辞退，N+1 怎么算？', views: 25610,
    answer: '<p>N 指工作年限：每满一年支付一个月工资；六个月以上不满一年的按一年计算；不满六个月的支付半个月工资。</p><p style="margin-top:8px">「+1」是代通知金，仅在用人单位<b>未提前 30 日书面通知</b>而依据第四十条解除时支付。月工资按解除前 12 个月平均应得工资计算，高于当地社平工资三倍的按三倍封顶，年限最多 12 年。</p>'
  },
  {
    id: 'q3', cat: 'house', title: '房东不退押金怎么办？', views: 9873,
    answer: '<p>先核对租赁合同中关于押金退还的约定，拍照留存退租时的房屋状况与交接记录。</p><p style="margin-top:8px">协商不成的，可向住建部门或街道调解委员会投诉，也可向房屋所在地人民法院起诉，小额案件适用<b>小额诉讼程序</b>，一审终审，审理较快。</p>'
  },
  {
    id: 'q4', cat: 'debt', title: '借钱只有微信转账记录，没有借条，能起诉吗？', views: 14402,
    answer: '<p>可以。微信转账记录配合聊天记录中对借款事实的确认（如「借你的 5 万什么时候还」），可以形成证据链。</p><p style="margin-top:8px">建议：截图要包含对方头像、微信号；必要时到公证处做电子证据保全。民间借贷的诉讼时效为<b>三年</b>，从约定的还款期限届满之日起算。</p>'
  },
  {
    id: 'q5', cat: 'marriage', title: '离婚时婚前买的房子怎么分？', views: 21133,
    answer: '<p>一方婚前全款购买的房屋原则上属于个人财产。婚前首付、婚后共同还贷的，离婚时房屋一般归登记方，对婚后共同还贷支付的款项及其相对应的财产增值部分，由登记方对另一方进行补偿。</p>'
  },
  {
    id: 'q6', cat: 'consumer', title: '网购买到假货，可以要求三倍赔偿吗？', views: 8765,
    answer: '<p>经营者提供商品有<b>欺诈行为</b>的，消费者可以要求增加赔偿购买商品价款的三倍，增加赔偿的金额不足五百元的，为五百元（《消费者权益保护法》第五十五条）。</p><p style="margin-top:8px">食品类不符合安全标准的，可依《食品安全法》主张价款十倍或损失三倍赔偿，最低一千元。</p>'
  }
];

// 服务类型（radio-group）
const services = [
  { id: 'text', name: '图文咨询', desc: '24 小时内不限次追问', key: 'text' },
  { id: 'phone', name: '电话咨询', desc: '30 分钟通话，预约时段', key: 'phone' },
  { id: 'doc', name: '文书审查', desc: '合同 / 协议审查并出具修改意见', key: 'doc' }
];

function catName(id) {
  const c = categories.find(x => x.id === id);
  return c ? c.name : '';
}
function lawyerById(id) { return lawyers.find(x => x.id === id); }
function decorate(l) {
  return Object.assign({}, l, { fieldText: l.fields.map(catName).join(' · ') });
}

module.exports = { categories, lawyers, qa, services, catName, lawyerById, decorate };
