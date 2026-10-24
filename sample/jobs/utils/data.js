// 职聘直通：离线演示数据（职位、公司、会话）
const companies = {
  c1: { id: 'c1', name: '字节跳动', short: '字', color: 'linear-gradient(135deg, #3C8CFF, #1F5FE0)', industry: '互联网·内容社区', size: '10000人以上', stage: '已上市', addr: '杭州市余杭区文一西路 998 号海创园 5 号楼', intro: '公司致力于用技术连接人与信息，旗下产品覆盖内容、社交、企业协作等多个领域，在杭州设有研发中心，团队氛围开放，重视工程质量与数据驱动。', welfare: ['六险一金', '免费三餐', '弹性工作', '年度体检', '租房补贴', '带薪年假 15 天'], jobs: ['j1', 'j5'] },
  c2: { id: 'c2', name: '网易严选', short: '网', color: 'linear-gradient(135deg, #FF6B6B, #E03C4A)', industry: '电子商务·新零售', size: '1000-9999人', stage: '已上市', addr: '杭州市滨江区网商路 599 号网易大厦', intro: '以「好的生活，没那么贵」为理念的自营电商品牌，覆盖居家、服饰、美食等品类，供应链与设计团队实力雄厚。', welfare: ['五险一金', '年终奖', '员工折扣', '下午茶', '定期团建'], jobs: ['j2'] },
  c3: { id: 'c3', name: '蚂蚁集团', short: '蚁', color: 'linear-gradient(135deg, #2BC4A9, #0E9C86)', industry: '金融科技', size: '10000人以上', stage: '不需要融资', addr: '杭州市西湖区西溪路 556 号蚂蚁 A 空间', intro: '以科技推动普惠服务，业务涵盖数字支付、数字金融科技平台与数字化服务，技术栈覆盖分布式、隐私计算与大模型。', welfare: ['六险一金', '股票期权', '免费班车', '健身房', '子女教育补贴'], jobs: ['j3', 'j6'] },
  c4: { id: 'c4', name: '有赞科技', short: '赞', color: 'linear-gradient(135deg, #FFA23A, #F06A00)', industry: '企业服务·SaaS', size: '1000-9999人', stage: '已上市', addr: '杭州市西湖区西斗门路 3 号天堂软件园 E 幢', intro: '帮助商家网上开店、社交营销的 SaaS 服务商，工程师文化浓厚，鼓励开源与技术分享。', welfare: ['五险一金', '餐补', '弹性工作', '技术大会门票'], jobs: ['j4'] },
  c5: { id: 'c5', name: '米哈游', short: '米', color: 'linear-gradient(135deg, #9A7BFF, #6B4FE0)', industry: '游戏', size: '1000-9999人', stage: '未融资', addr: '上海市徐汇区宜山路 700 号普天信息产业园', intro: '技术宅拯救世界。专注原创内容与高品质游戏研发，美术、程序、策划团队都是行业顶尖水平。', welfare: ['六险一金', '免费三餐', '十三薪', '节日福利', '员工宿舍'], jobs: ['j7'] }
};

const hrs = {
  h1: { name: '王女士', title: '招聘经理', bg: 'linear-gradient(135deg, #FFB3C1, #FF7A98)', ch: '王', online: true },
  h2: { name: '陈先生', title: '技术总监', bg: 'linear-gradient(135deg, #9EC9FF, #4C8DFF)', ch: '陈', online: false },
  h3: { name: '李女士', title: 'HRBP', bg: 'linear-gradient(135deg, #A6EBD4, #2BC49C)', ch: '李', online: true },
  h4: { name: '赵先生', title: '前端负责人', bg: 'linear-gradient(135deg, #FFD59A, #FF9F2E)', ch: '赵', online: true },
  h5: { name: '周女士', title: '人事主管', bg: 'linear-gradient(135deg, #D5C2FF, #9A7BFF)', ch: '周', online: false }
};

