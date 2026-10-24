// 管理页：登录态走 HttpOnly Cookie；改数据的请求带 X-Mini-Store 头（服务器据此挡 CSRF）
'use strict';
const $ = (id) => document.getElementById(id);
const COLORS = ['#07c160', '#10aeff', '#fa9d3b', '#e64340', '#6467f0', '#1485ee', '#91d300', '#576b95'];

async function api(method, path, body, headers = {}) {
  const res = await fetch(path, { method, body, headers: { 'X-Mini-Store': '1', ...headers }, credentials: 'same-origin' });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw Object.assign(new Error(data.error || `HTTP ${res.status}`), { status: res.status });
  return data;
}

function show(which) {
  $('login').classList.toggle('hidden', which !== 'login');
  $('app').classList.toggle('hidden', which !== 'app');
}

async function boot() {
  try {
    const me = await api('GET', '/api/admin/me');
    $('who').textContent = me.username;
    show('app');
    refresh();
    refreshDiag();
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

for (const r of document.querySelectorAll('input[name=src]')) {
  r.addEventListener('change', () => {
    const dir = $('srcDir').checked;
    $('udir').classList.toggle('hidden', !dir);
    $('uzip').classList.toggle('hidden', dir);
  });
}

// 选了文件夹：ID 没填就用文件夹名猜一个
$('udir').addEventListener('change', () => {
  const f = $('udir').files[0];
  if (f && !$('uid').value) {
    $('uid').value = f.webkitRelativePath.split('/')[0].toLowerCase().replace(/[^a-z0-9._-]+/g, '-').replace(/^-+/, '');
  }
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
    const meta = await api('PUT', `/api/admin/apps/${encodeURIComponent($('uid').value.trim())}?${q}`, blob,
      { 'Content-Type': 'application/zip' });
    msg.className = 'msg ok';
    msg.textContent = `已上架「${meta.name}」v${meta.version}（${meta.files} 个文件，${meta.pages} 页）`;
    refresh();
  } catch (err) {
    msg.className = 'msg err';
    msg.textContent = err.message;
    if (err.status === 401) show('login');
  } finally {
    $('ubtn').disabled = false;
  }
});

async function refresh() {
  const { apps } = await api('GET', '/api/apps');
  $('count').textContent = `${apps.length} 个`;
  const tbody = $('list');
  tbody.replaceChildren();
  apps.forEach((a, i) => {
    const tr = document.createElement('tr');
    const icon = document.createElement('span');
    icon.className = 'icon';
    icon.style.background = COLORS[hash(a.id) % COLORS.length];
    icon.textContent = [...a.name][0] || '?';
    const name = document.createElement('div');
    name.innerHTML = '<div class="name"></div><div class="id"></div><div class="hint"></div>';
    name.children[0].textContent = a.name;
    name.children[1].textContent = a.id;
    name.children[2].textContent = a.description || '';
    const ops = document.createElement('td');
    ops.className = 'ops';
    ops.append(
      btn('改名', () => edit(a)),
      btn('下载', () => { location.href = a.packageUrl; }),
      btn('删除', () => remove(a), 'danger'),
    );
    tr.append(td(icon), td(name), td(`v${a.version}`), td(`${(a.size / 1048576).toFixed(2)} MB`),
      td(new Date(a.updatedAt).toLocaleString()), ops);
    tbody.append(tr);
  });
}

async function refreshDiag() {
  const { items } = await api('GET', '/api/admin/diag');
  $('diagCount').textContent = `${items.length} 份`;
  const tbody = $('diagList');
  tbody.replaceChildren();
  for (const d of items) {
    const tr = document.createElement('tr');
    const app = d.app || {};
    const ops = document.createElement('td');
    ops.className = 'ops';
    ops.append(
      btn('下载', async () => {
        const full = await api('GET', `/api/admin/diag/${d.id}`);
        const a = document.createElement('a');
        a.href = URL.createObjectURL(new Blob([JSON.stringify(full, null, 1)], { type: 'application/json' }));
        a.download = `${d.id}.json`;
        a.click();
      }),
      btn('删除', async () => {
        if (!confirm(`删除诊断 ${d.id}？`)) return;
        await api('DELETE', `/api/admin/diag/${d.id}`);
        refreshDiag();
      }, 'danger'),
    );
    tr.append(td(`${d.id}\n${new Date(d.receivedAt).toLocaleString()}`), td(`${d.host || ''} · ${d.platform || ''}\n${d.device || ''}`),
      td(app.id || app.appId || app.name || '-'), td(`${d.logLines} 行 / ${(d.size / 1024).toFixed(0)} KB`), td(d.note || ''), ops);
    tr.querySelectorAll('td').forEach((c) => { c.style.whiteSpace = 'pre-line'; });
    tbody.append(tr);
  }
}

async function edit(a) {
  const name = prompt('名称', a.name);
  if (name === null) return;
  const description = prompt('描述', a.description || '');
  if (description === null) return;
  await api('PATCH', `/api/admin/apps/${a.id}`, JSON.stringify({ name, description }), { 'Content-Type': 'application/json' });
  refresh();
}

async function remove(a) {
  if (!confirm(`删除「${a.name}」（${a.id}）？客户端已下载的不受影响，列表里不再出现。`)) return;
  await api('DELETE', `/api/admin/apps/${a.id}`);
  refresh();
}

function td(content) {
  const el = document.createElement('td');
  if (typeof content === 'string') el.textContent = content; else el.append(content);
  return el;
}

function btn(text, onClick, cls = '') {
  const b = document.createElement('button');
  b.type = 'button';
  b.textContent = text;
  b.className = cls;
  b.addEventListener('click', () => Promise.resolve().then(onClick).catch((err) => alert(err.message)));
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
