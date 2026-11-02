// 云川大学（虚构）：本学期课表、成绩、通知、图书、空教室。学期从 2026-09-07（周一）开始，共 18 周
const kit = require('./kit.js');
const TERM_START = new Date(2026, 8, 7);
const WEEKS = 18;
const COLORS = ['#5b8def', '#14937f', '#f08a4b', '#a46be0', '#e05a7a', '#3aa6c8', '#c49a1a', '#6b8e23'];
const COURSES = [
  { id: 'c1', name: '高等数学 A(1)', teacher: '周明远', room: '博学楼 A201', day: 1, start: 1, len: 2, weeks: [1, 16], credit: 5, type: '必修' },
  { id: 'c2', name: '大学英语(3)', teacher: 'Emily Chen', room: '外语楼 305', day: 1, start: 5, len: 2, weeks: [1, 16], credit: 2, type: '必修' },
  { id: 'c3', name: '数据结构', teacher: '李维', room: '信息楼 B402', day: 2, start: 3, len: 2, weeks: [1, 18], credit: 4, type: '专业必修' },
  { id: 'c4', name: '大学物理 B', teacher: '王建国', room: '理学楼 108', day: 2, start: 7, len: 2, weeks: [1, 16], credit: 3, type: '必修' },
  { id: 'c5', name: '线性代数', teacher: '孙晓燕', room: '博学楼 A305', day: 3, start: 1, len: 2, weeks: [1, 12], credit: 3, type: '必修' },
  { id: 'c6', name: '程序设计实践', teacher: '赵磊', room: '信息楼 机房 3', day: 3, start: 5, len: 3, weeks: [3, 16], credit: 2, type: '实践' },
  { id: 'c7', name: '中国近现代史纲要', teacher: '陈立群', room: '文科楼 101', day: 4, start: 3, len: 2, weeks: [1, 16], credit: 3, type: '必修' },
  { id: 'c8', name: '数据结构实验', teacher: '李维', room: '信息楼 机房 1', day: 4, start: 7, len: 2, weeks: [2, 18], credit: 1, type: '实践' },
  { id: 'c9', name: '体育(3) · 羽毛球', teacher: '刘洋', room: '东区体育馆', day: 5, start: 3, len: 2, weeks: [1, 16], credit: 1, type: '必修' },
  { id: 'c10', name: '电影艺术鉴赏', teacher: '林静', room: '图书馆报告厅', day: 5, start: 10, len: 2, weeks: [1, 12], credit: 2, type: '通识选修' }
].map((c, i) => Object.assign(c, { color: COLORS[i % COLORS.length] }));
const SECTIONS = ['08:00', '08:55', '10:00', '10:55', '14:00', '14:55', '16:00', '16:55', '19:00', '19:55', '20:50', '21:45'];

function weekOf(date) {
  const d = date || new Date();
  const w = Math.floor((d - TERM_START) / (7 * 86400000)) + 1;
  return Math.min(WEEKS, Math.max(1, w));
}
function inWeek(c, w) { return w >= c.weeks[0] && w <= c.weeks[1]; }
function coursesOn(day, w) { return COURSES.filter(c => c.day === day && inWeek(c, w)).sort((a, b) => a.start - b.start); }
// 第 w 周 7 天的日期（周一开始）
function datesOf(w) {
  const base = new Date(TERM_START.getTime() + (w - 1) * 7 * 86400000);
  return [0, 1, 2, 3, 4, 5, 6].map(i => {
    const d = new Date(base.getTime() + i * 86400000);
    const ymd = kit.fmt(d, 'YYYY-MM-DD');
    return { md: (d.getMonth() + 1) + '/' + d.getDate(), ymd, wd: WD[i + 1], today: ymd === kit.dateStr(0) };
  });
}

const NOTICES = [
  { id: 'n1', type: '教务', title: '关于 2026-2027 学年第一学期期中教学检查的通知', dept: '教务处', date: '2026-10-28', top: true,
    body: '各学院：\n根据学校教学工作安排，定于第 9-10 周开展期中教学检查。检查内容包括课堂教学秩序、教学进度、作业批改与学生评教。\n请各学院于 11 月 6 日前提交自查报告。' },
  { id: 'n2', type: '考试', title: '全国大学英语四六级考试（2026 年 12 月）报名通知', dept: '教务处', date: '2026-10-26',
    body: '报名时间：11 月 3 日 9:00 至 11 月 12 日 17:00。\n报名方式：登录全国大学英语四六级考试报名网站完成报名并缴费。\n笔试时间：12 月 19 日。' },
  { id: 'n3', type: '图书馆', title: '图书馆延长开放时间至 23:00（考试周）', dept: '图书馆', date: '2026-10-24',
    body: '为方便同学们复习备考，第 17-18 周图书馆一至四层自习区开放时间延长至 23:00，请保持安静、文明占座。' },
  { id: 'n4', type: '后勤', title: '东区学生公寓 11 月 5 日停水检修通知', dept: '后勤服务中心', date: '2026-10-22',
    body: '因供水管网改造，11 月 5 日 9:00-17:00 东区 1-6 号公寓停水，请提前储水。如遇紧急情况请拨打后勤热线 0838-6612110。' },
  { id: 'n5', type: '活动', title: '第十二届「云川杯」程序设计竞赛报名开始', dept: '计算机学院', date: '2026-10-20',
    body: '比赛采用 ACM-ICPC 赛制，三人一队。报名截止 11 月 15 日，初赛 11 月 22 日在信息楼机房举行。获奖队伍可获创新学分。' },
  { id: 'n6', type: '资助', title: '2026 年度国家励志奖学金评审结果公示', dept: '学生工作部', date: '2026-10-18',
    body: '经学生申请、学院评审、学校审核，拟推荐 312 名同学获得国家励志奖学金，公示期 5 个工作日，如有异议请联系学生工作部。' }
];

