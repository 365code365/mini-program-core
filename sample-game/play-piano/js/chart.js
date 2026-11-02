// 曲谱 → 谱面：每个旋律音一块黑块（占 [start, start + dur) 这几格），伴奏音成一串排程事件。
// 曲子弹完接着从头再来一遍（中间空 2 格），每一遍比上一遍快：game.js 在快到谱面末尾时调 append。
// 黑块分到哪条轨按旋律走向：音往上走就往右挪、往下走就往左挪，同音换一条；相邻两块永远不在同一条轨
// （上下紧挨着的两块同轨看起来像一块长条，点一下只算一块，很容易误判成漏按）。轨道用固定种子的随机数，同一首每次一样。
var P = require('./piano.js');

var SOLFA = ['do', '#do', 're', '#re', 'mi', 'fa', '#fa', 'sol', '#sol', 'la', '#la', 'si'];

function parse(str, def) {
  return str.trim().split(/\s+/).map(function (tok) {
    var p = tok.split(':'), d = p[1] ? Number(p[1]) : def;
    return { midi: p[0] === 'R' ? -1 : P.midiOf(p[0]), dur: d };
  });
}
function rng(seed) {
  var s = seed >>> 0;
  return function () {
    s = (s + 0x6D2B79F5) >>> 0;
    var t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
function hash(s) { var h = 2166136261; for (var i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619); return h >>> 0; }

function Chart(song) {
  this.song = song;
  this.mel = parse(song.melody, song.def);
  this.bas = parse(song.bass, song.def);
  this.len = this.mel.reduce(function (a, n) { return a + n.dur; }, 0);
  this.tiles = []; this.bass = [];
  this.loops = 0; this.end = 0; this.lane = 1; this.lastMidi = -1;
  this.rand = rng(hash(song.id));
}
// 追加一遍：返回这一遍开头的格位
Chart.prototype.append = function () {
  var at = this.loops ? this.end + 2 : 0, u = at, self = this;
  this.mel.forEach(function (n) {
    if (n.midi >= 0) {
      var lane = self.pick(n.midi);
      self.tiles.push({ start: u, dur: n.dur, midi: n.midi, lane: lane, label: SOLFA[n.midi % 12], hit: false, hitAt: 0, loop: self.loops });
    }
    u += n.dur;
  });
  var b = at;
  this.bas.forEach(function (n) { if (n.midi >= 0) self.bass.push({ at: b, midi: n.midi }); b += n.dur; });
  this.end = at + this.len;
  this.loops++;
  return at;
};
Chart.prototype.pick = function (midi) {
  var prev = this.lane, r = this.rand(), lane;
  if (this.lastMidi < 0 || midi === this.lastMidi) {
    lane = (prev + 1 + Math.floor(r * 3)) % 4;                 // 换到另外三条里的一条
  } else {
    var step = r < 0.65 ? 1 : 2, dir = midi > this.lastMidi ? 1 : -1;
    lane = prev + dir * step;
    if (lane < 0 || lane > 3) lane = prev - dir * step;          // 到边了就折回来
    if (lane < 0 || lane > 3) lane = prev === 0 ? 1 : prev - 1;
  }
  this.lane = lane; this.lastMidi = midi;
  return lane;
};
// 用到的全部音高（预合成用）
Chart.notesOf = function (song) {
  var seen = {}, out = [];
  parse(song.melody, song.def).concat(parse(song.bass, song.def)).forEach(function (n) {
    if (n.midi >= 0 && !seen[n.midi]) { seen[n.midi] = 1; out.push(n.midi); }
  });
  return out;
};

module.exports = Chart;
