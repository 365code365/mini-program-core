// 词书目录：词表放在 utils/books/<id>.js（[单词, 音标, 释义, 例句, 例句译文]）
const RAW = {
  cet4: require('./books/cet4.js'),
  cet6: require('./books/cet6.js'),
  kaoyan: require('./books/kaoyan.js'),
  ielts: require('./books/ielts.js')
};

const BOOKS = [
  { id: 'cet4', name: '四级核心词', full: '大学英语四级高频词', cover: '/images/covers/cet4.jpg', level: '大学 · 四级',
    desc: '按近十年真题词频精选，先背出现最多的那批，阅读与听力里最常碰到。', learners: '86.2 万人在学' },
  { id: 'cet6', name: '六级核心词', full: '大学英语六级核心词', cover: '/images/covers/cet6.jpg', level: '大学 · 六级',
    desc: '六级写作与翻译的高分动词、形容词，配真题风格例句。', learners: '41.7 万人在学' },
  { id: 'kaoyan', name: '考研大纲词', full: '考研英语大纲词汇', cover: '/images/covers/kaoyan.jpg', level: '考研 · 英语一 / 二',
    desc: '大纲里阅读理解最容易卡住的抽象词，例句取自学术语境。', learners: '33.5 万人在学' },
  { id: 'ielts', name: '雅思话题词', full: '雅思话题词汇', cover: '/images/covers/ielts.jpg', level: '出国 · 雅思',
    desc: '环境、教育、城市、健康四大高频话题，写作口语直接用得上。', learners: '18.9 万人在学' }
];

const cache = {};
function words(id) {
  if (!cache[id]) {
    cache[id] = (RAW[id] || []).map((x) => ({ w: x[0], ph: x[1], mean: x[2], ex: x[3], exZh: x[4], book: id }));
  }
  return cache[id];
}
function book(id) { return BOOKS.find((b) => b.id === id) || BOOKS[0]; }
let index = null;
function find(w) {
  if (!index) {
    index = {};
    BOOKS.forEach((b) => words(b.id).forEach((x) => { index[x.w] = x; }));
  }
  return index[w] || null;
}

module.exports = { BOOKS, words, book, find };
