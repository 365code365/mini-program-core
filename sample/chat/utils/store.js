// 轻聊的数据层：联系人是固定数据，会话 / 消息 / 资料都落在 storage 里（重进小程序保持状态）
const COLORS = ['#4F6BFF', '#00B42A', '#FF7D00', '#F53F3F', '#722ED1', '#14C9C9', '#EB2F96', '#3491FA'];

const CONTACTS = [
  { id: 'c01', name: '安然', py: 'A', wxid: 'anran_1994', region: '浙江 杭州', sign: '慢慢来，比较快。', remark: '大学室友', replies: ['哈哈好的', '我刚下班，晚点回你', '可以呀，周末见！'] },
  { id: 'c02', name: '白露', py: 'B', wxid: 'bailu0921', region: '上海 徐汇', sign: '设计是解决问题的艺术', remark: '设计组', replies: ['稿子我改好发群里了', '这个配色再淡一点会更好', '收到～'] },
  { id: 'c03', name: '陈思远', py: 'C', wxid: 'chensy', region: '北京 海淀', sign: 'Talk is cheap.', remark: '后端', replies: ['接口已经部署到测试环境了', '我看下日志', '那个 bug 修了，你再试试'] },
  { id: 'c04', name: '程一诺', py: 'C', wxid: 'yinuo_cheng', region: '广东 深圳', sign: '跑步中，勿扰', remark: '', replies: ['今晚跑 5 公里吗？', '我在路上', '好嘞'] },
  { id: 'c05', name: '丁小雨', py: 'D', wxid: 'dingxy', region: '四川 成都', sign: '火锅是信仰', remark: '', replies: ['走起！吃火锅去', '我请客哈哈', '在排队，大概还要 20 分钟'] },
  { id: 'c06', name: '方宇航', py: 'F', wxid: 'fyh_pilot', region: '湖北 武汉', sign: '心有山海，静而不争', remark: '产品经理', replies: ['需求文档我更新了', '下午三点开评审会', '这个优先级先放一放'] },
  { id: 'c07', name: '高晴', py: 'G', wxid: 'gaoqing', region: '江苏 南京', sign: '读书 · 旅行 · 摄影', remark: '', replies: ['照片我发你邮箱了', '下个月去大理！', '好呀'] },
  { id: 'c08', name: '韩梅梅', py: 'H', wxid: 'hanmm', region: '浙江 宁波', sign: 'Nice to meet you.', remark: '英语角', replies: ['Sounds great!', '明早 8 点图书馆见', '没问题'] },
  { id: 'c09', name: '何嘉明', py: 'H', wxid: 'hejiaming', region: '广东 广州', sign: '早茶爱好者', remark: '', replies: ['虾饺必点', '我订好位了', '收到收到'] },
  { id: 'c10', name: '江南', py: 'J', wxid: 'jiangnan_s', region: '江苏 苏州', sign: '人间烟火气，最抚凡人心', remark: '', replies: ['在开会，等下说', '好的，没问题', '我看看日程'] },
  { id: 'c11', name: '李想', py: 'L', wxid: 'lixiang88', region: '浙江 杭州', sign: '想得美', remark: '前端', replies: ['页面我改好了，你刷新看看', '这个动画有点卡，我再优化下', '已提交'] },
  { id: 'c12', name: '林晓', py: 'L', wxid: 'linxiao', region: '福建 厦门', sign: '海风很温柔', remark: '', replies: ['今天的晚霞超美', '下次来厦门找我', '嗯嗯'] },
  { id: 'c13', name: '刘洋', py: 'L', wxid: 'liuyang_x', region: '山东 青岛', sign: '啤酒配海鲜', remark: '高中同学', replies: ['同学聚会定在 12 号', '你来吗？', '哈哈哈哈'] },
  { id: 'c14', name: '孟凡', py: 'M', wxid: 'mengfan', region: '河南 郑州', sign: '平凡之路', remark: '', replies: ['好的', '我知道了', '晚点打给你'] },
  { id: 'c15', name: '彭佳', py: 'P', wxid: 'pengjia', region: '湖南 长沙', sign: '辣椒是灵魂', remark: '', replies: ['茶颜悦色走一个', '我在五一广场', '马上到'] },
  { id: 'c16', name: '宋词', py: 'S', wxid: 'songci', region: '陕西 西安', sign: '一蓑烟雨任平生', remark: '', replies: ['明月几时有', '在看书', '好'] },
  { id: 'c17', name: '孙悦', py: 'S', wxid: 'sunyue', region: '天津 和平', sign: '每天开心一点点', remark: 'HR', replies: ['offer 已经发你邮箱了', '入职材料记得带齐', '有问题随时找我'] },
  { id: 'c18', name: '唐果', py: 'T', wxid: 'tangguo', region: '重庆 渝中', sign: '甜度刚好', remark: '', replies: ['奶茶要三分糖', '我到楼下了', '冲鸭'] },
  { id: 'c19', name: '王磊', py: 'W', wxid: 'wanglei_dev', region: '浙江 杭州', sign: '代码写诗', remark: '组长', replies: ['周报记得交', '这个方案可以，按这个推进', '辛苦了'] },
  { id: 'c20', name: '吴桐', py: 'W', wxid: 'wutong', region: '安徽 合肥', sign: '梧桐更兼细雨', remark: '', replies: ['下雨了记得带伞', '嗯', '晚安'] },
  { id: 'c21', name: '许诺', py: 'X', wxid: 'xunuo', region: '浙江 温州', sign: '说到做到', remark: '', replies: ['答应你的事一定办到', '放心', '好的'] },
  { id: 'c22', name: '杨帆', py: 'Y', wxid: 'yangfan', region: '辽宁 大连', sign: '乘风破浪', remark: '', replies: ['船票买好了', '周五出发', '冲'] },
  { id: 'c23', name: '叶知秋', py: 'Y', wxid: 'yezhiqiu', region: '云南 昆明', sign: '一叶知秋', remark: '', replies: ['昆明今天 22 度', '来玩呀', '哈哈'] },
  { id: 'c24', name: '张伟', py: 'Z', wxid: 'zhangwei', region: '北京 朝阳', sign: '平平无奇', remark: '房东', replies: ['房租这周记得转', '热水器明天有人来修', '好的'] },
  { id: 'c25', name: '周舟', py: 'Z', wxid: 'zhouzhou', region: '浙江 舟山', sign: '一叶扁舟', remark: '', replies: ['在海边', '信号不太好', '回去聊'] }
];
CONTACTS.forEach((c, i) => { c.color = COLORS[i % COLORS.length]; c.initial = c.name.slice(0, 1); });

