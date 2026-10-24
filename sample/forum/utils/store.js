// 兴趣圈的数据层：帖子 / 点赞 / 收藏 / 评论 / 草稿都落 storage，种子数据由固定规则生成（离线、可复现）
const COLORS = ['#FF6B3D', '#3491FA', '#00B42A', '#722ED1', '#F7BA1E', '#14C9C9', '#EB2F96', '#F53F3F'];
const CIRCLES = [
  { id: 'photo', name: '摄影', desc: '用镜头记录日常', members: 128403, color: '#3491FA' },
  { id: 'run', name: '跑步', desc: '每一公里都算数', members: 86211, color: '#00B42A' },
  { id: 'cook', name: '厨房', desc: '家常菜也能很好吃', members: 203117, color: '#FF7D00' },
  { id: 'code', name: '程序员', desc: '写代码，也写生活', members: 156902, color: '#722ED1' },
  { id: 'book', name: '读书', desc: '一本好书，一段人生', members: 97350, color: '#14C9C9' },
  { id: 'pet', name: '萌宠', desc: '猫猫狗狗治愈一切', members: 241886, color: '#EB2F96' },
  { id: 'travel', name: '旅行', desc: '世界那么大', members: 175420, color: '#F7BA1E' }
];
const TAGS = ['新手求助', '经验分享', '晒图', '好物推荐', '避坑', '打卡', '讨论', '干货'];
const USERS = ['林小满', '周末去哪', '阿杰在跑步', '番茄炒蛋', '代码搬运工', '书虫小鹿', '橘猫铲屎官', '背包客老王', '七月的风', '咖啡续命', '山间明月', '晚风'];
const ME = { name: '沈夜', color: '#FF6B3D' };

const TITLES = {
  photo: ['傍晚的西湖，随手一拍', '新手入门第一台相机怎么选？', '胶片味调色参数分享', '雨天街拍的几个小技巧', '50mm 定焦真的是万金油吗'],
  run: ['第一次跑完半马，记录一下', '跑步膝盖疼怎么办', '夏天晨跑还是夜跑？', '入手了新跑鞋，脚感分享', '坚持跑步 100 天的变化'],
  cook: ['十分钟搞定的番茄牛腩', '空气炸锅鸡翅不翻车配方', '新手做红烧肉总是太腻？', '周末烤了一炉戚风', '一人食：酸汤肥牛'],
  code: ['Rust 学到所有权就劝退了', '远程办公两年的真实感受', '分享一个提效的 Git 别名', '面试被问到浏览器渲染流程', '小程序性能优化踩坑合集'],
  book: ['今年读过最好的一本小说', '每天 30 分钟阅读打卡', '《人类简史》读后感', '有没有适合通勤听的书', '纸质书还是电子书？'],
  pet: ['家里猫学会开门了怎么办', '领养了一只小橘', '狗狗挑食有什么办法', '猫咪第一次洗澡全记录', '晒晒我家的傻狗'],
  travel: ['大理五天四晚攻略', '一个人去西藏是什么体验', '周末周边游推荐：莫干山', '带爸妈出游的注意事项', '最近去的小众海岛']
};
const BODIES = [
  '分享一下最近的一点心得，欢迎大家一起交流。刚开始的时候踩了不少坑，后来慢慢摸索出了一套适合自己的方法。',
  '记录一下，也给有同样问题的朋友参考。整体体验比预期好很多，细节上还有提升空间，有经验的大佬欢迎指点。',
  '纠结了很久终于下定决心，过程比想象中顺利。最重要的是坚持，哪怕每天只进步一点点。',
  '先说结论：值得。具体原因写在下面，篇幅有点长，感兴趣的可以慢慢看，有问题评论区见。'
];
const MIN = 60000;

function seedPosts() {
  const now = Date.now();
  const list = [];
  let k = 0;
  for (let r = 0; r < 5; r++) {
    CIRCLES.forEach((c, ci) => {
      const u = USERS[(k * 5 + ci) % USERS.length];
      list.push({
        id: 'p' + (100 + k), circle: c.id, author: u, color: COLORS[k % COLORS.length], mine: false,
        title: TITLES[c.id][r], body: BODIES[(k + r) % BODIES.length],
        tags: [TAGS[k % TAGS.length], TAGS[(k + 3) % TAGS.length]].slice(0, 1 + (k % 2)),
        pics: (k % 3 === 0) ? 3 : (k % 3 === 1 ? 1 : 0),
        likes: 12 + (k * 37) % 480, comments: 0, time: now - (k * 47 + 6) * MIN
      });
      k++;
    });
  }
  list.sort((a, b) => b.time - a.time);
  const mineTitles = [['code', '记录一次线上内存泄漏排查', '起因是服务每隔两天就 OOM 一次，最后发现是一个缓存没有设置上限。排查过程写一下，希望对大家有帮助。', ['经验分享', '干货']],
    ['run', '杭州适合夜跑的几条路线', '整理了几条我常跑的路线：西湖环湖、钱塘江边、运河沿线，路灯充足、人不算多，适合下班后跑。', ['好物推荐']]];
  mineTitles.forEach((m, i) => list.unshift({
    id: 'my' + i, circle: m[0], author: ME.name, color: ME.color, mine: true, title: m[1], body: m[2], tags: m[3],
    pics: i === 0 ? 0 : 1, likes: 36 + i * 21, comments: 0, time: now - (300 + i * 900) * MIN
  }));
  list.sort((a, b) => b.time - a.time);
  return list;
}

