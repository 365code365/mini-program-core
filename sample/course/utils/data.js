// 课程、讲师、题库：纯离线数据
const categories = ['全部', '编程开发', '职场提升', '外语学习', '考研考公', '设计创作', '理财投资', '兴趣生活'];

const teachers = [
  { id: 't1', name: '陈思远', title: '前阿里资深架构师', field: '编程开发', color: 'linear-gradient(135deg, #6f9bff, #3d6cf6)', fans: '12.6万', courses: 8,
    intro: '12 年互联网研发经验，主导过日活千万级系统的架构升级，擅长把复杂的技术讲得通俗易懂。' },
  { id: 't2', name: '林晓雯', title: '北外英语系副教授', field: '外语学习', color: 'linear-gradient(135deg, #ff9fb6, #f0577a)', fans: '9.8万', courses: 5,
    intro: '剑桥大学应用语言学硕士，雅思口语考官培训背景，累计带出 3 万余名高分学员。' },
  { id: 't3', name: '周明哲', title: 'CFA 持证人', field: '理财投资', color: 'linear-gradient(135deg, #ffc46b, #ff8a3d)', fans: '7.2万', courses: 4,
    intro: '前头部券商研究所首席分析师，专注个人资产配置与基金投资教育。' },
  { id: 't4', name: '许嘉禾', title: '资深 UI 设计总监', field: '设计创作', color: 'linear-gradient(135deg, #8de0c1, #20b486)', fans: '6.5万', courses: 6,
    intro: '曾任一线大厂设计中心负责人，作品获红点设计奖，教学风格实战派。' },
  { id: 't5', name: '王立诚', title: '公考培训首席讲师', field: '考研考公', color: 'linear-gradient(135deg, #b79bff, #7b5cf0)', fans: '15.1万', courses: 7,
    intro: '深耕行测申论教学 10 年，独创「图形推理五步法」，学员上岸率行业领先。' }
];

function chapters(names) {
  return names.map((c, i) => ({
    id: 'c' + (i + 1),
    title: c[0],
    lessons: c[1].map((l, j) => ({ id: 'l' + (i + 1) + '-' + (j + 1), title: l[0], mins: l[1], free: i === 0 && j < 2 }))
  }));
}

const courses = [
  { id: 'k1', title: 'Python 从入门到实战：数据分析全流程', cat: '编程开发', teacher: 't1', price: 199, origin: 399, students: 38612, rating: 4.9, lessons: 64, hours: 28,
    cover: 'linear-gradient(135deg, #5b8cff, #3d4ff0)', mark: 'Py', tags: ['零基础', '项目实战'], hot: true,
    desc: '从语法基础到 pandas、可视化与自动化报表，用 6 个真实业务项目带你完成一次完整的数据分析。',
    chapters: chapters([
      ['第一章 Python 基础语法', [['课程导学与环境搭建', 12], ['变量、类型与运算', 18], ['条件与循环', 22]]],
      ['第二章 函数与模块', [['函数定义与参数', 20], ['常用标准库', 25], ['异常处理', 16]]],
      ['第三章 pandas 数据处理', [['DataFrame 入门', 28], ['数据清洗实战', 32], ['分组聚合与透视表', 30]]],
      ['第四章 可视化与报表', [['Matplotlib 基础', 24], ['自动生成周报', 35]]]
    ]) },
  { id: 'k2', title: '30 天突破雅思口语 7 分', cat: '外语学习', teacher: 't2', price: 299, origin: 599, students: 21437, rating: 4.8, lessons: 42, hours: 18,
    cover: 'linear-gradient(135deg, #ff9fb6, #e8466f)', mark: 'EN', tags: ['考试冲刺', '外教点评'], hot: true,
    desc: '覆盖 Part 1-3 高频题库，拆解评分标准，配合每日跟读训练，帮你从 5.5 稳步提升到 7 分。',
    chapters: chapters([
      ['第一章 评分标准拆解', [['四项评分维度详解', 15], ['高分答案长什么样', 18]]],
      ['第二章 Part 1 高频话题', [['家乡与住所', 20], ['工作与学习', 20], ['兴趣爱好', 22]]],
      ['第三章 Part 2 话题卡', [['人物类万能思路', 26], ['事件类答题框架', 24]]]
    ]) },
  { id: 'k3', title: '职场高效沟通与汇报技巧', cat: '职场提升', teacher: 't1', price: 99, origin: 199, students: 52890, rating: 4.7, lessons: 24, hours: 8,
    cover: 'linear-gradient(135deg, #5fd3b4, #1f9e7a)', mark: '职', tags: ['新人必修'], hot: false,
    desc: '向上汇报、跨部门协作、会议主持，20 个职场场景的沟通模板拿来即用。',
    chapters: chapters([
      ['第一章 结构化表达', [['金字塔原理入门', 18], ['一句话说清结论', 15]]],
      ['第二章 向上汇报', [['周报怎么写', 20], ['汇报中的数据呈现', 22]]]
    ]) },
  { id: 'k4', title: '基金定投与家庭资产配置', cat: '理财投资', teacher: 't3', price: 149, origin: 299, students: 17305, rating: 4.8, lessons: 30, hours: 12,
    cover: 'linear-gradient(135deg, #ffcf70, #ff8a3d)', mark: '¥', tags: ['理财入门'], hot: true,
    desc: '搞懂指数基金、债券基金的底层逻辑，建立适合自己的定投计划和家庭资产配置方案。',
    chapters: chapters([
      ['第一章 理财第一课', [['认识风险与收益', 16], ['家庭财务体检', 20]]],
      ['第二章 基金投资', [['指数基金怎么选', 26], ['定投的止盈策略', 24]]]
    ]) },
  { id: 'k5', title: 'Figma 界面设计零基础实战', cat: '设计创作', teacher: 't4', price: 179, origin: 359, students: 12076, rating: 4.9, lessons: 36, hours: 15,
    cover: 'linear-gradient(135deg, #b39bff, #6b4ff0)', mark: 'UI', tags: ['作品集', '零基础'], hot: false,
    desc: '从组件、自动布局到设计系统，完成一套完整的 App 界面作品集。',
    chapters: chapters([
      ['第一章 Figma 基础操作', [['界面与工具栏', 14], ['图层与编组', 16]]],
      ['第二章 组件与自动布局', [['组件与变体', 25], ['Auto Layout 实战', 28]]]
    ]) },
  { id: 'k6', title: '公务员行测 5 大模块系统课', cat: '考研考公', teacher: 't5', price: 399, origin: 799, students: 46218, rating: 4.9, lessons: 80, hours: 40,
    cover: 'linear-gradient(135deg, #7d9cff, #4a5fd8)', mark: '考', tags: ['省考', '国考'], hot: true,
    desc: '言语理解、数量关系、判断推理、资料分析、常识判断全覆盖，配套 2000 道真题精讲。',
    chapters: chapters([
      ['第一章 言语理解', [['逻辑填空技巧', 30], ['片段阅读主旨题', 28]]],
      ['第二章 判断推理', [['图形推理五步法', 35], ['定义判断', 26]]],
      ['第三章 资料分析', [['速算技巧', 32], ['增长率专题', 30]]]
    ]) },
  { id: 'k7', title: '手机摄影：拍出朋友圈大片', cat: '兴趣生活', teacher: 't4', price: 0, origin: 69, students: 88341, rating: 4.7, lessons: 16, hours: 5,
    cover: 'linear-gradient(135deg, #ffa98a, #f0607a)', mark: '摄', tags: ['限时免费'], hot: false,
    desc: '构图、光线、后期调色三板斧，只用手机也能拍出有质感的照片。',
    chapters: chapters([
      ['第一章 构图', [['九宫格与引导线', 12], ['前景与留白', 14]]],
      ['第二章 后期', [['醒图调色思路', 18]]]
    ]) }
];

