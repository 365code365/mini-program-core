// 静态数据表（策划配置）：建筑、宠物、商店、任务、签到、等级、地块。数值都在这里调，逻辑代码里不写死数字。
// 时间单位：秒；产量单位：每分钟。

// 资源：id → 显示名、图标、卖价（集市）
var RES = {
  coin:  { name: '金币', icon: 'coin' },
  gem:   { name: '钻石', icon: 'gem' },
  wheat: { name: '小麦', icon: 'wheat', sell: 2 },
  wood:  { name: '木材', icon: 'wood', sell: 3 },
  stone: { name: '石料', icon: 'stone', sell: 4 },
  bread: { name: '面包', icon: 'bread', sell: 14 },
  fish:  { name: '鲜鱼', icon: 'fish', sell: 9 },
  honey: { name: '蜂蜜', icon: 'honey', sell: 22 }
};
var GOODS = ['wheat', 'wood', 'stone', 'bread', 'fish', 'honey'];

// 建筑：rate = 1 级每分钟产量；每级 ×rateGrowth。cost 是 1→2 级的花费，每级 ×costGrowth；time 是升级用时（秒），每级 ×timeGrowth。
// consume：生产时消耗别的资源（面包坊 2 小麦 → 1 面包）。cap = 1 级时这座建筑自己攒得下多少（满了停产，等玩家来收）
var BUILDINGS = {
  farm:      { name: '麦田', desc: '种小麦，面包坊的原料', unlock: 1, produce: 'wheat', rate: 12, cap: 60, buildCost: { coin: 30 },
               cost: { coin: 40, wood: 5 }, costGrowth: 1.6, rateGrowth: 1.28, time: 6, timeGrowth: 1.5, maxLv: 20, color: 0xf2c14e },
  lumber:    { name: '伐木场', desc: '砍树得木材，建造升级都要用', unlock: 1, produce: 'wood', rate: 8, cap: 40, buildCost: { coin: 50 },
               cost: { coin: 60, stone: 4 }, costGrowth: 1.6, rateGrowth: 1.27, time: 8, timeGrowth: 1.5, maxLv: 20, color: 0x8d5a3b },
  quarry:    { name: '采石场', desc: '开采石料', unlock: 2, produce: 'stone', rate: 6, cap: 30, buildCost: { coin: 120, wood: 20 },
               cost: { coin: 90, wood: 12 }, costGrowth: 1.62, rateGrowth: 1.27, time: 10, timeGrowth: 1.5, maxLv: 20, color: 0x9aa5b1 },
  bakery:    { name: '面包坊', desc: '2 小麦烤成 1 面包，面包很值钱', unlock: 3, produce: 'bread', rate: 4, cap: 20, consume: { wheat: 2 },
               buildCost: { coin: 300, wood: 40, stone: 20 }, cost: { coin: 220, wood: 25, stone: 15 }, costGrowth: 1.65, rateGrowth: 1.26, time: 15, timeGrowth: 1.55, maxLv: 15, color: 0xe76f51 },
  dock:      { name: '渔港', desc: '出海打鱼，只能建在海边', unlock: 4, produce: 'fish', rate: 5, cap: 25, coastal: true,
               buildCost: { coin: 600, wood: 80 }, cost: { coin: 380, wood: 40, stone: 20 }, costGrowth: 1.65, rateGrowth: 1.26, time: 18, timeGrowth: 1.55, maxLv: 15, color: 0x3a86ff },
  apiary:    { name: '蜂场', desc: '采蜜，宠物加成翻倍', unlock: 6, produce: 'honey', rate: 2.5, cap: 12,
               buildCost: { coin: 1500, wood: 120, stone: 80 }, cost: { coin: 900, wood: 70, stone: 50 }, costGrowth: 1.7, rateGrowth: 1.25, time: 25, timeGrowth: 1.6, maxLv: 12, color: 0xffb703 },
  warehouse: { name: '仓库', desc: '岛上资源上限 +40%/级', unlock: 2, storage: 0.4, buildCost: { coin: 200, wood: 30 },
               cost: { coin: 160, wood: 30, stone: 10 }, costGrowth: 1.7, time: 10, timeGrowth: 1.5, maxLv: 15, color: 0x6d597a },
  market:    { name: '集市', desc: '把货物卖成金币', unlock: 2, market: true, unique: true, buildCost: { coin: 150, wood: 20 },
               cost: { coin: 200, wood: 30, stone: 20 }, costGrowth: 1.8, time: 12, timeGrowth: 1.5, maxLv: 10, color: 0xef476f },
  pethouse:  { name: '宠物小屋', desc: '每级多住一只宠物，宠物给全岛加成', unlock: 4, pets: 1, unique: true, buildCost: { coin: 800, wood: 60, stone: 40 },
               cost: { coin: 700, wood: 50, stone: 40 }, costGrowth: 1.9, time: 20, timeGrowth: 1.6, maxLv: 5, color: 0x06d6a0 },
  lighthouse:{ name: '灯塔', desc: '全岛产量 +10%（装饰，钻石建造）', unlock: 5, boost: 0.1, unique: true, buildCost: { gem: 60 },
               cost: { gem: 40 }, costGrowth: 1.5, time: 30, timeGrowth: 1.4, maxLv: 5, color: 0xffffff }
};
var BUILD_ORDER = ['farm', 'lumber', 'quarry', 'warehouse', 'market', 'bakery', 'dock', 'pethouse', 'lighthouse', 'apiary'];

