// 管理页：登录态走 HttpOnly Cookie；改数据的请求带 X-Mini-Store 头（服务器据此挡 CSRF）
'use strict';
const $ = (id) => document.getElementById(id);
const CATEGORIES = ['美食', '购物', '出行', '生活服务', '医疗健康', '金融', '工具', '教育', '娱乐', '资讯', '政务', '其他'];
for (const id of ['ucat', 'fcat', 'ecat']) {
  for (const c of CATEGORIES) $(id).append(new Option(c, c));
}
const COLORS = ['#07c160', '#10aeff', '#fa9d3b', '#e64340', '#6467f0', '#1485ee', '#91d300', '#576b95'];

async function api(method, path, body, headers = {}) {
  const res = await fetch(path, { method, body, headers: { 'X-Mini-Store': '1', ...headers }, credentials: 'same-origin' });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw Object.assign(new Error(data.error || `HTTP ${res.status}`), { status: res.status });
  return data;
}

// ── 通用：提示条、确认框、页面切换 ──
function toast(text, err = false) {
  const t = document.createElement('div');
  t.className = err ? 'toast err' : 'toast';
  t.textContent = text;
  $('toast').append(t);
  setTimeout(() => t.remove(), 2600);
}

function fail(err) {
  if (err.status === 401) { show('login'); return; }
  toast(err.message, true);
}

function confirmDanger(title, text) {
  const dlg = $('confirmDlg');
  $('confirmTitle').textContent = title;
  $('confirmText').textContent = text;
  dlg.returnValue = '';
  dlg.showModal();
  return new Promise((resolve) => { dlg.onclose = () => resolve(dlg.returnValue === 'ok'); });
}

const VIEWS = ['overview', 'apps', 'upload', 'diag'];
function route() {
  const v = VIEWS.includes(location.hash.slice(1)) ? location.hash.slice(1) : 'overview';
  for (const name of VIEWS) $(`view-${name}`).classList.toggle('hidden', name !== v);
  for (const a of document.querySelectorAll('nav a')) a.classList.toggle('on', a.dataset.view === v);
  $('crumb').textContent = $(`view-${v}`).dataset.title;
}
window.addEventListener('hashchange', route);
for (const b of document.querySelectorAll('[data-go]')) b.addEventListener('click', () => { location.hash = b.dataset.go; });

function show(which) {
  $('login').classList.toggle('hidden', which !== 'login');
  $('app').classList.toggle('hidden', which !== 'app');
}

async function boot() {
  try {
    const me = await api('GET', '/api/admin/me');
    $('who').textContent = me.username;
    $('avatar').textContent = ([...(me.username || 'A')][0] || 'A').toUpperCase();
    show('app');
    route();
    refresh().catch(fail);
    refreshDiag().catch(fail);
  } catch (_) {
    show('login');
  }
}

$('loginForm').addEventListener('submit', async (e) => {
  e.preventDefault();
  $('loginMsg').textContent = '';
  try {
    await api('POST', '/api/login', JSON.stringify({ username: $('lu').value, password: $('lp').value }),
      { 'Content-Type': 'application/json' });
    $('lp').value = '';
    boot();
  } catch (err) {
    $('loginMsg').textContent = err.message;
  }
});

$('logout').addEventListener('click', async () => {
  await api('POST', '/api/logout').catch(() => {});
  show('login');
});

// ── 上传 ──
function pickedText() {
  if ($('srcDir').checked) {
    const files = $('udir').files;
    return files.length ? `已选文件夹「${files[0].webkitRelativePath.split('/')[0]}」，${files.length} 个文件` : '';
  }
  const f = $('uzip').files[0];
  return f ? `已选 ${f.name}（${(f.size / 1048576).toFixed(2)} MB）` : '';
}

function syncSrc() {
  const dir = $('srcDir').checked;
  $('udir').classList.toggle('hidden', !dir);
  $('uzip').classList.toggle('hidden', dir);
  $('dropT1').textContent = dir ? '点击或拖入小程序文件夹' : '点击或拖入 zip 文件';
  $('dropT2').textContent = dir ? '选含 app.json 的那一层；会在浏览器里打成 zip 再上传' : 'app.json 在 zip 根上或唯一一层子目录里';
  $('picked').textContent = pickedText();
}
for (const r of document.querySelectorAll('input[name=src]')) r.addEventListener('change', syncSrc);

