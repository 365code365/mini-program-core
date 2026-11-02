// 演出数据（艺人、剧目为虚构；场馆为真实场馆名）。开售时间相对「今天」生成，开售日历与倒计时任何一天打开都有内容。
const DAY = 86400000;
function at(dayOffset, hh, mm) {
  const d = new Date();
  d.setHours(hh, mm || 0, 0, 0);
  return d.getTime() + dayOffset * DAY;
}

const CITIES = ['上海', '北京', '杭州', '广州', '成都', '深圳'];
const CATS = [
  { id: 'all', name: '全部' },
  { id: 'concert', name: '演唱会' },
  { id: 'festival', name: '音乐节' },
  { id: 'drama', name: '话剧' },
  { id: 'musical', name: '音乐剧' },
  { id: 'talk', name: '脱口秀' },
  { id: 'live', name: 'Livehouse' },
  { id: 'classic', name: '古典' }
];

// seat: true 的演出走选座；tiers 的 rows 是该票档在座位图上占的排（含首尾，从 1 起）
const SHOWS = [
  {
    id: 's1', cat: 'concert', city: '上海', title: '林屿「潮汐」2026 巡回演唱会·上海站', artist: '林屿',
    venue: '梅赛德斯-奔驰文化中心', addr: '上海市浦东新区世博大道1200号', phone: '021-20206789',
    poster: '/images/poster/s1.jpg', seat: true, saleAt: at(-12, 12), want: 23854, duration: '约150分钟',
    tags: ['实名购票', '每单限购4张', '不支持退票'], refundable: false,
    sessions: [
      { id: 's1-1', date: '2026-12-19', week: '周六', time: '19:30' },
      { id: 's1-2', date: '2026-12-20', week: '周日', time: '19:30' }
    ],
    tiers: [
      { id: 'A', name: '内场 VIP', price: 1680, color: '#ff9f43', rows: [1, 2] },
      { id: 'B', name: '内场', price: 1280, color: '#ff6b81', rows: [3, 5] },
      { id: 'C', name: '看台 A 区', price: 880, color: '#a55eea', rows: [6, 8] },
      { id: 'D', name: '看台 B 区', price: 580, color: '#4b7bec', rows: [9, 11] },
      { id: 'E', name: '看台 C 区', price: 380, color: '#20bf6b', rows: [12, 13] }
    ],
    intro: '林屿出道十周年后的首轮大型巡演，以「潮汐」为主题，用四个篇章串起《退潮》《夜航》《灯塔》等 26 首作品。舞台采用 360° 环形升降设计，内场观众可近距离感受潮水般的灯光矩阵。'
  },
  {
    id: 's2', cat: 'festival', city: '上海', title: '夏日回声音乐节 2026 冬季场', artist: '白噪 / 苏沐 / 南岸乐队 等 16 组',
    venue: '上海世博公园', addr: '上海市浦东新区浦明路1号', phone: '021-58812233',
    poster: '/images/poster/s2.jpg', seat: false, saleAt: at(-20, 10), want: 15420, duration: '每日 13:00-22:00',
    tags: ['实名购票', '每单限购4张', '演出前7天可退'], refundable: true,
    sessions: [
      { id: 's2-1', date: '2026-12-26', week: '周六', time: '13:00' },
      { id: 's2-2', date: '2026-12-27', week: '周日', time: '13:00' }
    ],
    tiers: [
      { id: 'A', name: '单日票', price: 399, color: '#ff9f43' },
      { id: 'B', name: '单日 VIP（含休息区）', price: 899, color: '#ff6b81' },
      { id: 'C', name: '早鸟单日票', price: 299, color: '#20bf6b', soldOut: true }
    ],
    intro: '两天两个舞台、16 组乐队与音乐人。园区设有热饮市集与暖身帐篷，请提前查看天气，注意保暖。'
  },
  {
    id: 's3', cat: 'drama', city: '上海', title: '原创话剧《雾港来信》', artist: '雾港剧社',
    venue: '上海大剧院·中剧场', addr: '上海市黄浦区人民大道300号', phone: '021-63868686',
    poster: '/images/poster/s3.jpg', seat: true, saleAt: at(-6, 10), want: 6802, duration: '约130分钟（含中场休息15分钟）',
    tags: ['实名购票', '1.2米以下儿童谢绝入场', '演出前48小时可退'], refundable: true,
    sessions: [
      { id: 's3-1', date: '2026-12-11', week: '周五', time: '19:30' },
      { id: 's3-2', date: '2026-12-12', week: '周六', time: '14:00' },
      { id: 's3-3', date: '2026-12-12', week: '周六', time: '19:30' }
    ],
    tiers: [
      { id: 'A', name: '池座前区', price: 480, color: '#ff6b81', rows: [1, 4] },
      { id: 'B', name: '池座中区', price: 380, color: '#a55eea', rows: [5, 8] },
      { id: 'C', name: '池座后区', price: 280, color: '#4b7bec', rows: [9, 11] },
      { id: 'D', name: '楼座', price: 180, color: '#20bf6b', rows: [12, 13] }
    ],
    intro: '1937 年的港口小城，一封迟到四十年的信，把三代人的命运重新系在一起。全剧采用旋转舞台与实时投影，讲述关于离别与重逢的故事。'
  },
  {
    id: 's4', cat: 'talk', city: '上海', title: '「周末不加班」脱口秀专场', artist: '阿凯 / 小满 / 老周',
    venue: '上海·笑果工厂', addr: '上海市静安区昌平路68号', phone: '021-62180088',
    poster: '/images/poster/s4.jpg', seat: false, saleAt: at(-3, 18), want: 3260, duration: '约90分钟',
    tags: ['16岁以下谢绝入场', '演出前24小时可退'], refundable: true,
    sessions: [
      { id: 's4-1', date: '2026-11-14', week: '周六', time: '20:00' },
      { id: 's4-2', date: '2026-11-21', week: '周六', time: '20:00' }
    ],
    tiers: [
      { id: 'A', name: '前排', price: 260, color: '#ff6b81' },
      { id: 'B', name: '中排', price: 180, color: '#a55eea' },
      { id: 'C', name: '后排', price: 120, color: '#20bf6b' }
    ],
    intro: '三位演员轮番上阵，聊打工、租房和相亲。现场不录像，段子只在今晚。'
  },
  {
    id: 's5', cat: 'live', city: '杭州', title: '白噪「回声定位」Livehouse 巡演·杭州站', artist: '白噪乐队',
    venue: '杭州·MAO Livehouse', addr: '杭州市西湖区文三路477号', phone: '0571-88823456',
    poster: '/images/poster/s5.jpg', seat: false, saleAt: at(5, 18), want: 4198, duration: '约120分钟',
    tags: ['站票', '实名入场', '不支持退票'], refundable: false,
    sessions: [{ id: 's5-1', date: '2026-12-05', week: '周六', time: '20:30' }],
    tiers: [
      { id: 'A', name: '预售站票', price: 168, color: '#20bf6b' },
      { id: 'B', name: '现场站票', price: 228, color: '#4b7bec' }
    ],
    intro: '新专辑《回声定位》首轮巡演，后摇与噪音的现场，建议佩戴耳塞。'
  },
  {
    id: 's6', cat: 'musical', city: '上海', title: '原创音乐剧《星河旅馆》中文版', artist: '星河剧团',
    venue: '上汽·上海文化广场', addr: '上海市黄浦区永嘉路36号', phone: '021-64729000',
    poster: '/images/poster/s6.jpg', seat: true, saleAt: at(3, 10), want: 9512, duration: '约160分钟（含中场休息20分钟）',
    tags: ['实名购票', '每单限购4张', '演出前72小时可退'], refundable: true,
    sessions: [
      { id: 's6-1', date: '2027-01-08', week: '周五', time: '19:30' },
      { id: 's6-2', date: '2027-01-09', week: '周六', time: '14:00' }
    ],
    tiers: [
      { id: 'A', name: 'VIP 区', price: 799, color: '#ff9f43', rows: [1, 3] },
      { id: 'B', name: '甲等', price: 599, color: '#ff6b81', rows: [4, 7] },
      { id: 'C', name: '乙等', price: 399, color: '#4b7bec', rows: [8, 11] },
      { id: 'D', name: '丙等', price: 199, color: '#20bf6b', rows: [12, 13] }
    ],
    intro: '一间只在流星夜出现的旅馆，七位旅人各自带着未说出口的愿望入住。全剧 22 首原创曲目，现场乐队伴奏。'
  },
  {
    id: 's7', cat: 'classic', city: '北京', title: '苏沐「月光与河流」钢琴独奏音乐会', artist: '苏沐',
    venue: '国家大剧院·音乐厅', addr: '北京市西城区西长安街2号', phone: '010-66550000',
    poster: '/images/poster/s7.jpg', seat: true, saleAt: at(-30, 10), want: 2875, duration: '约100分钟',
    tags: ['实名购票', '1.2米以下儿童谢绝入场', '演出前48小时可退'], refundable: true,
    sessions: [{ id: 's7-1', date: '2026-12-31', week: '周四', time: '19:30' }],
    tiers: [
      { id: 'A', name: '一等', price: 680, color: '#ff6b81', rows: [1, 4] },
      { id: 'B', name: '二等', price: 480, color: '#a55eea', rows: [5, 8] },
      { id: 'C', name: '三等', price: 280, color: '#20bf6b', rows: [9, 13] }
    ],
    intro: '曲目：德彪西《月光》、肖邦《第一叙事曲》、拉威尔《水之嬉戏》及苏沐原创组曲《河流》。'
  },
  {
    id: 's8', cat: 'concert', city: '北京', title: 'NEON WAVE 2027 演唱会·北京站', artist: 'NEON WAVE',
    venue: '国家体育馆', addr: '北京市朝阳区天辰东路9号', phone: '010-84373008',
    poster: '/images/poster/s8.jpg', seat: true, saleAt: at(1, 12), want: 31206, duration: '约140分钟',
    tags: ['实名购票', '每单限购4张', '不支持退票'], refundable: false,
    sessions: [
      { id: 's8-1', date: '2027-01-16', week: '周六', time: '19:00' },
      { id: 's8-2', date: '2027-01-17', week: '周日', time: '19:00' }
    ],
    tiers: [
      { id: 'A', name: '内场 VIP', price: 1580, color: '#ff9f43', rows: [1, 2] },
      { id: 'B', name: '内场', price: 1180, color: '#ff6b81', rows: [3, 5] },
      { id: 'C', name: '看台', price: 780, color: '#a55eea', rows: [6, 9] },
      { id: 'D', name: '看台后区', price: 480, color: '#20bf6b', rows: [10, 13] }
    ],
    intro: '合成器流行乐队 NEON WAVE 首次登陆体育馆，霓虹激光与全息投影同场呈现。'
  }
];

const NOTICE = [
  { k: '实名制', v: '一张有效证件同一场次限购 1 张，入场时需出示购票时填写的证件原件。' },
  { k: '入场时间', v: '演出开始前 60 分钟开放入场，迟到观众需待节目间隙由工作人员引导入场。' },
  { k: '禁止携带', v: '专业摄影设备、自拍杆、充电宝以外的锂电池、易燃易爆物品、食品饮料。' },
  { k: '电子票', v: '无需取纸质票，在「票夹」出示电子票二维码入场，截图无效。' }
];

const byId = {};
SHOWS.forEach((s) => { byId[s.id] = s; });

function getShow(id) { return byId[id] || null; }
function minPrice(s) { return Math.min.apply(null, s.tiers.map((t) => t.price)); }
function onSale(s, now) { return (now || Date.now()) >= s.saleAt; }

module.exports = { CITIES, CATS, SHOWS, NOTICE, getShow, minPrice, onSale, DAY };