// 宠物：稀有度决定加成档位；bonus 是对某种资源产量的加成（all = 全部）
var RARITY = {
  N:   { name: '普通', color: '#9aa5b1', weight: 60, mult: 1 },
  R:   { name: '稀有', color: '#4ea8de', weight: 28, mult: 2 },
  SR:  { name: '史诗', color: '#b66dff', weight: 10, mult: 3.5 },
  SSR: { name: '传说', color: '#ffb703', weight: 2, mult: 6 }
};
var PETS = [
  { id: 'chick',  name: '小鸡啾啾', rarity: 'N',   body: 0xffe066, ear: 0xff9f1c, bonus: 'wheat', pct: 5 },
  { id: 'bunny',  name: '兔兔团子', rarity: 'N',   body: 0xffffff, ear: 0xffafcc, bonus: 'wheat', pct: 5 },
  { id: 'pup',    name: '柴柴',     rarity: 'N',   body: 0xe9a15c, ear: 0x8c5a2b, bonus: 'wood', pct: 5 },
  { id: 'piggy',  name: '粉粉猪',   rarity: 'N',   body: 0xffb5c2, ear: 0xff8fa3, bonus: 'coin', pct: 3 },
  { id: 'cat',    name: '橘猫大福', rarity: 'R',   body: 0xf4a259, ear: 0xd1752a, bonus: 'fish', pct: 6 },
  { id: 'panda',  name: '滚滚',     rarity: 'R',   body: 0xffffff, ear: 0x222222, bonus: 'wood', pct: 6 },
  { id: 'frog',   name: '呱太',     rarity: 'R',   body: 0x8ac926, ear: 0x5c8a12, bonus: 'stone', pct: 6 },
  { id: 'fox',    name: '火狐',     rarity: 'SR',  body: 0xff7b00, ear: 0xffffff, bonus: 'bread', pct: 8 },
  { id: 'owl',    name: '学者猫头鹰', rarity: 'SR', body: 0x9c6644, ear: 0xe6ccb2, bonus: 'all', pct: 4 },
  { id: 'unicorn',name: '彩虹独角兽', rarity: 'SSR', body: 0xf8f0ff, ear: 0xc77dff, bonus: 'all', pct: 10, horn: true },
  { id: 'dragon', name: '小金龙',   rarity: 'SSR', body: 0xffd60a, ear: 0xff006e, bonus: 'coin', pct: 15, horn: true }
];
var GACHA = { single: 60, ten: 540, pity: 50, dupeCoins: { N: 80, R: 200, SR: 600, SSR: 2000 } };