const MIN = 60 * 1000;
function seed() {
  const now = Date.now();
  const s = [
    { id: 'c19', pinned: true, unread: 2, ago: 3, msgs: [['them', '明天上午十点项目周会，记得准备进度'], ['me', '好的，我把演示环境也准备一下'], ['them', '这个方案可以，按这个推进'], ['them', '周报记得交']] },
    { id: 'c11', pinned: false, unread: 5, ago: 12, msgs: [['me', '首页那个列表滑起来有点卡'], ['them', '我看下是不是图片没压缩'], ['them', '确实，图片 2MB 一张'], ['them', '压完了'], ['them', '已提交'], ['them', '页面我改好了，你刷新看看']] },
    { id: 'c05', pinned: false, unread: 1, ago: 40, msgs: [['them', '周六晚上有空吗'], ['me', '有啊，什么安排'], ['them', '走起！吃火锅去']] },
    { id: 'c17', pinned: true, unread: 0, ago: 180, msgs: [['them', '您好，面试结果已出'], ['them', 'offer 已经发你邮箱了'], ['me', '收到，非常感谢！']] },
    { id: 'c02', pinned: false, unread: 0, ago: 60 * 26, msgs: [['me', '图标那版我觉得第二个更好'], ['them', '这个配色再淡一点会更好']] },
    { id: 'c24', pinned: false, unread: 0, ago: 60 * 50, msgs: [['them', '房租这周记得转'], ['me', '已转，您查收一下']] },
    { id: 'c13', pinned: false, unread: 12, ago: 60 * 75, msgs: [['them', '同学聚会定在 12 号'], ['them', '你来吗？']] },
    { id: 'c07', pinned: false, unread: 0, ago: 60 * 24 * 5, msgs: [['them', '照片我发你邮箱了'], ['me', '拍得太好看了吧']] }
  ];
  const sessions = [];
  s.forEach((it) => {
    const base = now - it.ago * MIN;
    const list = it.msgs.map((m, k) => ({ id: 'm' + base + '_' + k, from: m[0], text: m[1], time: base - (it.msgs.length - 1 - k) * 2 * MIN }));
    wx.setStorageSync('chat_msgs_' + it.id, list);
    const last = list[list.length - 1];
    sessions.push({ id: it.id, pinned: it.pinned, unread: it.unread, lastText: last.text, lastTime: last.time });
  });
  wx.setStorageSync('chat_sessions', sessions);
  wx.setStorageSync('chat_profile', { nickname: '沈夜', gender: 'male', birthday: '1996-08-12', signature: '把每一行代码都写成诗。', color: '#4F6BFF', city: '杭州' });
  wx.setStorageSync('chat_star', ['c01', 'c11']);
  wx.setStorageSync('chat_seeded', 1);
}

