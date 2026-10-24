// 游戏状态：体力、段位、道具、战绩、错题本、排行榜（全部 storage）
const MAX_ENERGY = 10;
const RECOVER_MS = 5 * 60 * 1000;

function get(key, def) {
  const v = wx.getStorageSync('quiz_' + key);
  return v === '' || v === undefined || v === null ? def : v;
}
function set(key, v) { wx.setStorageSync('quiz_' + key, v); }

// 体力：存 { energy, ts }，ts 是「下一点开始累计」的时刻；每满 5 分钟 +1，满了就不再累计
function energy() {
  const now = Date.now();
  let e = get('energy', null);
  if (!e) e = { energy: 6, ts: now - 2 * 60 * 1000 - 25 * 1000 };
  if (e.energy >= MAX_ENERGY) {
    e = { energy: MAX_ENERGY, ts: now };
  } else {
    const gained = Math.floor((now - e.ts) / RECOVER_MS);
    if (gained > 0) {
      e.energy = Math.min(MAX_ENERGY, e.energy + gained);
      e.ts = e.energy >= MAX_ENERGY ? now : e.ts + gained * RECOVER_MS;
    }
  }
  set('energy', e);
  const left = e.energy >= MAX_ENERGY ? 0 : RECOVER_MS - (now - e.ts);
  return { energy: e.energy, max: MAX_ENERGY, left };
}
function useEnergy() {
  const s = energy();
  if (s.energy <= 0) return false;
  const e = get('energy');
  // 从满体力扣下去时，从此刻开始累计恢复
  if (e.energy >= MAX_ENERGY) e.ts = Date.now();
  e.energy -= 1;
  set('energy', e);
  return true;
}
function addEnergy(n) {
  const e = get('energy');
  e.energy = Math.min(MAX_ENERGY, e.energy + n);
  set('energy', e);
}

const TIERS = [
  { t: '倔强青铜', min: 0, c: '#B08D57' },
  { t: '秩序白银', min: 2000, c: '#94A3B8' },
  { t: '荣耀黄金', min: 5000, c: '#F59E0B' },
  { t: '尊贵铂金', min: 9000, c: '#14B8A6' },
  { t: '永恒钻石', min: 14000, c: '#6366F1' },
  { t: '最强王者', min: 20000, c: '#E11D48' }
];
function tier(total) {
  let i = 0;
  while (i + 1 < TIERS.length && total >= TIERS[i + 1].min) i++;
  const cur = TIERS[i];
  const next = TIERS[i + 1];
  const span = next ? next.min - cur.min : 1;
  const into = next ? total - cur.min : 1;
  // 每段 3 小段（III → I）
  const sub = next ? ['III', 'II', 'I'][Math.min(2, Math.floor(into / span * 3))] : '';
  return { name: cur.t + (sub ? ' ' + sub : ''), color: cur.c, next: next ? next.t : '', need: next ? next.min - total : 0, pct: Math.round(into / span * 100), idx: i };
}

function profile() { return get('profile', { total: 3260, best: 0, games: 0, props: { half: 3, skip: 2 } }); }
function saveProfile(p) { set('profile', p); }
function cats() { return get('cats', ['history', 'geo', 'science']); }

function wrongs() { return get('wrongs', []); }
function addWrong(q, picked) {
  const list = wrongs().filter(w => w.id !== q.id);
  list.unshift({ id: q.id, picked, time: Date.now(), times: (wrongs().find(w => w.id === q.id) || { times: 0 }).times + 1 });
  set('wrongs', list.slice(0, 100));
}
function removeWrong(id) { set('wrongs', wrongs().filter(w => w.id !== id)); }

// 排行榜：固定的机器人选手 + 自己的最好成绩
const NAMES = ['学霸小林', '知识海绵', '答题狂魔', '北大在读', '百科全书', '脑力担当', '一题不错', '夜猫子', '冲鸭', '奶茶续命', '小鱼干', '木子李', '橙子味', '风一样', '程序猿老张', '大白兔', '江南第一题', '苏打水', '甜甜圈', '起风了', '半糖去冰', '月亮不睡', '星河', '咸鱼翻身', '阿飞', '蓝鲸', '番茄炒蛋', '清风徐来', '向日葵', '早睡早起'];
function bots(seed, base) {
  return NAMES.map((n, i) => ({ name: n, score: Math.max(300, base - i * 97 - ((i * seed) % 7) * 31), city: ['北京', '上海', '广州', '深圳', '杭州', '成都', '武汉', '南京'][(i + seed) % 8] }));
}
function board(kind) {
  const p = profile();
  const mine = kind === 'today' ? get('todayBest', 1860) : get('weekBest', 6420);
  const list = bots(kind === 'today' ? 3 : 5, kind === 'today' ? 3150 : 9800).concat([{ name: '我', score: mine, city: '杭州', me: true }]);
  list.sort((a, b) => b.score - a.score || (a.me ? -1 : b.me ? 1 : 0));
  return { list: list.map((x, i) => Object.assign({ rank: i + 1 }, x)), total: p.total };
}
function record(score) {
  if (score > get('todayBest', 1860)) set('todayBest', score);
  // 本周累计：一周内每局得分加起来
  set('weekBest', get('weekBest', 6420) + score);
}

module.exports = { get, set, energy, useEnergy, addEnergy, tier, profile, saveProfile, cats, wrongs, addWrong, removeWrong, board, record, MAX_ENERGY };
