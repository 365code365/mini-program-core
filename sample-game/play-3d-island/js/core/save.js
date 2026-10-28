// 存档：版本号 + 迁移 + 校验和，自动存（每 10 秒、切后台、关键操作后立刻）。
// 存的是「状态」，不存派生值（产量、上限这些每次按 defs 现算），调数值不用洗档。
// 校验和防的是「存档写坏了」（写一半被杀、手改 JSON），不是防作弊：真要防作弊得服务器记账。
var KEY = 'island.save', VERSION = 3;

function fresh(now) {
  return {
    v: VERSION, created: now, saved: now, lastTick: now,
    res: { coin: 200, gem: 50, wheat: 0, wood: 30, stone: 10, bread: 0, fish: 0, honey: 0 },
    lv: 1, xp: 0,
    buildings: [],          // { uid, type, x, z, rot, lv, store, busyUntil, busyKind: 'build'|'upgrade'|null }
    nextUid: 1,
    expanded: [],           // 扩出来的地块 "x,z"
    pets: {},               // id → { n: 拥有几只, home: 是否住进小屋 }
    pity: 0, pulls: 0,
    stats: {},              // 累计计数（成就用）：build、earn:coin、collect …
    daily: { day: '', counts: {}, claimed: {} },
    ach: {},                // id → 已领到第几阶
    signin: { last: '', streak: 0, total: 0 },
    shop: { firstBuy: {}, starterUntil: 0, starterBought: false, monthlyUntil: 0, monthlyClaimed: '' },
    orders: [],             // 未完成的支付单（先记单再拉支付，回来补发）{ id, item, at, state }
    boostUntil: 0,          // 看广告 2 倍产量到期时刻
    redeemed: {},
    settings: { music: true, sfx: true, vibrate: true, quality: 'auto' },
    tutorial: 0
  };
}

// 迁移：每一版对上一版补字段；只往前迁
var MIGRATE = {
  1: function (s) { s.orders = s.orders || []; s.redeemed = s.redeemed || {}; },
  2: function (s) { s.shop = s.shop || {}; s.shop.monthlyClaimed = s.shop.monthlyClaimed || ''; s.boostUntil = s.boostUntil || 0; }
};

function checksum(str) {
  var h = 2166136261 >>> 0;                         // FNV-1a
  for (var i = 0; i < str.length; i++) { h ^= str.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; }
  return h.toString(36);
}

function load(now) {
  var raw = null;
  try { raw = wx.getStorageSync(KEY); } catch (e) { raw = null; }
  if (!raw || typeof raw !== 'object' || typeof raw.body !== 'string') return { state: fresh(now), status: 'new' };
  if (checksum(raw.body) !== raw.sum) {
    console.warn('[save] 校验和不对，存档可能写坏了：用备份');
    try { raw = wx.getStorageSync(KEY + '.bak'); } catch (e) { raw = null; }
    if (!raw || typeof raw.body !== 'string' || checksum(raw.body) !== raw.sum) return { state: fresh(now), status: 'corrupt' };
  }
  var s;
  try { s = JSON.parse(raw.body); } catch (e) { return { state: fresh(now), status: 'corrupt' }; }
  var base = fresh(now);
  for (var v = s.v || 1; v < VERSION; v++) if (MIGRATE[v]) MIGRATE[v](s);
  // 新版加了字段的：用默认值补齐（深一层）
  for (var k in base) {
    if (!(k in s)) s[k] = base[k];
    else if (base[k] && typeof base[k] === 'object' && !Array.isArray(base[k])) for (var kk in base[k]) if (!(kk in s[k])) s[k][kk] = base[k][kk];
  }
  s.v = VERSION;
  return { state: s, status: 'ok' };
}

function save(s, now) {
  s.saved = now;
  var body = JSON.stringify(s);
  var rec = { body: body, sum: checksum(body), v: VERSION };
  try {
    var prev = wx.getStorageSync(KEY);
    if (prev && prev.body) wx.setStorageSync(KEY + '.bak', prev);     // 先把上一份挪成备份，写坏了还能回退
    wx.setStorageSync(KEY, rec);
    return true;
  } catch (e) { console.error('[save] 写存档失败', e); return false; }
}

function wipe() { try { wx.removeStorageSync(KEY); wx.removeStorageSync(KEY + '.bak'); } catch (e) {} }

module.exports = { load: load, save: save, wipe: wipe, fresh: fresh, VERSION: VERSION, checksum: checksum };
