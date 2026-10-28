// 保龄球计分（十局制，纯逻辑）：rolls 是每一投倒的瓶数。
// 全中（X）= 10 + 后两投；补中（/）= 10 + 后一投；第 10 局全中 / 补中再加投（最多 3 投）。
function frames(rolls) {
  var out = [], i = 0, total = 0;
  for (var f = 0; f < 10; f++) {
    var r1 = rolls[i], r2 = rolls[i + 1], r3 = rolls[i + 2];
    if (r1 === undefined) break;
    var fr = { marks: [], score: null };
    if (f < 9) {
      if (r1 === 10) {
        fr.marks = ['X'];
        if (r2 !== undefined && r3 !== undefined) fr.score = (total += 10 + r2 + r3);
        i += 1;
      } else {
        fr.marks = [mark(r1), r2 === undefined ? '' : r1 + r2 === 10 ? '/' : mark(r2)];
        if (r2 !== undefined && r1 + r2 === 10) { if (r3 !== undefined) fr.score = (total += 10 + r3); }
        else if (r2 !== undefined) fr.score = (total += r1 + r2);
        i += 2;
      }
    } else {
      // 第 10 局：照原样列出最多 3 投
      var a = rolls.slice(i, i + 3);
      fr.marks = a.map(function (r, k) {
        if (r === 10 && (k === 0 || a[k - 1] === 10 || (k === 2 && a[0] + a[1] === 10))) return 'X';
        if (k > 0 && a[k - 1] !== 10 && !(k === 2 && a[0] + a[1] === 10) && a[k - 1] + r === 10) return '/';
        return mark(r);
      });
      if (tenthDone(a)) fr.score = (total += a.reduce(function (s, r) { return s + r; }, 0));
      i += a.length;
    }
    out.push(fr);
  }
  return out;
}
function mark(r) { return r === 0 ? '-' : String(r); }
function tenthDone(a) { return a.length === 3 || (a.length === 2 && a[0] + a[1] < 10); }

// 当前在第几局（0 起）、这一局的第几投（0 起）、这一投之前站着几个瓶（要不要重摆）、整场打完没有
function cursor(rolls) {
  var i = 0;
  for (var f = 0; f < 9; f++) {
    if (i >= rolls.length) return { frame: f, ball: 0, fresh: true, done: false };
    if (rolls[i] === 10) { i += 1; continue; }
    if (i + 1 >= rolls.length) return { frame: f, ball: 1, fresh: false, done: false };
    i += 2;
  }
  var a = rolls.slice(i);
  if (tenthDone(a)) return { frame: 9, ball: a.length, fresh: false, done: true };
  // 第 10 局：全中或补中之后重摆
  var fresh = a.length === 0 || (a.length === 1 && a[0] === 10) || (a.length === 2 && (a[1] === 10 || a[0] + a[1] === 10) && !(a[0] === 10 && a[1] < 10));
  return { frame: 9, ball: a.length, fresh: fresh, done: false };
}
function total(rolls) { var fs = frames(rolls), t = 0; fs.forEach(function (f) { if (f.score !== null) t = f.score; }); return t; }

module.exports = { frames: frames, cursor: cursor, total: total };
