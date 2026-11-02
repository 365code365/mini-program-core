// 内置曲谱（全部是公版曲目）。每首两条声部：melody 是落下来要弹的黑块，bass 是按音频时钟提前排好的伴奏。
// 记法：「音名+八度[:时值]」，时值以 unit 为单位（不写就是 def）；R 是休止。
// unit 是一格的时长：ups = 每秒几格（未加速时）。八分音符记谱的 unit = 八分音符，致爱丽丝用十六分音符。
// 黑块的高度 = 时值 × 每格像素，所以长音就是长块，休止就是两块之间的空当：踩准了弹出来就是原曲的节奏。
module.exports = [
  {
    id: 'twinkle', name: '小星星', by: '法国童谣 · 莫扎特变奏主题', stars: 1, ups: 3.6, def: 2,
    melody:
      'C4 C4 G4 G4 A4 A4 G4:4 F4 F4 E4 E4 D4 D4 C4:4 ' +
      'G4 G4 F4 F4 E4 E4 D4:4 G4 G4 F4 F4 E4 E4 D4:4 ' +
      'C4 C4 G4 G4 A4 A4 G4:4 F4 F4 E4 E4 D4 D4 C4:4',
    bass:
      'C3:4 E3:4 F3:4 C3:4 F3:4 C3:4 G2:4 C3:4 ' +
      'C3:4 F3:4 C3:4 G2:4 C3:4 F3:4 C3:4 G2:4 ' +
      'C3:4 E3:4 F3:4 C3:4 F3:4 C3:4 G2:4 C3:4'
  },
  {
    id: 'tigers', name: '两只老虎', by: '法国民歌《雅克兄弟》', stars: 1, ups: 4.2, def: 2,
    melody:
      'C4 D4 E4 C4 C4 D4 E4 C4 E4 F4 G4:4 E4 F4 G4:4 ' +
      'G4:1 A4:1 G4:1 F4:1 E4 C4 G4:1 A4:1 G4:1 F4:1 E4 C4 C4 G3 C4:4 C4 G3 C4:4',
    bass:
      'C3:4 G2:4 C3:4 G2:4 C3:4 C3:4 C3:4 C3:4 ' +
      'C3:4 G2:4 C3:4 G2:4 C3:4 G2:4 C3:4 G2:4'
  },
  {
    id: 'ode', name: '欢乐颂', by: '贝多芬《第九交响曲》', stars: 2, ups: 4, def: 2,
    melody:
      'E4 E4 F4 G4 G4 F4 E4 D4 C4 C4 D4 E4 E4:3 D4:1 D4:4 ' +
      'E4 E4 F4 G4 G4 F4 E4 D4 C4 C4 D4 E4 D4:3 C4:1 C4:4 ' +
      'D4 D4 E4 C4 D4 E4:1 F4:1 E4 C4 D4 E4:1 F4:1 E4 D4 C4 D4 G3:4 ' +
      'E4 E4 F4 G4 G4 F4 E4 D4 C4 C4 D4 E4 D4:3 C4:1 C4:4',
    bass:
      'C3:4 C3:4 G2:4 G2:4 C3:4 C3:4 C3:4 G2:4 ' +
      'C3:4 C3:4 G2:4 G2:4 C3:4 C3:4 G2:4 C3:4 ' +
      'G2:4 C3:4 G2:4 C3:4 G2:4 C3:4 C3:4 G2:4 ' +
      'C3:4 C3:4 G2:4 G2:4 C3:4 C3:4 G2:4 C3:4'
  },
  {
    id: 'jingle', name: '铃儿响叮当', by: '詹姆斯·皮尔彭特', stars: 2, ups: 4.4, def: 2,
    melody:
      'E4 E4 E4:4 E4 E4 E4:4 E4 G4 C4:3 D4:1 E4:8 ' +
      'F4 F4 F4:3 F4:1 F4 E4 E4 E4:1 E4:1 E4 D4 D4 E4 D4:4 G4:4 ' +
      'E4 E4 E4:4 E4 E4 E4:4 E4 G4 C4:3 D4:1 E4:8 ' +
      'F4 F4 F4 F4 F4 E4 E4 E4:1 E4:1 G4 G4 F4 D4 C4:8',
    bass:
      'C3:4 C3:4 C3:4 C3:4 C3:4 C3:4 C3:4 G2:4 ' +
      'F3:4 F3:4 C3:4 C3:4 G2:4 D3:4 G2:4 G2:4 ' +
      'C3:4 C3:4 C3:4 C3:4 C3:4 C3:4 C3:4 G2:4 ' +
      'F3:4 F3:4 C3:4 C3:4 G2:4 G2:4 C3:8'
  },
  {
    id: 'canon', name: 'D 大调卡农', by: '帕赫贝尔', stars: 3, ups: 3.4, def: 2,
    melody:
      'F#5 E5 D5 C#5 B4 A4 B4 C#5 D5 C#5 B4 A4 G4 F#4 G4 E4 ' +
      'D4:1 F#4:1 A4:1 G4:1 F#4:1 D4:1 F#4:1 E4:1 D4:1 B3:1 D4:1 A4:1 G4:1 B4:1 A4:1 G4:1 ' +
      'F#4:1 D4:1 E4:1 C#5:1 D5:1 F#5:1 A5:1 A4:1 B4:1 G4:1 A4:1 F#4:1 D4:4',
    bass:
      'D3 A2 B2 F#2 G2 D2 G2 A2 D3 A2 B2 F#2 G2 D2 G2 A2 ' +
      'D3 A2 B2 F#2 G2 D2 G2 A2 D3 A2 B2 F#2 G2 D2 G2 A2'
  },
  {
    id: 'elise', name: '致爱丽丝', by: '贝多芬', stars: 4, ups: 5.6, def: 1,
    melody:
      'E5 D#5 E5 D#5 E5 B4 D5 C5 A4:2 R C4 E4 A4 B4:2 R E4 G#4 B4 C5:2 R E4 ' +
      'E5 D#5 E5 D#5 E5 B4 D5 C5 A4:2 R C4 E4 A4 B4:2 R E4 C5 B4 A4:4 R:2',
    bass:
      'R:8 A2 E3 A3 R:3 E2 E3 G#3 R:3 A2 E3 A3 R:3 ' +
      'R:6 A2 E3 A3 R:3 E2 E3 G#3 R:3 A2 E3 A3 R:3'
  }
];