const drop = $('drop');
for (const ev of ['dragenter', 'dragover']) drop.addEventListener(ev, () => drop.classList.add('over'));
for (const ev of ['dragleave', 'drop']) drop.addEventListener(ev, () => drop.classList.remove('over'));
$('uzip').addEventListener('change', syncSrc);

// 选了文件夹：ID 没填就用文件夹名猜一个
$('udir').addEventListener('change', () => {
  const f = $('udir').files[0];
  if (f && !$('uid').value) {
    $('uid').value = f.webkitRelativePath.split('/')[0].toLowerCase().replace(/[^a-z0-9._-]+/g, '-').replace(/^-+/, '');
  }
  syncSrc();
});

$('ureset').addEventListener('click', () => {
  $('uploadForm').reset();
  $('uploadMsg').textContent = '';
  syncSrc();
});

$('uploadForm').addEventListener('submit', async (e) => {
  e.preventDefault();
  const msg = $('uploadMsg');
  msg.className = 'msg';
  let blob;
  try {
    if ($('srcDir').checked) {
      const files = [...$('udir').files];
      if (!files.length) throw new Error('先选文件夹');
      msg.textContent = `打包 ${files.length} 个文件…`;
      blob = await buildZip(files);
    } else {
      blob = $('uzip').files[0];
      if (!blob) throw new Error('先选 zip');
    }
    $('ubtn').disabled = true;
    msg.textContent = `上传 ${(blob.size / 1048576).toFixed(1)} MB…`;
    const q = new URLSearchParams({ name: $('uname').value.trim(), description: $('udesc').value.trim() });
    for (const [k, id] of [['category', 'ucat'], ['tags', 'utags'], ['developer', 'udev']]) {
      if ($(id).value.trim()) q.set(k, $(id).value.trim());
    }
    const meta = await api('PUT', `/api/admin/apps/${encodeURIComponent($('uid').value.trim())}?${q}`, blob,
      { 'Content-Type': 'application/zip' });
    msg.className = 'msg ok';
    msg.textContent = `已上架「${meta.name}」v${meta.version}（${meta.files} 个文件，${meta.pages} 页）`;
    toast(`已上架「${meta.name}」v${meta.version}`);
    refresh().catch(fail);
  } catch (err) {
    msg.className = 'msg err';
    msg.textContent = err.message;
    if (err.status === 401) show('login');
  } finally {
    $('ubtn').disabled = false;
  }
});

// ── 列表：搜索框与分类走服务器的 /api/apps?q=&category=（与 App 同一套排序），空着就是全部 ──
let searchTimer = 0;
$('q').addEventListener('input', () => { clearTimeout(searchTimer); searchTimer = setTimeout(() => refresh().catch(fail), 200); });
$('fcat').addEventListener('change', () => refresh().catch(fail));
$('reloadApps').addEventListener('click', () => refresh().then(() => toast('已刷新'), fail));
$('reloadDiag').addEventListener('click', () => refreshDiag().then(() => toast('已刷新'), fail));

function appIcon(a) {
  let icon;
  if (a.iconUrl) {
    icon = document.createElement('img');
    icon.src = a.iconUrl;
    icon.alt = '';
  } else {
    icon = document.createElement('span');
    icon.style.background = COLORS[hash(a.id) % COLORS.length];
    icon.textContent = [...a.name][0] || '?';
  }
  icon.className = 'icon';
  return icon;
}

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

const fmtTime = (t) => new Date(t).toLocaleString('zh-CN', { hour12: false });