function ensure() {
  if (wx.getStorageSync('forum_seeded')) return;
  wx.setStorageSync('forum_posts', seedPosts());
  wx.setStorageSync('forum_likes', ['p103', 'p108']);
  wx.setStorageSync('forum_favs', ['p101']);
  wx.setStorageSync('forum_joined', ['photo', 'code', 'run']);
  wx.setStorageSync('forum_seeded', 1);
}
function posts() { ensure(); return wx.getStorageSync('forum_posts') || []; }
function savePosts(l) { wx.setStorageSync('forum_posts', l); }
function likes() { ensure(); return wx.getStorageSync('forum_likes') || []; }
function favs() { ensure(); return wx.getStorageSync('forum_favs') || []; }
function joined() { ensure(); return wx.getStorageSync('forum_joined') || []; }
function circle(id) { return CIRCLES.find((c) => c.id === id) || CIRCLES[0]; }

// 评论：首次打开时按帖子 id 生成几层楼，之后读写 storage
function comments(pid) {
  const key = 'forum_cmt_' + pid;
  const saved = wx.getStorageSync(key);
  if (saved) return saved;
  const n = parseInt(pid.replace(/\D/g, ''), 10) || 0;
  const now = Date.now();
  const lines = ['说得太对了，收藏了', '同问，蹲一个答案', '我也遇到过，后来换了个方法就好了', '楼主好厉害！', '请问具体是怎么做的？', '学到了，感谢分享', '这个我有不同看法', '已经照着试了，确实有用'];
  const floors = [];
  const count = 3 + (n % 3);
  for (let i = 0; i < count; i++) {
    const u = USERS[(n + i * 3) % USERS.length];
    const f = { id: 'f' + i, floor: i + 1, author: u, color: COLORS[(n + i) % COLORS.length], text: lines[(n + i) % lines.length], time: now - (count - i) * 37 * MIN, likes: (n * 7 + i * 13) % 60, replies: [] };
    if (i % 2 === 0) {
      const r1 = USERS[(n + i * 3 + 1) % USERS.length];
      f.replies.push({ id: 'r' + i + '_0', author: r1, to: '', text: '+1，我也想知道', time: f.time + 5 * MIN });
      if (i === 0) f.replies.push({ id: 'r0_1', author: u, to: r1, text: '我私信你吧', time: f.time + 9 * MIN });
    }
    floors.push(f);
  }
  wx.setStorageSync(key, floors);
  return floors;
}
function saveComments(pid, floors) {
  wx.setStorageSync('forum_cmt_' + pid, floors);
  const total = floors.reduce((a, f) => a + 1 + f.replies.length, 0);
  const all = posts(); const p = all.find((x) => x.id === pid);
  if (p) { p.comments = total; savePosts(all); }
  return total;
}
function commentCount(p) {
  if (p.comments) return p.comments;
  const n = parseInt(p.id.replace(/\D/g, ''), 10) || 0;
  const count = 3 + (n % 3);
  let total = count;
  for (let i = 0; i < count; i += 2) total += i === 0 ? 2 : 1;
  return total;
}

function ago(ts) {
  const d = Date.now() - ts;
  if (d < MIN) return '刚刚';
  if (d < 60 * MIN) return Math.floor(d / MIN) + ' 分钟前';
  if (d < 24 * 60 * MIN) return Math.floor(d / 60 / MIN) + ' 小时前';
  if (d < 7 * 24 * 60 * MIN) return Math.floor(d / 24 / 60 / MIN) + ' 天前';
  const t = new Date(ts); return (t.getMonth() + 1) + '-' + t.getDate();
}
function tint(hex, a) {
  const v = parseInt(hex.slice(1), 16);
  return 'rgba(' + (v >> 16) + ',' + ((v >> 8) & 255) + ',' + (v & 255) + ',' + a + ')';
}
function fmtNum(n) { return n >= 10000 ? (n / 10000).toFixed(1) + '万' : '' + n; }

// 卡片视图：把点赞 / 收藏状态、圈子名、相对时间等算好给 WXML（WXML 里不调方法）
function card(p, likeSet, favSet) {
  const c = circle(p.circle);
  const liked = likeSet.indexOf(p.id) >= 0;
  return {
    id: p.id, title: p.title, body: p.body, author: p.author, initial: p.author.slice(0, 1), color: p.color, mine: p.mine,
    circleName: c.name, circleColor: c.color, circleBg: tint(c.color, 0.1), picFrom: tint(p.color, 0.35), tags: p.tags, pics: [0, 1, 2].slice(0, p.pics).map((i) => { const col = COLORS[(i + p.title.length) % COLORS.length]; return { k: i, color: col, from: tint(col, 0.35) }; }),
    summary: p.body.length > 46 ? p.body.slice(0, 46) + '…' : p.body,
    picCls: p.pics === 1 ? 'pic-one' : 'pic-three',
    liked, likes: p.likes + (liked ? 1 : 0), fav: favSet.indexOf(p.id) >= 0, comments: commentCount(p), time: ago(p.time), edited: !!p.edited
  };
}

module.exports = { tint, COLORS, CIRCLES, TAGS, USERS, ME, ensure, posts, savePosts, likes, favs, joined, circle, comments, saveComments, ago, fmtNum, card, seedPosts };
