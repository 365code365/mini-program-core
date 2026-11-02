// 把 <p> 段落拆成数组，每段交给一个 rich-text 渲染。
// 原因：本引擎的 rich-text 把 <p> / <div> / <br> 都排成同一行（块级不换段），一段一个 rich-text 才能分段显示。
function paras(html) {
  return String(html || '').split(/<\/p>/i).map(s => s.replace(/<p[^>]*>/i, '').trim()).filter(Boolean).map(s => '<p>' + s + '</p>');
}
module.exports = { paras };
