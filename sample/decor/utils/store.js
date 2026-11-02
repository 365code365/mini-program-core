// 本地数据：收藏、预约 / 装修项目、户型方案、预算方案都走 storage
const K = {
  seeded: 'decor_seeded',
  favCases: 'decor_fav_cases',
  favDesigners: 'decor_fav_designers',
  projects: 'decor_projects',
  layout: 'decor_layout',
  budgets: 'decor_budgets',
  user: 'decor_user'
};

const STAGES = ['预约量房', '上门量房', '方案设计', '签约报价', '开工交底', '水电改造', '泥木施工', '油漆涂装', '安装收尾', '竣工验收'];
const DAY = 86400000;

function get(key, def) {
  const v = wx.getStorageSync(key);
  return v === '' || v === undefined || v === null ? def : v;
}
function set(key, v) { wx.setStorageSync(key, v); }

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function ymd(ts) { const d = new Date(ts); return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()); }
function ymdhm(ts) { const d = new Date(ts); return ymd(ts) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes()); }

function orderNo(ts) {
  const d = new Date(ts);
  return 'HZ' + d.getFullYear() + pad(d.getMonth() + 1) + pad(d.getDate()) + String(Math.floor(ts / 1000) % 100000).padStart(5, '0');
}

// 首次启动：一个正在施工的项目（水电阶段）+ 两个收藏
function seed() {
  if (get(K.seeded, 0)) return;
  const now = Date.now();
  const start = now - 52 * DAY;
  const logs = STAGES.slice(0, 6).map((s, i) => ({ stage: i, ts: start + [0, 2, 9, 20, 31, 38][i] * DAY, text: s + (i < 5 ? ' 已完成' : ' 进行中') }));
  const project = {
    id: 'p' + start.toString(36), no: orderNo(start), name: '林女士', phone: '13867451209', community: '滨江 春晓府 3 幢 1802',
    area: 112, houseType: '三室两厅', date: ymd(start + 2 * DAY), slot: '上午 9:00-12:00', budget: '35~45 万',
    designer: 'd1', caseId: 'c1', remark: '家里有猫，阳台想做猫爬架', stage: 5, createdAt: start, logs,
    diaries: [
      { id: 'y3', ts: now - 2 * DAY, stage: 5, text: '水电进场第 5 天。厨房插座加到了 9 个，冰箱位预留了独立回路；卫生间做了闭水试验 48 小时，楼下没有渗水。', photo: '/images/rooms/r1-raw.jpg' },
      { id: 'y2', ts: now - 14 * DAY, stage: 4, text: '开工交底，项目经理带着把每面墙的拆改都用红线标了。拆了厨房和客厅之间的非承重墙，空间一下亮了。', photo: '/images/rooms/r3-raw.jpg' },
      { id: 'y1', ts: now - 32 * DAY, stage: 3, text: '签约。报价单逐项过了一遍，把吊顶从全屋改成只做过道和卫生间，省了 6000 多。', photo: '' }
    ]
  };
  wx.setStorageSync(K.projects, [project]);
  wx.setStorageSync(K.favCases, ['c3', 'c6']);
  wx.setStorageSync(K.user, { nick: '林女士', phone: '13867451209' });
  wx.setStorageSync(K.seeded, 1);
}

function projects() { return get(K.projects, []); }
function project(id) { return projects().find((p) => p.id === id); }
function saveProject(p) {
  const list = projects();
  const i = list.findIndex((x) => x.id === p.id);
  if (i >= 0) list[i] = p; else list.unshift(p);
  set(K.projects, list);
}

// 已完成的节点占比：当前节点还没确认，不算完成
function percent(p) { return Math.round((p.stage + (p.finished ? 1 : 0)) / STAGES.length * 100); }

function toggle(key, id) {
  let list = get(key, []);
  const on = list.indexOf(id) < 0;
  list = on ? [id].concat(list) : list.filter((x) => x !== id);
  set(key, list);
  return on;
}

module.exports = { K, STAGES, DAY, get, set, seed, ymd, ymdhm, orderNo, projects, project, saveProject, toggle, percent };
