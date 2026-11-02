// 静态数据：影片 / 影院 / 场次 / 优惠券 / 影评样例。场次按 影片 + 影院 + 第几天 算出来，每次进页面都一样
const MOVIES = [
  { id: 'm1', title: '长河落日', en: 'The Long River', score: 9.2, genre: '剧情 / 历史', duration: 148, release: '2026-09-30', region: '中国大陆', status: 'now', poster: '/images/m1.jpg', wish: 286000, box: '18.6亿',
    director: '陈墨', actors: [['张译', '饰 林守望'], ['周冬雨', '饰 苏晚'], ['黄渤', '饰 老秦'], ['王一博', '饰 林小川'], ['倪大红', '饰 船老大'], ['宋佳', '饰 秦嫂']],
    intro: '1949 年初春，黄河渡口的船工林守望接下一桩秘密任务：七天之内，把一船救命的药品送过封锁线。河面结着薄冰，对岸枪声不断，一家人的命运与一座城的明天，被系在这条摇晃的木船上。' },
  { id: 'm2', title: '山海之间', en: 'Between Mountains', score: 8.9, genre: '剧情 / 家庭', duration: 126, release: '2026-10-01', region: '中国大陆', status: 'now', poster: '/images/m2.jpg', wish: 152000, box: '7.2亿',
    director: '李湘云', actors: [['易烊千玺', '饰 阿山'], ['惠英红', '饰 阿嬷'], ['张子枫', '饰 海妹'], ['范伟', '饰 村长']],
    intro: '在城市打拼多年的阿山回到海边的老家，为阿嬷筹办八十岁寿宴。三天里，渔村的旧事一件件浮上来，他终于读懂了那个不肯离开大山与大海的老人。' },
  { id: 'm3', title: '星际回响', en: 'Echoes of Stars', score: 9.0, genre: '科幻 / 冒险', duration: 155, release: '2026-10-17', region: '中国大陆', status: 'now', poster: '/images/m3.jpg', wish: 398000, box: '12.3亿',
    director: '郭帆', actors: [['吴京', '饰 刘启明'], ['沈腾', '饰 王磊'], ['刘德华', '饰 图恒宇'], ['李雪健', '饰 周喆直'], ['朱颜曼滋', '饰 韩朵朵']],
    intro: '深空探测器在柯伊伯带收到一段来自人类自己的求救信号。为了弄清信号的来历，一支五人小队踏上了单程的远航，而他们要面对的，是三十年后的地球。' },
  { id: 'm4', title: '长安夜行', en: 'Night in Chang\'an', score: 8.6, genre: '悬疑 / 古装', duration: 132, release: '2026-10-10', region: '中国大陆', status: 'now', poster: '/images/m4.jpg', wish: 97000, box: '4.1亿',
    director: '曹盾', actors: [['雷佳音', '饰 张小敬'], ['易烊千玺', '饰 李必'], ['周一围', '饰 龙波'], ['热依扎', '饰 檀棋']],
    intro: '上元节前夜，长安城一百零八坊灯火通明。死囚张小敬被临时放出，要在十二个时辰内揪出潜入城中的刺客，否则整座长安将付之一炬。' },
  { id: 'm5', title: '深海营救', en: 'Deep Rescue', score: 0, genre: '动作 / 灾难', duration: 138, release: '2026-11-08', region: '中国大陆', status: 'soon', poster: '/images/m5.jpg', wish: 213000,
    director: '林超贤', actors: [['彭于晏', '饰 高远'], ['杜江', '饰 陆辰'], ['蓝盈莹', '饰 夏岚']],
    intro: '南海一座钻井平台在台风中倾覆，三十七名工人被困海底舱室。救援队只有七十二小时，深海的黑暗和不断下降的氧气，都在倒数。' },
  { id: 'm6', title: '外婆的菜园', en: 'Grandma\'s Garden', score: 0, genre: '剧情 / 温情', duration: 108, release: '2026-11-15', region: '中国大陆', status: 'soon', poster: '/images/m6.jpg', wish: 64000,
    director: '文晏', actors: [['吴彦姝', '饰 外婆'], ['王俊凯', '饰 小树']],
    intro: '十岁的小树被送到乡下外婆家过暑假。外婆的菜园里，有会说话的南瓜藤，也有妈妈小时候埋下的秘密。' },
  { id: 'm7', title: '无声证词', en: 'Silent Witness', score: 0, genre: '犯罪 / 悬疑', duration: 119, release: '2026-11-22', region: '中国香港', status: 'soon', poster: '/images/m7.jpg', wish: 88000,
    director: '陈茂贤', actors: [['张家辉', '饰 何律师'], ['杨紫', '饰 林家宁'], ['张子枫', '饰 小哑']],
    intro: '一桩豪门命案唯一的目击者，是一个不会说话的女孩。律师要在法庭上，让她“开口”作证。' },
  { id: 'm8', title: '熊猫奇遇记', en: 'Panda Adventure', score: 0, genre: '动画 / 喜剧', duration: 96, release: '2026-12-20', region: '中国大陆', status: 'soon', poster: '/images/m8.jpg', wish: 175000,
    director: '饺子', actors: [['吕艳婷', '配 团团'], ['瀚墨', '配 竹竹']],
    intro: '贪吃的熊猫团团为了找回被偷走的竹笋，第一次走出了卧龙山，闯进了热闹的成都城。' }
];
const CINEMAS = [
  { id: 'c1', name: '万达影城（五角场万达广场店）', addr: '杨浦区邯郸路600号万达广场5楼', dist: '1.2km', tags: ['IMAX', '杜比全景声', '退签'], base: 49, phone: '021-6551 8899', lat: 31.3017, lng: 121.5146 },
  { id: 'c2', name: 'CGV影城（大宁久光店）', addr: '静安区共和新路1898号5楼', dist: '3.6km', tags: ['4DX', 'SCREENX', '改签'], base: 45, phone: '021-6630 2288', lat: 31.2810, lng: 121.4580 },
  { id: 'c3', name: '上影国际影城（新天地店）', addr: '黄浦区马当路388号SOHO复兴广场', dist: '8.1km', tags: ['杜比影院', '情侣座'], base: 59, phone: '021-6386 6600', lat: 31.2152, lng: 121.4745 },
  { id: 'c4', name: '博纳国际影城（龙之梦店）', addr: '长宁区长宁路1018号龙之梦购物中心9楼', dist: '10.4km', tags: ['巨幕', '退签'], base: 42, phone: '021-5238 1018', lat: 31.2203, lng: 121.4166 }
];
const TIMES = [['10:20', '国语2D', '3号厅'], ['12:45', '国语IMAX', 'IMAX厅'], ['14:30', '国语2D', '5号厅'], ['16:55', '国语杜比', '杜比厅'], ['19:10', '国语IMAX', 'IMAX厅'], ['21:35', '国语2D', '2号厅']];
const DAY_NAMES = ['今天', '明天', '后天'];
const DAY = 86400000;
const STOP_MIN = 30; // 开场前 30 分钟停售
function pad(n) { return (n < 10 ? '0' : '') + n; }
function dayStart(offset) {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d.getTime() + offset * DAY;
}
function dayLabel(offset) {
  const d = new Date(dayStart(offset));
  return DAY_NAMES[offset] + ' ' + (d.getMonth() + 1) + '月' + d.getDate() + '日';
}
function days() {
  return DAY_NAMES.map((n, i) => ({ label: dayLabel(i) }));
}
// 场次：ts 是开场时刻；stop = 已过停售时间
function shows(movie, cinema, day) {
  const offs = (movie.id.charCodeAt(1) + cinema.id.charCodeAt(1) + day) % 3;
  const now = Date.now();
  return TIMES.filter((t, i) => (i + offs) % 4 !== 0).map(t => {
    const [h, m] = t[0].split(':').map(Number);
    const end = h * 60 + m + movie.duration;
    const extra = t[1].indexOf('IMAX') >= 0 ? 20 : t[1].indexOf('杜比') >= 0 ? 15 : 0;
    const ts = dayStart(day) + (h * 60 + m) * 60000;
    return { time: t[0], end: pad(Math.floor(end / 60) % 24) + ':' + pad(end % 60), lang: t[1], hall: t[2], price: cinema.base + extra, ts, day, stop: ts - now < STOP_MIN * 60000 };
  });
}
function getMovie(id) { return MOVIES.find(m => m.id === id) || MOVIES[0]; }
function getCinema(id) { return CINEMAS.find(c => c.id === id) || CINEMAS[0]; }
// 优惠券：min 是票价门槛
const COUPONS = [
  { id: 'q1', name: '新人立减券', off: 10, min: 0, exp: '2026-12-31', desc: '全部影片可用' },
  { id: 'q2', name: '周三半价日', off: 20, min: 60, exp: '2026-12-31', desc: '票价满 ¥60 可用' },
  { id: 'q3', name: '会员观影券', off: 5, min: 0, exp: '2026-11-30', desc: '光影会员专享' }
];
// 兑换码 → 券
const CODES = {
  GY2026: { id: 'q-gy2026', name: '影迷福利券', off: 15, min: 50, exp: '2026-12-31', desc: '票价满 ¥50 可用' },
  IMAX: { id: 'q-imax', name: 'IMAX 专享券', off: 12, min: 60, exp: '2026-12-31', desc: '票价满 ¥60 可用' }
};
const RECHARGE = [
  { amount: 50, gift: 5 },
  { amount: 100, gift: 15 },
  { amount: 200, gift: 40 }
];
const VIP_PRICE = 19;
const SERVICE_FEE = 3;
const SNACK_PRICE = 39;
const REFUND_FEE = 2;
const STAR_TEXT = ['', '很差', '较差', '还行', '推荐', '神作'];
const REVIEW_TAGS = ['剧情精彩', '演技在线', '特效震撼', '配乐好听', '值回票价', '节奏拖沓'];
const SEED_REVIEWS = [
  { name: '午夜场常客', star: 5, text: '散场的时候全场都在鼓掌，结尾那段真的忍不住，值得二刷。', likes: 3260 },
  { name: '爆米花不加糖', star: 4, text: '前半段稍慢，后半段节奏一下子起来了，大银幕看很过瘾。', likes: 1185 },
  { name: '影院第七排', star: 4, text: '演员都很在状态，配乐加分，就是片长有点考验膀胱。', likes: 642 }
];
module.exports = {
  MOVIES, CINEMAS, COUPONS, CODES, RECHARGE, VIP_PRICE, SERVICE_FEE, SNACK_PRICE, REFUND_FEE, STAR_TEXT, REVIEW_TAGS, SEED_REVIEWS,
  shows, days, dayLabel, getMovie, getCinema, pad
};
