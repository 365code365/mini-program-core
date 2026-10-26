// 官网的两块动态内容：示例截图（固定列表）、下载区（deploy.sh 生成的 downloads.json：文件名、大小、sha256、时间）
(function () {
  var shots = [
    ['movie-home.jpg', '光影放映厅 · 首页'], ['movie-detail.jpg', '光影放映厅 · 详情'],
    ['music-home.jpg', '云听音乐 · 首页'], ['music-playlist.jpg', '云听音乐 · 歌单'],
    ['photo-wall.jpg', '拾光图库 · 瀑布流'], ['mini-hub.jpg', '小程序跳转中心'],
    ['28_ecommerce_home.jpg', '电商首页'], ['36_news_feed.jpg', '资讯信息流'],
    ['41_live_shopping.jpg', '直播带货'], ['51_health_dashboard.jpg', '健康看板'],
  ];
  var box = document.getElementById('shots');
  shots.forEach(function (s) {
    var d = document.createElement('div');
    d.className = 'shot';
    var img = document.createElement('img');
    img.src = 'shots/' + s[0];
    img.alt = s[1];
    img.loading = 'lazy';
    var p = document.createElement('p');
    p.textContent = s[1];
    d.append(img, p);
    box.append(d);
  });

  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }
  var dl = document.getElementById('downloads');
  fetch('downloads.json', { cache: 'no-store' }).then(function (r) { return r.json(); }).then(function (d) {
    dl.replaceChildren.apply(dl, d.files.map(function (f) {
      var card = el('article', 'dl');
      var head = el('header');
      var icon = el('img');
      icon.src = f.icon;
      icon.alt = '';
      var t = el('div');
      t.append(el('h3', '', f.title), el('div', 'meta', 'v' + f.version + ' · ' + (f.size / 1048576).toFixed(1) + ' MB · ' + f.date));
      head.append(icon, t);
      var a = el('a', 'btn primary', '下载 ' + f.file);
      a.href = 'download/' + encodeURIComponent(f.file);
      a.setAttribute('download', f.file);
      card.append(head, el('p', 'note', f.note), a, el('div', 'sha', 'SHA-256 ' + f.sha256));
      return card;
    }));
  }).catch(function () {
    dl.replaceChildren(el('p', 'muted', '版本信息读取失败，请刷新重试'));
  });
})();
