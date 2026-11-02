// 规格面板的互斥计算：某个选项能不能点，看「把它换进当前选择后，是否还存在有库存的组合」。
// 没选的维度当通配。全部选齐才得到一个 sku 键（各维下标用 | 连起来）。

function keys(g) { return Object.keys(g.sku.map); }

function match(key, sel) {
  const parts = key.split('|');
  return sel.every((v, i) => v < 0 || +parts[i] === v);
}

function available(g, sel, dim, opt) {
  const s = sel.slice();
  s[dim] = opt;
  return keys(g).some((k) => match(k, s) && g.sku.map[k].stock > 0);
}

// 面板要画的：每个选项的选中 / 置灰状态
function view(g, sel) {
  return g.sku.dims.map((d, di) => ({
    name: d.name,
    opts: d.opts.map((label, oi) => ({ label, on: sel[di] === oi, off: !available(g, sel, di, oi) }))
  }));
}

function firstInStock(g) {
  const k = keys(g).find((x) => g.sku.map[x].stock > 0);
  return k ? k.split('|').map(Number) : g.sku.dims.map(() => -1);
}

// 点一个选项：已选再点 = 取消；置灰的不响应
function toggle(g, sel, dim, opt) {
  if (sel[dim] === opt) { const s = sel.slice(); s[dim] = -1; return s; }
  if (!available(g, sel, dim, opt)) return sel;
  const s = sel.slice();
  s[dim] = opt;
  return s;
}

function keyOf(sel) { return sel.every((v) => v >= 0) ? sel.join('|') : ''; }

// 没选齐时显示价格区间
function priceRange(g, sel, field) {
  const ps = keys(g).filter((k) => match(k, sel) && g.sku.map[k].stock > 0).map((k) => g.sku.map[k][field]);
  if (!ps.length) return '';
  const lo = Math.min.apply(null, ps), hi = Math.max.apply(null, ps);
  return lo === hi ? String(lo) : `${lo}~${hi}`;
}

module.exports = { view, firstInStock, toggle, keyOf, priceRange };
