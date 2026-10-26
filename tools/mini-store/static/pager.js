// 管理页的分页条：「第 x–y 条 / 共 N 条」+ 每页条数 + 上一页 / 页码 / 下一页 + 跳转。
// 单独一个文件：admin.js 已经四百多行，分页与业务无关，别再往里堆。
// 用法：renderPager(容器, { page, pageSize, total, sizes, onChange(page, pageSize) })
'use strict';
(function () {
  function mk(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  // 页码列表：总是有首页、末页、当前页左右各 2 页，中间省略号（页数多了也只占一行）
  function pageItems(cur, pages) {
    const out = [];
    const keep = new Set([1, pages]);
    for (let p = cur - 2; p <= cur + 2; p++) if (p >= 1 && p <= pages) keep.add(p);
    let last = 0;
    for (const p of [...keep].sort((a, b) => a - b)) {
      if (p - last > 1) out.push('…');
      out.push(p);
      last = p;
    }
    return out;
  }

  window.renderPager = function (box, o) {
    const pages = Math.max(1, Math.ceil(o.total / o.pageSize));
    const cur = Math.min(Math.max(1, o.page), pages);
    const go = (p, size = o.pageSize) => o.onChange(Math.min(Math.max(1, p), size === o.pageSize ? pages : Math.max(1, Math.ceil(o.total / size))), size);
    const from = o.total ? (cur - 1) * o.pageSize + 1 : 0;
    const to = Math.min(o.total, cur * o.pageSize);

    const info = mk('span', 'pg-info', `第 ${from}–${to} 条 / 共 ${o.total} 条`);
    const size = mk('select', 'pg-size');
    size.setAttribute('aria-label', '每页条数');
    for (const n of o.sizes || [10, 20, 50]) size.append(new Option(`${n} 条/页`, String(n), false, n === o.pageSize));
    // 换每页条数：保持当前页第一条仍在视野里（跳到它所在的新页），而不是回第 1 页
    size.addEventListener('change', () => { const n = Number(size.value); go(Math.floor((from - 1) / n) + 1, n); });

    const nav = mk('div', 'pg-nav');
    const prev = mk('button', 'pg-btn', '‹');
    prev.type = 'button';
    prev.title = '上一页';
    prev.disabled = cur <= 1;
    prev.addEventListener('click', () => go(cur - 1));
    nav.append(prev);
    for (const it of pageItems(cur, pages)) {
      if (it === '…') { nav.append(mk('span', 'pg-dots', '…')); continue; }
      const b = mk('button', it === cur ? 'pg-btn on' : 'pg-btn', String(it));
      b.type = 'button';
      if (it === cur) b.setAttribute('aria-current', 'page');
      b.addEventListener('click', () => go(it));
      nav.append(b);
    }
    const next = mk('button', 'pg-btn', '›');
    next.type = 'button';
    next.title = '下一页';
    next.disabled = cur >= pages;
    next.addEventListener('click', () => go(cur + 1));
    nav.append(next);

    const jump = mk('label', 'pg-jump', '跳至');
    const input = mk('input');
    input.type = 'text';
    input.inputMode = 'numeric';
    input.setAttribute('aria-label', '跳转到页码');
    input.addEventListener('keydown', (e) => {
      if (e.key !== 'Enter') return;
      const n = parseInt(input.value, 10);
      if (Number.isFinite(n)) go(n);
    });
    jump.append(input, document.createTextNode('页'));

    box.replaceChildren(info, mk('span', 'sp'), size, nav, jump);
    box.classList.toggle('hidden', o.total === 0);
  };
})();