const GRADES = {
  '2025-2026|1': [
    { name: '高等数学预备', credit: 2, score: 92, type: '必修' }, { name: '大学英语(1)', credit: 2, score: 85, type: '必修' },
    { name: '程序设计基础', credit: 4, score: 95, type: '专业必修' }, { name: '思想道德与法治', credit: 3, score: 88, type: '必修' },
    { name: '体育(1)', credit: 1, score: 79, type: '必修' }, { name: '计算机导论', credit: 2, score: 90, type: '专业必修' }
  ],
  '2025-2026|2': [
    { name: '离散数学', credit: 3, score: 83, type: '专业必修' }, { name: '大学英语(2)', credit: 2, score: 81, type: '必修' },
    { name: 'C++ 面向对象程序设计', credit: 3, score: 91, type: '专业必修' }, { name: '概率论与数理统计', credit: 3, score: 68, type: '必修' },
    { name: '体育(2)', credit: 1, score: 86, type: '必修' }, { name: '摄影基础', credit: 2, score: 94, type: '通识选修' }
  ],
  '2026-2027|1': [
    { name: '数据结构（期中）', credit: 4, score: 87, type: '专业必修' }, { name: '线性代数（期中）', credit: 3, score: 76, type: '必修' }
  ]
};
function gpaOf(score) { return score >= 90 ? 4.0 : score >= 85 ? 3.7 : score >= 82 ? 3.3 : score >= 78 ? 3.0 : score >= 75 ? 2.7 : score >= 72 ? 2.3 : score >= 68 ? 2.0 : score >= 64 ? 1.5 : score >= 60 ? 1.0 : 0; }

const BOOKS = [
  { id: 'b1', title: '算法导论（第 3 版）', author: 'Thomas H. Cormen 等', call: 'TP301.6/C81', loc: '三楼 自然科学书库', total: 6, left: 2, cover: '/images/books/b1.jpg' },
  { id: 'b2', title: '深入理解计算机系统', author: 'Randal E. Bryant', call: 'TP303/B91', loc: '三楼 自然科学书库', total: 5, left: 0, cover: '/images/books/b2.jpg' },
  { id: 'b3', title: '活着', author: '余华', call: 'I247.57/Y79', loc: '二楼 文学书库', total: 8, left: 5, cover: '/images/books/b3.jpg' },
  { id: 'b4', title: '人类简史', author: '尤瓦尔·赫拉利', call: 'K02/H32', loc: '二楼 社科书库', total: 4, left: 1, cover: '/images/books/b4.jpg' },
  { id: 'b5', title: '线性代数应该这样学', author: 'Sheldon Axler', call: 'O151.2/A97', loc: '三楼 自然科学书库', total: 3, left: 3, cover: '/images/books/b5.jpg' },
  { id: 'b6', title: '三体（全三册）', author: '刘慈欣', call: 'I247.55/L71', loc: '二楼 文学书库', total: 10, left: 0, cover: '/images/books/b6.jpg' },
  { id: 'b7', title: '经济学原理（微观）', author: 'N. Gregory Mankiw', call: 'F0/M26', loc: '二楼 社科书库', total: 6, left: 4, cover: '/images/books/b7.jpg' },
  { id: 'b8', title: 'Python 编程：从入门到实践', author: 'Eric Matthes', call: 'TP311.56/M43', loc: '三楼 自然科学书库', total: 7, left: 2, cover: '/images/books/b8.jpg' }
];
// 与 utils/fmt.wxs 同一套规则的 JS 版本：本引擎在带自定义 tabBar 的小程序里 WXS 调用返回空，模板里用 `wxs() || 字段` 兜底
function levelOf(s) { return s >= 90 ? '优秀' : s >= 80 ? '良好' : s >= 70 ? '中等' : s >= 60 ? '及格' : '不及格'; }
function levelClassOf(s) { return s >= 90 ? 'lv-a' : s >= 80 ? 'lv-b' : s >= 60 ? 'lv-c' : 'lv-d'; }
const WD = ['', '周一', '周二', '周三', '周四', '周五', '周六', '周日'];
function sectionText(start, len) { return '第' + start + '-' + (start + len - 1) + '节 ' + SECTIONS[start - 1]; }
const BUILDINGS = ['博学楼', '信息楼', '理学楼', '文科楼', '外语楼'];
module.exports = { levelOf, levelClassOf, WD, sectionText, TERM_START, WEEKS, COURSES, SECTIONS, weekOf, inWeek, coursesOn, datesOf, NOTICES, GRADES, gpaOf, BOOKS, BUILDINGS };