// 商店：钻石包走虚拟支付（price 单位元；首充翻倍）；金币包花钻石
var SHOP = {
  gems: [
    { id: 'g6',   price: 6,   gems: 60,   tag: '' },
    { id: 'g30',  price: 30,  gems: 330,  tag: '+10%' },
    { id: 'g68',  price: 68,  gems: 780,  tag: '+15%' },
    { id: 'g128', price: 128, gems: 1520, tag: '热卖' },
    { id: 'g328', price: 328, gems: 4100, tag: '+25%' },
    { id: 'g648', price: 648, gems: 8400, tag: '最划算' }
  ],
  coins: [
    { id: 'c1', gems: 20,  coinMinutes: 30 },          // 给 = 当前全岛每分钟金币当量 × 分钟数（随进度涨）
    { id: 'c2', gems: 90,  coinMinutes: 150 },
    { id: 'c3', gems: 320, coinMinutes: 600 }
  ],
  starter: { id: 'starter', price: 6, gems: 300, coin: 3000, pet: 'fox', hours: 48 },
  monthly: { id: 'monthly', price: 30, gems: 300, daily: 100, days: 30 }
};
var MIDAS = { offerId: '1450000000', env: 0, currencyType: 'CNY', zoneId: '1', rate: 10 };   // 1 元 = 10 游戏币

// 每日任务（每天 0 点重置）：key 对应事件名，n 是目标次数
var DAILY = [
  { id: 'd_collect', name: '收获 30 次', key: 'collect', n: 30, reward: { coin: 300 } },
  { id: 'd_upgrade', name: '升级建筑 2 次', key: 'upgrade', n: 2, reward: { gem: 10 } },
  { id: 'd_sell',    name: '在集市卖货 3 次', key: 'sell', n: 3, reward: { coin: 500 } },
  { id: 'd_ad',      name: '看 1 次广告', key: 'ad', n: 1, reward: { gem: 15 } },
  { id: 'd_gacha',   name: '孵 1 次宠物蛋', key: 'gacha', n: 1, reward: { gem: 20 } },
  { id: 'd_bread',   name: '生产 50 个面包', key: 'make:bread', n: 50, reward: { gem: 15 } }
];
// 成就（一次性，分阶）
var ACHIEVEMENTS = [
  { id: 'a_build',  name: '岛主', key: 'build', tiers: [3, 10, 25], reward: [{ gem: 20 }, { gem: 50 }, { gem: 120 }] },
  { id: 'a_coin',   name: '小富翁', key: 'earn:coin', tiers: [1e4, 1e5, 1e6], reward: [{ gem: 20 }, { gem: 60 }, { gem: 200 }] },
  { id: 'a_pets',   name: '宠物收藏家', key: 'pets', tiers: [3, 6, 11], reward: [{ gem: 30 }, { gem: 80 }, { gem: 300 }] },
  { id: 'a_level',  name: '小岛繁荣', key: 'level', tiers: [5, 10, 20], reward: [{ gem: 30 }, { gem: 100 }, { gem: 300 }] }
];
var SIGNIN = [{ coin: 300 }, { gem: 20 }, { coin: 800 }, { gem: 30 }, { coin: 2000 }, { gem: 50 }, { gem: 120, egg: 1 }];

// 等级：升到 lv+1 要的经验；经验来自收获（每次 1）、建造（10）、升级（5×目标等级）
function xpFor(lv) { return Math.round(60 * Math.pow(1.45, lv - 1)); }

// 地图：MAP×MAP 格；中心 START 半径内一开始就能用，外圈按环一块块扩（价格按已扩块数涨）
var MAP = 14, START_R = 3.2;
function expandCost(n) { return { coin: Math.round(400 * Math.pow(1.35, n)) }; }

var OFFLINE_CAP_H = 8;
var REDEEM = { MINI2026: { gem: 200, coin: 5000 }, WELCOME: { gem: 50 } };

module.exports = { RES: RES, GOODS: GOODS, BUILDINGS: BUILDINGS, BUILD_ORDER: BUILD_ORDER, RARITY: RARITY, PETS: PETS, GACHA: GACHA,
  SHOP: SHOP, MIDAS: MIDAS, DAILY: DAILY, ACHIEVEMENTS: ACHIEVEMENTS, SIGNIN: SIGNIN, xpFor: xpFor, MAP: MAP, START_R: START_R,
  expandCost: expandCost, OFFLINE_CAP_H: OFFLINE_CAP_H, REDEEM: REDEEM };
