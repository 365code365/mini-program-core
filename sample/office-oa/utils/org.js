// 组织架构与员工（虚构公司「云协同科技（杭州）有限公司」）。py 是姓名全拼，用于 A-Z 分组与拼音搜索。
const company = { name: '云协同科技', full: '云协同科技（杭州）有限公司' };

const depts = [
  { id: 'ceo', name: '总经办' }, { id: 'rd', name: '产品研发部' }, { id: 'mkt', name: '市场部' },
  { id: 'sales', name: '销售部' }, { id: 'fin', name: '财务部' }, { id: 'hr', name: '人力行政部' }
];

const COLORS = ['#1677ff', '#13a8a8', '#fa8c16', '#722ed1', '#eb2f96', '#52c41a', '#2f54eb', '#d4380d'];

const raw = [
  ['e001', '林晓舟', 'linxiaozhou', 'rd', '高级产品经理', '13957120381', '8021'],
  ['e002', '陈思远', 'chensiyuan', 'rd', '研发总监', '13805716622', '8001'],
  ['e003', '安若溪', 'anruoxi', 'mkt', '品牌策划', '13666651230', '8302'],
  ['e004', '白嘉树', 'baijiashu', 'rd', '前端工程师', '13588402719', '8033'],
  ['e005', '蔡文静', 'caiwenjing', 'fin', '财务经理', '13757183306', '8501'],
  ['e006', '邓子航', 'dengzihang', 'sales', '大客户经理', '15067120948', '8203'],
  ['e007', '方雨桐', 'fangyutong', 'hr', '招聘主管', '13989462215', '8602'],
  ['e008', '高明轩', 'gaomingxuan', 'rd', '后端工程师', '18657130072', '8035'],
  ['e009', '韩冰洁', 'hanbingjie', 'fin', '出纳', '13626510839', '8503'],
  ['e010', '黄志强', 'huangzhiqiang', 'ceo', '总经理', '13906512288', '8000'],
  ['e011', '金佳怡', 'jinjiayi', 'mkt', '新媒体运营', '15158109367', '8305'],
  ['e012', '柯宇辰', 'keyuchen', 'rd', '测试工程师', '13777409152', '8038'],
  ['e013', '李沐阳', 'limuyang', 'sales', '销售总监', '13858001276', '8200'],
  ['e014', '刘欣妍', 'liuxinyan', 'hr', '人力行政经理', '13675817740', '8600'],
  ['e015', '马骁', 'maxiao', 'rd', 'UI 设计师', '18758230619', '8040'],
  ['e016', '倪诗涵', 'nishihan', 'mkt', '市场总监', '13819420053', '8300'],
  ['e017', '欧阳辰', 'ouyangchen', 'sales', '渠道经理', '15990113384', '8206'],
  ['e018', '潘一鸣', 'panyiming', 'rd', '架构师', '13567180925', '8030'],
  ['e019', '钱慧敏', 'qianhuimin', 'fin', '会计', '13735870461', '8502'],
  ['e020', '任浩然', 'renhaoran', 'sales', '销售代表', '18268039517', '8210'],
  ['e021', '沈梦琪', 'shenmengqi', 'hr', '行政专员', '13858197604', '8605'],
  ['e022', '唐俊杰', 'tangjunjie', 'rd', '运维工程师', '15869102748', '8036'],
  ['e023', '王雅琳', 'wangyalin', 'ceo', '总经理助理', '13906518830', '8002'],
  ['e024', '吴昊天', 'wuhaotian', 'sales', '销售代表', '13777582096', '8211'],
  ['e025', '谢晓萌', 'xiexiaomeng', 'mkt', '活动策划', '15157103369', '8306'],
  ['e026', '杨帆', 'yangfan', 'rd', '移动端工程师', '13819570214', '8034'],
  ['e027', '叶知秋', 'yezhiqiu', 'fin', '税务专员', '18667031852', '8505'],
  ['e028', '张一诺', 'zhangyinuo', 'rd', '产品经理', '13588217450', '8022'],
  ['e029', '赵敏', 'zhaomin', 'fin', '财务总监', '13906516671', '8500'],
  ['e030', '周景行', 'zhoujingxing', 'hr', '薪酬专员', '13646817293', '8606']
];

const employees = raw.map((r, i) => {
  const dept = depts.find(d => d.id === r[3]);
  return {
    id: r[0], name: r[1], py: r[2], letter: r[2][0].toUpperCase(), dept: r[3], deptName: dept.name,
    title: r[4], phone: r[5], ext: r[6], email: r[2] + '@yunxietong.cn',
    ini: r[1].length > 2 ? r[1].slice(-2) : r[1].slice(-1), color: COLORS[i % COLORS.length]
  };
});

const ME = 'e001';
function emp(id) { return employees.find(e => e.id === id); }
function me() { return emp(ME); }

// 审批流：直属主管 → 第二节点（请假：人力；报销：财务；用印：总经办）
const FLOW = { leave: ['e002', 'e014'], expense: ['e002', 'e029'], seal: ['e002', 'e023'] };

module.exports = { company, depts, employees, ME, emp, me, FLOW };
