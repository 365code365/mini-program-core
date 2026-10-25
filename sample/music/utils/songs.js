// 曲库与歌单（本地数据，不联网）。封面是两色渐变，由 c1 / c2 决定
const LINES = [
  '风吹过街角的旧唱片', '你说晚霞会记得我们', '把心事折成一只纸船', '顺着河流漂向远方',
  '路灯一盏一盏亮起', '影子被拉得好长好长', '我在人海里慢慢走', '想起那年夏天的蝉鸣',
  '雨落在窗台的声音', '像一首没写完的诗', '我们都曾奔跑过', '在没有尽头的操场',
  '如果时间可以停下', '就停在你笑的那一刻', '星星落进了海里', '浪花替我说晚安',
  '明天会是晴天吧', '我把歌唱给你听', '温柔是最长的告白', '这一路有你真好'
]
const RAW = [
  [1, '晚风告白', '林夏', '夏日来信', 'zh', 196, '#ff9a8b', '#ff6a88'],
  [2, '城市漫游', '陈一舟', '漫游者', 'zh', 214, '#4facfe', '#00c6fb'],
  [3, '纸船', '苏晚', '纸船', 'zh', 183, '#43e97b', '#38f9d7'],
  [4, '海边的信', '林夏', '夏日来信', 'zh', 228, '#fa709a', '#fee140'],
  [5, 'Midnight Train', 'The Lanterns', 'Night Lines', 'en', 205, '#30cfd0', '#330867'],
  [6, 'Golden Hour', 'Ivy Moon', 'Sunsets', 'en', 191, '#f6d365', '#fda085'],
  [7, 'Paper Planes', 'Ivy Moon', 'Sunsets', 'en', 176, '#a18cd1', '#fbc2eb'],
  [8, '雨后清晨', '白噪声乐团', '呼吸', 'light', 240, '#89f7fe', '#66a6ff'],
  [9, '森林小径', '白噪声乐团', '呼吸', 'light', 252, '#d4fc79', '#96e6a1'],
  [10, '咖啡馆午后', '慢半拍', '午后', 'light', 233, '#e6b980', '#eacda3'],
  [11, '星河', '陈一舟', '漫游者', 'zh', 219, '#5f72bd', '#9b23ea'],
  [12, 'Ocean Eyes', 'Blue Harbor', 'Tides', 'en', 200, '#13547a', '#80d0c7'],
  [13, '旧时光', '苏晚', '纸船', 'zh', 187, '#c79081', '#dfa579'],
  [14, '月光钢琴', '慢半拍', '午后', 'light', 210, '#6a85b6', '#bac8e0'],
  [15, 'Run Away', 'The Lanterns', 'Night Lines', 'en', 198, '#ff0844', '#ffb199'],
  [16, '晴天日记', '林夏', '夏日来信', 'zh', 205, '#fddb92', '#d1fdff']
]
// 每行歌词 6 秒；第 0 行是歌名、第 1 行是作者
const LINE_SEC = 6
function makeLyric(id, name, artist, duration) {
  const n = Math.ceil(duration / LINE_SEC)
  const out = [{ t: 0, text: name + ' - ' + artist }, { t: LINE_SEC, text: '词：' + artist + '  曲：' + artist }]
  for (let i = 2; i < n; i++) out.push({ t: i * LINE_SEC, text: LINES[(i * 7 + id * 3) % LINES.length] })
  return out
}
const SONGS = RAW.map(r => ({
  id: r[0], name: r[1], artist: r[2], album: r[3], lang: r[4], duration: r[5], c1: r[6], c2: r[7],
  initial: r[1].charAt(0), durText: fmt(r[5])
}))
const BY_ID = {}
SONGS.forEach(s => { BY_ID[s.id] = s })
const LANGS = [
  { key: 'all', text: '全部' },
  { key: 'zh', text: '华语' },
  { key: 'en', text: '欧美' },
  { key: 'light', text: '轻音乐' }
]
const PLAYLISTS = [
  { id: 'p1', name: '夏日晚风', desc: '傍晚散步时听的温柔华语', ids: [1, 4, 3, 16, 13, 2, 11], plays: '12.6万', c1: '#ff9a8b', c2: '#ff6a88' },
  { id: 'p2', name: '深夜列车', desc: '一个人的夜路，耳机里的欧美独立', ids: [5, 12, 15, 6, 7], plays: '8.3万', c1: '#30cfd0', c2: '#330867' },
  { id: 'p3', name: '专注白噪音', desc: '学习、工作时不打扰的轻音乐', ids: [8, 9, 10, 14], plays: '21.9万', c1: '#89f7fe', c2: '#66a6ff' },
  { id: 'p4', name: '私藏华语', desc: '编辑精选：不容错过的华语新歌', ids: [2, 11, 13, 3, 1, 16, 4], plays: '5.1万', c1: '#5f72bd', c2: '#9b23ea' },
  { id: 'p5', name: '午后咖啡', desc: '慵懒的下午，来一杯拿铁', ids: [10, 14, 6, 7, 8], plays: '3.7万', c1: '#e6b980', c2: '#eacda3' },
  { id: 'p6', name: '全部单曲', desc: '曲库里的每一首歌', ids: SONGS.map(s => s.id), plays: '30.2万', c1: '#f6d365', c2: '#fda085' }
]
function pad(n) {
  return n < 10 ? '0' + n : '' + n
}
function fmt(sec) {
  sec = Math.max(0, Math.floor(sec))
  return pad(Math.floor(sec / 60)) + ':' + pad(sec % 60)
}
function song(id) {
  return BY_ID[id] || null
}
function lyric(id) {
  const s = BY_ID[id]
  return s ? makeLyric(s.id, s.name, s.artist, s.duration) : []
}
// 当前进度落在第几行歌词
function lineAt(sec) {
  return Math.max(0, Math.floor(sec / LINE_SEC))
}
function playlist(id) {
  return PLAYLISTS.find(p => p.id === id) || null
}
module.exports = { SONGS, LANGS, PLAYLISTS, LINE_SEC, song, lyric, lineAt, playlist, fmt }