function renderOverview(all, cats, hot) {
  $('stApps').textContent = all.length;
  $('stCats').textContent = cats.length;
  $('stDl').textContent = all.reduce((n, a) => n + (a.downloads || 0), 0).toLocaleString();
  $('navApps').textContent = all.length || '';
  const max = Math.max(1, ...cats.map((c) => c.count));
  $('catBars').replaceChildren(...(cats.length ? cats.map((c) => {
    const row = el('div', 'bar');
    const track = el('div', 'track');
    const fill = el('div', 'fill');
    fill.style.width = `${(c.count / max) * 100}%`;
    track.append(fill);
    row.append(el('span', '', c.name), track, el('span', 'n', String(c.count)));
    return row;
  }) : [el('div', 'empty', '暂无数据')]));
  const recent = [...all].sort((a, b) => new Date(b.updatedAt) - new Date(a.updatedAt)).slice(0, 5);
  $('recent').replaceChildren(...(recent.length ? recent.map((a) => {
    const li = el('li');
    const meta = el('div', 'meta');
    meta.append(el('div', 'name', a.name), el('div', 'id', `${a.id} · v${a.version}`));
    li.append(appIcon(a), meta, el('span', 't', fmtTime(a.updatedAt)));
    return li;
  }) : [el('li', 'empty', '暂无小程序')]));
  $('hot').replaceChildren(...(hot.length ? hot.map((w, i) => {
    const b = btn('', () => { $('q').value = w; location.hash = 'apps'; return refresh(); });
    b.append(el('span', 'rk', String(i + 1)), document.createTextNode(w));
    return b;
  }) : [el('span', 'help', '暂无热搜')]));
}

async function refresh() {
  const q = new URLSearchParams({ q: $('q').value.trim(), category: $('fcat').value, limit: '200' });
  const [{ apps }, all, cats, hot] = await Promise.all([
    api('GET', `/api/apps?${q}`),
    api('GET', '/api/apps'),
    api('GET', '/api/categories'),
    api('GET', '/api/search/hot'),
  ]);
  renderOverview(all.apps, cats.categories, hot.keywords);
  $('count').textContent = `共 ${all.apps.length} 个${apps.length !== all.apps.length ? `，筛选出 ${apps.length} 个` : ''}`;
  $('emptyApps').classList.toggle('hidden', apps.length > 0);
  $('list').replaceChildren(...apps.map((a) => {
    const tr = document.createElement('tr');
    const cell = el('div', 'appcell');
    const info = el('div');
    const name = el('div', 'name');
    name.append(el('span', '', a.name), el('span', 'cat', a.category || '其他'));
    info.append(name, el('div', 'id', `${a.id}${a.appId ? ` · ${a.appId}` : ''}`));
    if (a.description) info.append(el('div', 'desc-t', a.description));
    if ((a.tags || []).length) {
      const tags = el('div', 'tags');
      for (const t of a.tags) tags.append(el('span', 'tag', t));
      info.append(tags);
    }
    cell.append(appIcon(a), info);
    const ops = el('td', 'ops');
    ops.append(
      btn('编辑', () => edit(a), 'link'),
      btn('下载', () => { location.href = a.packageUrl; }, 'link'),
      btn('删除', () => remove(a), 'link danger'),
    );
    tr.append(td(cell), td(a.developer || '-', 'muted'), td(el('span', 'ver', `v${a.version}`)),
      td(`${(a.size / 1048576).toFixed(2)} MB`, 'num'), td(String(a.downloads || 0), 'num'), td(fmtTime(a.updatedAt), 'muted'), ops);
    return tr;
  }));
}

async function refreshDiag() {
  const { items } = await api('GET', '/api/admin/diag');
  $('diagCount').textContent = `共 ${items.length} 份`;
  $('stDiag').textContent = items.length;
  $('navDiag').textContent = items.length || '';
  $('emptyDiag').classList.toggle('hidden', items.length > 0);
  $('diagList').replaceChildren(...items.map((d) => {
    const tr = document.createElement('tr');
    const app = d.app || {};
    const ops = el('td', 'ops');
    ops.append(
      btn('下载', async () => {
        const full = await api('GET', `/api/admin/diag/${d.id}`);
        const a = document.createElement('a');
        a.href = URL.createObjectURL(new Blob([JSON.stringify(full, null, 1)], { type: 'application/json' }));
        a.download = `${d.id}.json`;
        a.click();
      }, 'link'),
      btn('删除', async () => {
        if (!await confirmDanger('删除诊断日志', `删除诊断 ${d.id}？`)) return;
        await api('DELETE', `/api/admin/diag/${d.id}`);
        toast('已删除');
        await refreshDiag();
      }, 'link danger'),
    );
    const idc = el('div');
    idc.append(el('div', 'id', d.id), el('div', 'help', fmtTime(d.receivedAt)));
    const dev = el('div');
    dev.append(el('div', '', [d.host, d.platform].filter(Boolean).join(' · ') || '-'), el('div', 'help', d.device || ''));
    tr.append(td(idc), td(dev), td(app.id || app.appId || app.name || '-'),
      td(`${d.logLines} 行 / ${(d.size / 1024).toFixed(0)} KB`, 'num'), td(d.note || '', 'pre'), ops);
    return tr;
  }));
}