const jobs = [
  { id: 'j1', title: '高级前端开发工程师', salary: '25-45K·15薪', city: '杭州', area: '余杭区·未来科技城', exp: '3-5年', edu: '本科', company: 'c1', hr: 'h4', tags: ['React', 'TypeScript', '小程序', '性能优化'], dist: 3.2, posted: 1, hot: 98, desc: ['负责内容社区 Web 与小程序端核心业务的研发与迭代；', '参与前端基础设施建设，包括组件库、构建工具与监控体系；', '持续优化首屏性能与交互体验，推动工程化最佳实践落地。'], req: ['3 年以上前端开发经验，精通 React 或 Vue 其一；', '熟悉 TypeScript、Webpack/Vite，有大型项目工程化经验；', '有小程序或跨端框架开发经验者优先。'] },
  { id: 'j2', title: '电商运营专员', salary: '10-15K', city: '杭州', area: '滨江区·长河', exp: '1-3年', edu: '本科', company: 'c2', hr: 'h1', tags: ['店铺运营', '活动策划', '数据分析'], dist: 6.8, posted: 0, hot: 76, desc: ['负责居家类目日常运营与大促活动策划执行；', '跟踪销售数据，分析用户行为，输出优化方案；', '协同设计、供应链推进新品上线。'], req: ['1 年以上电商运营经验，熟悉主流平台规则；', '数据敏感，熟练使用 Excel 与常见 BI 工具；', '沟通能力强，有责任心。'] },
  { id: 'j3', title: 'Java 后端开发工程师', salary: '30-50K·16薪', city: '杭州', area: '西湖区·西溪', exp: '3-5年', edu: '本科', company: 'c3', hr: 'h2', tags: ['Java', '分布式', 'MySQL', '高并发'], dist: 4.5, posted: 2, hot: 92, desc: ['负责支付核心链路的服务设计与开发；', '参与高可用架构演进，保障大促期间系统稳定；', '编写高质量代码与技术文档，参与代码评审。'], req: ['计算机相关专业本科及以上，3 年以上 Java 开发经验；', '熟悉 Spring、MyBatis、消息队列与缓存中间件；', '有金融或支付系统经验者优先。'] },
  { id: 'j4', title: 'UI/UX 设计师', salary: '15-25K', city: '杭州', area: '西湖区·文三路', exp: '3-5年', edu: '大专', company: 'c4', hr: 'h3', tags: ['Figma', 'B 端设计', '设计规范'], dist: 2.1, posted: 0, hot: 64, desc: ['负责商家后台与移动端产品的交互与视觉设计；', '建设与维护设计系统，提升设计一致性与效率；', '参与用户研究，推动体验问题改进。'], req: ['3 年以上互联网产品设计经验，有 B 端作品；', '熟练使用 Figma、Sketch 等工具；', '良好的逻辑思维与表达能力。'] },
  { id: 'j5', title: '数据分析师', salary: '18-30K·14薪', city: '杭州', area: '余杭区·未来科技城', exp: '1-3年', edu: '硕士', company: 'c1', hr: 'h1', tags: ['SQL', 'Python', 'A/B 实验'], dist: 3.4, posted: 3, hot: 81, desc: ['负责业务核心指标体系搭建与日常监控；', '设计并分析 A/B 实验，给出可落地的产品建议；', '沉淀分析方法与数据产品。'], req: ['统计、数学、计算机相关专业硕士；', '熟练使用 SQL 与 Python，熟悉常见统计方法；', '有互联网数据分析实习或工作经验。'] },
  { id: 'j6', title: '产品经理（支付方向）', salary: '25-40K·16薪', city: '杭州', area: '西湖区·西溪', exp: '3-5年', edu: '本科', company: 'c3', hr: 'h5', tags: ['支付', 'B 端产品', '需求分析'], dist: 4.6, posted: 1, hot: 88, desc: ['负责商户支付产品规划与迭代；', '深入理解商户场景，挖掘需求并推动落地；', '协调研发、设计、运营等多方资源。'], req: ['3 年以上产品经验，有支付或金融产品经验优先；', '逻辑清晰，文档能力强；', '具备良好的项目推动能力。'] },
  { id: 'j7', title: '游戏客户端开发（Unity）', salary: '20-40K·13薪', city: '上海', area: '徐汇区·漕河泾', exp: '1-3年', edu: '本科', company: 'c5', hr: 'h4', tags: ['Unity', 'C#', '图形渲染'], dist: 168, posted: 4, hot: 95, desc: ['负责游戏客户端玩法与系统功能开发；', '参与渲染效果与性能优化；', '配合策划与美术完成版本迭代。'], req: ['熟悉 Unity 引擎与 C#，有完整项目经验；', '了解图形学基础与常见渲染管线；', '热爱游戏，有作品者优先。'] }
];

const chats = [
  { id: 's1', hr: 'h4', company: 'c1', job: 'j1', last: '您好，看了您的简历，方便明天下午聊一下吗？', time: '10:32', unread: 2 },
  { id: 's2', hr: 'h1', company: 'c2', job: 'j2', last: '岗位还在招的，欢迎投递简历～', time: '09:15', unread: 1 },
  { id: 's3', hr: 'h2', company: 'c3', job: 'j3', last: '[面试邀请] 10月28日 14:00 视频面试', time: '昨天', unread: 0 },
  { id: 's4', hr: 'h3', company: 'c4', job: 'j4', last: '作品集收到了，我们评估后尽快回复您', time: '周二', unread: 0 },
  { id: 's5', hr: 'h5', company: 'c3', job: 'j6', last: '好的，谢谢您的关注', time: '10/19', unread: 0 }
];

function decorate(j) {
  const c = companies[j.company];
  const h = hrs[j.hr];
  return Object.assign({}, j, { co: c, hrInfo: h, distText: j.dist >= 100 ? '异地' : j.dist.toFixed(1) + 'km' });
}

function getJob(id) {
  const j = jobs.find(x => x.id === id) || jobs[0];
  return decorate(j);
}

module.exports = { companies, hrs, jobs, chats, decorate, getJob };