function ensure() { if (!wx.getStorageSync('chat_seeded')) seed(); }
function contact(id) { return CONTACTS.find((c) => c.id === id); }
function sessions() { ensure(); return wx.getStorageSync('chat_sessions') || []; }
function saveSessions(list) { wx.setStorageSync('chat_sessions', list); }
function messages(id) { return wx.getStorageSync('chat_msgs_' + id) || []; }
function saveMessages(id, list) { wx.setStorageSync('chat_msgs_' + id, list); }
function profile() { ensure(); return wx.getStorageSync('chat_profile'); }
function stars() { return wx.getStorageSync('chat_star') || []; }

// 会话里追加一条消息并同步会话摘要（不在聊天页时 them 的消息记未读）
function touchSession(id, msg, countUnread) {
  const list = sessions();
  let s = list.find((x) => x.id === id);
  if (!s) { s = { id, pinned: false, unread: 0, lastText: '', lastTime: 0 }; list.push(s); }
  s.lastText = msg.text; s.lastTime = msg.time;
  if (countUnread) s.unread += 1;
  saveSessions(list);
}

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function fmtTime(ts, now) {
  const d = new Date(ts); const n = new Date(now || Date.now());
  const day0 = new Date(n.getFullYear(), n.getMonth(), n.getDate()).getTime();
  const hm = pad(d.getHours()) + ':' + pad(d.getMinutes());
  if (ts >= day0) return hm;
  if (ts >= day0 - 86400000) return '昨天';
  if (ts >= day0 - 6 * 86400000) return '星期' + '日一二三四五六'[d.getDay()];
  return (d.getMonth() + 1) + '/' + d.getDate();
}
function fmtFull(ts, now) {
  const d = new Date(ts); const hm = pad(d.getHours()) + ':' + pad(d.getMinutes());
  const label = fmtTime(ts, now);
  return label === hm ? hm : label + ' ' + hm;
}
function unreadTotal() { return sessions().reduce((a, s) => a + s.unread, 0); }
function syncTabBadge(page, selected) {
  if (typeof page.getTabBar !== 'function' || !page.getTabBar()) return;
  const n = unreadTotal();
  page.getTabBar().setData({ selected, badge: n, badgeText: n > 99 ? '99+' : '' + n });
}

module.exports = { COLORS, CONTACTS, ensure, contact, sessions, saveSessions, messages, saveMessages, profile, stars, touchSession, fmtTime, fmtFull, unreadTotal, syncTabBadge, seed };