function edit(a) {
  $('ename').value = a.name;
  $('edesc').value = a.description || '';
  $('ecat').value = a.category || '其他';
  $('etags').value = (a.tags || []).join(', ');
  $('edev').value = a.developer || '';
  $('eorder').value = String(a.order ?? 1000);
  const dlg = $('editDlg');
  dlg.returnValue = '';
  dlg.onclose = async () => {
    if (dlg.returnValue !== 'ok') return;
    const body = {
      name: $('ename').value.trim(), description: $('edesc').value.trim(), category: $('ecat').value,
      tags: $('etags').value, developer: $('edev').value.trim(),
    };
    const order = parseInt($('eorder').value, 10);
    if (Number.isFinite(order)) body.order = order;
    try {
      await api('PATCH', `/api/admin/apps/${a.id}`, JSON.stringify(body), { 'Content-Type': 'application/json' });
      toast('已保存');
      await refresh();
    } catch (err) {
      fail(err);
    }
  };
  dlg.showModal();
}

async function remove(a) {
  if (!await confirmDanger('删除小程序', `删除「${a.name}」（${a.id}）？客户端已下载的不受影响，列表里不再出现。`)) return;
  await api('DELETE', `/api/admin/apps/${a.id}`);
  toast(`已删除「${a.name}」`);
  await refresh();
}

function td(content, cls = '') {
  const e = document.createElement('td');
  if (cls) e.className = cls;
  if (typeof content === 'string') e.textContent = content; else e.append(content);
  return e;
}

function btn(text, onClick, cls = '') {
  const b = document.createElement('button');
  b.type = 'button';
  b.textContent = text;
  b.className = cls;
  b.addEventListener('click', () => Promise.resolve().then(onClick).catch(fail));
  return b;
}

function hash(s) {
  let h = 0;
  for (const c of s) h = (h * 31 + c.codePointAt(0)) >>> 0;
  return h;
}

// ── 浏览器里打 zip（STORE，不压缩：服务器收到后会重新 deflate） ──
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

async function buildZip(files) {
  const enc = new TextEncoder();
  const parts = [];
  const central = [];
  let offset = 0;
  for (const f of files) {
    const name = f.webkitRelativePath || f.name;
    const base = name.split('/').pop();
    if (base === '.DS_Store' || base.startsWith('._') || name.split('/').includes('node_modules')) continue;
    const data = new Uint8Array(await f.arrayBuffer());
    const nameBytes = enc.encode(name);
    const crc = crc32(data);
    // 本地文件头：通用标志 0x0800 = 文件名是 UTF-8（中文文件名）
    const h = new DataView(new ArrayBuffer(30));
    h.setUint32(0, 0x04034b50, true); h.setUint16(4, 20, true); h.setUint16(6, 0x0800, true);
    h.setUint16(8, 0, true); h.setUint32(10, 0x00210000, true); h.setUint32(14, crc, true);
    h.setUint32(18, data.length, true); h.setUint32(22, data.length, true);
    h.setUint16(26, nameBytes.length, true); h.setUint16(28, 0, true);
    parts.push(h.buffer, nameBytes, data);
    const c = new DataView(new ArrayBuffer(46));
    c.setUint32(0, 0x02014b50, true); c.setUint16(4, 20, true); c.setUint16(6, 20, true);
    c.setUint16(8, 0x0800, true); c.setUint16(10, 0, true); c.setUint32(12, 0x00210000, true);
    c.setUint32(16, crc, true); c.setUint32(20, data.length, true); c.setUint32(24, data.length, true);
    c.setUint16(28, nameBytes.length, true); c.setUint32(42, offset, true);
    central.push(c.buffer, nameBytes);
    offset += 30 + nameBytes.length + data.length;
  }
  const cdSize = central.reduce((n, b) => n + b.byteLength, 0);
  const end = new DataView(new ArrayBuffer(22));
  end.setUint32(0, 0x06054b50, true);
  end.setUint16(8, central.length / 2, true); end.setUint16(10, central.length / 2, true);
  end.setUint32(12, cdSize, true); end.setUint32(16, offset, true);
  return new Blob([...parts, ...central, end.buffer], { type: 'application/zip' });
}

boot();
