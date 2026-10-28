// 宠物蛋：按稀有度权重抽，再在该稀有度里均匀抽；保底：连续 GACHA.pity 次没出 SSR，下一次必出。
// 十连保底至少一只 SR 以上。重复的宠物换金币（不浪费）。概率公示（rates）给界面显示 —— 正式上线的抽卡必须公示。
var D = require('../data/defs.js');

function make(state, eco, bus, rnd) {
  rnd = rnd || Math.random;
  var total = 0; for (var k in D.RARITY) total += D.RARITY[k].weight;
  function rates() {
    var out = {};
    for (var r in D.RARITY) {
      var pets = D.PETS.filter(function (p) { return p.rarity === r; });
      out[r] = { pct: D.RARITY[r].weight / total * 100, pets: pets.map(function (p) { return p.name; }) };
    }
    return out;
  }
  function rollRarity(minR) {
    if (state.pity + 1 >= D.GACHA.pity) return 'SSR';
    var order = ['N', 'R', 'SR', 'SSR'];
    var x = rnd() * total, r = 'N';
    for (var i = 0; i < order.length; i++) { x -= D.RARITY[order[i]].weight; if (x < 0) { r = order[i]; break; } }
    if (minR && order.indexOf(r) < order.indexOf(minR)) r = minR;
    return r;
  }
  function one(minR) {
    var r = rollRarity(minR);
    state.pity = r === 'SSR' ? 0 : state.pity + 1;
    state.pulls++;
    var pool = D.PETS.filter(function (p) { return p.rarity === r; });
    var pet = pool[Math.floor(rnd() * pool.length)];
    var o = state.pets[pet.id] || (state.pets[pet.id] = { n: 0, home: false });
    o.n++;
    var dupe = o.n > 1, refund = dupe ? D.GACHA.dupeCoins[r] : 0;
    if (refund) eco.add({ coin: refund }, 'dupe');
    // 新宠物：小屋还有空位就自动住进去
    if (!dupe && homeCount() < eco.maxPets()) o.home = true;
    return { pet: pet, rarity: r, dupe: dupe, refund: refund };
  }
  function homeCount() { var n = 0; for (var k in state.pets) if (state.pets[k].home) n++; return n; }
  // 抽：n = 1 或 10；free = 签到送的蛋（不扣钻）
  function pull(n, free) {
    var price = n === 10 ? D.GACHA.ten : D.GACHA.single;
    if (!free && !eco.spend({ gem: price }, 'gacha')) return null;
    var out = [];
    for (var i = 0; i < n; i++) out.push(one(n === 10 && i === 9 && !out.some(function (x) { return x.rarity === 'SR' || x.rarity === 'SSR'; }) ? 'SR' : null));
    eco.stat('gacha', n);
    bus.emit('gacha', n);
    bus.emit('pets');
    return out;
  }
  function setHome(id, home) {
    var o = state.pets[id];
    if (!o || o.n <= 0) return false;
    if (home && !o.home && homeCount() >= eco.maxPets()) return false;
    o.home = home;
    bus.emit('pets'); bus.emit('res');
    return true;
  }
  return { pull: pull, rates: rates, setHome: setHome, homeCount: homeCount, pityLeft: function () { return D.GACHA.pity - state.pity; } };
}

module.exports = { make: make };