const reviews = [
  { name: '小鹿同学', score: 5, time: '2024-10-21', text: '老师讲得特别清楚，项目案例都是工作里能直接用的，第三章的数据清洗让我少加了好几天班。', color: '#7aa2ff' },
  { name: 'Kevin', score: 5, time: '2024-10-18', text: '节奏刚好，每节课 20 分钟左右，通勤路上就能学完一节。', color: '#ffa86b' },
  { name: '晴天', score: 4, time: '2024-10-12', text: '内容很扎实，就是希望课后练习能再多一些。', color: '#5fcfa8' }
];

const questions = [
  { q: '在 Python 中，以下哪个表达式的结果是 list 类型？', opts: ['(1, 2, 3)', '[x for x in range(3)]', '{1, 2, 3}', "'1,2,3'.strip()"], ans: 1,
    exp: '方括号形式的推导式生成列表；圆括号是元组，花括号是集合，strip() 返回字符串。' },
  { q: 'pandas 中按列分组并求平均值，正确的写法是？', opts: ["df.group('city').avg()", "df.groupby('city').mean()", "df.sort('city').mean()", "df.pivot('city')"], ans: 1,
    exp: 'groupby() 负责分组，mean() 对每组求均值。DataFrame 没有 group / avg 方法。' },
  { q: '下列关于函数默认参数的说法，正确的是？', opts: ['默认参数必须放在最前面', '默认参数只能是数字', '可变对象作默认值会在多次调用间共享', '默认参数不能被覆盖'], ans: 2,
    exp: '默认值只在函数定义时求值一次，list、dict 等可变对象会被多次调用共享，通常改用 None。' },
  { q: '执行 len({"a": 1, "b": 2, "a": 3}) 的结果是？', opts: ['1', '2', '3', '报错'], ans: 1,
    exp: '字典的键唯一，重复的 "a" 会覆盖前值，最终只有 a、b 两个键。' },
  { q: '读取 CSV 文件并跳过第一行表头以外的空行，最常用的函数是？', opts: ['pd.read_csv()', 'pd.load()', 'open().csv()', 'np.read()'], ans: 0,
    exp: 'pd.read_csv() 默认会把第一行作为表头，并通过 skip_blank_lines=True 跳过空行。' }
];

function teacherOf(id) { return teachers.find(t => t.id === id); }
function courseOf(id) { return courses.find(c => c.id === id); }
function fmtStudents(n) { return n >= 10000 ? (n / 10000).toFixed(1) + '万' : '' + n; }

module.exports = { categories, teachers, courses, reviews, questions, teacherOf, courseOf, fmtStudents };
