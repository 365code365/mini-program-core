            // ==================== 组件实例树：每处使用一个实例 ====================
            // 宿主每次取渲染数据时，把这一帧里所有组件的出现位置（实例键 + 带类型的属性 +
            // 宿主上的事件绑定）交过来（Rust 侧 `component_mount::render_data`），这里对账：
            // 没见过的键建实例，见过的比属性、变了才写进去并触发 observer，这一帧不在了的
            // 跑 detached 并回收。实例键的规则见 Rust 侧 `parser::template::instance_key`。
            //
            // 从前是「一个标签一个实例」，属性取第一处使用、只在挂载时求一次值：同一个组件用两次
            // 显示两份一样的内容，列表里的组件全拿第一项的数据，页面 setData 后属性也不会变。

            // 一页一份：实例键 -> 实例、文档顺序、标签 -> 组件路径。挂在页面对象上（不可枚举，
            // 不会出现在框架对页面实例的遍历里），页面出栈时跟着走；没有页面时（单测直接调）用孤儿表
            var __orphanCompState = null;
            function __newCompState(page) { return { map: {}, order: [], paths: {}, page: page || null }; }
            function __compStateOf(page) {
                if (!page) {
                    if (!__orphanCompState) { __orphanCompState = __newCompState(null); }
                    return __orphanCompState;
                }
                if (!page.__compState) {
                    var st = __newCompState(page);
                    try {
                        Object.defineProperty(page, '__compState', { value: st, enumerable: false, writable: true, configurable: true });
                    } catch (e) { page.__compState = st; }
                }
                return page.__compState;
            }
            function __compState() { return __compStateOf(__currentPage); }

            /// 登记本页用到的组件：标签 -> 组件路径（`Component()` 的注册键）。
            /// 换页时不清：SDK 返回上一页不会重新挂载，靠这张表在对账时重建实例
            function __registerPageComponent(tag, path) { __compState().paths[tag] = path; }

            var __compWarned = {};

            /// 对账入口。`input.list` 按文档顺序（外层组件在前）；返回 1 表示有实例被建出、
            /// 属性变了或被回收 —— 宿主据此重取渲染数据再对一轮（新实例的子组件要等它有了 data 才看得见）。
            function __reconcileComponents(input) {
                var list = (input && input.list) || [];
                var st = __compState();
                var seen = {};
                var order = [];
                var changed = false;
                for (var i = 0; i < list.length; i++) {
                    var o = list[i];
                    if (!o || !o.key || seen[o.key]) { continue; }
                    seen[o.key] = true;
                    var inst = st.map[o.key];
                    if (inst && inst.__tag !== o.tag) { __detachComponentInstance(inst.id); inst = null; changed = true; }
                    if (inst) {
                        __compApplyMeta(inst, o);
                        if (__compUpdateProps(inst, o.props || {})) { changed = true; }
                    } else {
                        var path = st.paths[o.tag];
                        if (!path || !__componentDefs[path]) {
                            if (!__compWarned[o.tag]) {
                                __compWarned[o.tag] = 1;
                                __native_print('⚠️ 组件 <' + o.tag + '> 没有注册定义（Component() 未执行或路径不对）');
                            }
                            continue;
                        }
                        var meta = {
                            key: o.key, tag: o.tag, owner: o.owner || '', tree: st,
                            id: o.id, 'class': o['class'], dataset: o.dataset, events: o.events
                        };
                        inst = __createComponentInstance(path, o.props || {}, meta);
                        if (!inst) { continue; }
                        changed = true;
                    }
                    order.push(o.key);
                }
                // 这一帧不在了的（wx:if 变假、列表变短、外层组件没了）：detached 并回收。
                // 按上一帧顺序倒着走，内层组件先于外层
                var gone = [];
                for (var j = st.order.length - 1; j >= 0; j--) {
                    var ok = st.order[j];
                    if (!seen[ok] && st.map[ok]) { gone.push(ok); }
                }
                for (var mk in st.map) {
                    if (st.map.hasOwnProperty(mk) && !seen[mk] && gone.indexOf(mk) < 0) { gone.push(mk); }
                }
                for (var g = 0; g < gone.length; g++) {
                    var dead = st.map[gone[g]];
                    if (dead) { __detachComponentInstance(dead.id); changed = true; }
                }
                st.order = order;
                __syncPageComponents(st);
                return changed ? 1 : 0;
            }

            /// 老接口 `__pageComponents`（标签 -> 第一个实例 id）跟着实例树同步，`__hasHandler` 还在用
            function __syncPageComponents(st) {
                __pageComponents = {};
                for (var i = 0; i < st.order.length; i++) {
                    var inst = st.map[st.order[i]];
                    if (inst && !__pageComponents.hasOwnProperty(inst.__tag)) { __pageComponents[inst.__tag] = inst.id; }
                }
                for (var k in st.map) {
                    var x = st.map[k];
                    if (st.map.hasOwnProperty(k) && x && !__pageComponents.hasOwnProperty(x.__tag)) { __pageComponents[x.__tag] = x.id; }
                }
            }

            /// 这处使用的身份：实例键、归属（使用方实例键，页面为空串）、宿主 id/class/dataset、事件绑定
            function __compApplyMeta(inst, m) {
                if (m.key !== undefined) { inst.__key = m.key; }
                if (m.tag !== undefined) { inst.__tag = m.tag; }
                if (m.owner !== undefined) { inst.__ownerKey = m.owner || ''; }
                if (m.tree) { inst.__tree = m.tree; }
                inst.__hostId = m.id || '';
                inst.__hostClass = m['class'] || '';
                inst.dataset = m.dataset || {};
                inst.__events = m.events || {};
                // triggerEvent 冒泡路径：宿主自己 + 使用方模板里写了绑定的祖先（由内向外，Rust 侧 ChainFrame）
                inst.__chain = m.chain || null;
            }

            /// `wx://form-field` 组件的当前值（form 提交时宿主按实例键来取）
            function __compFieldValue(key) {
                var inst = __compState().map[key];
                var v = inst && inst.data ? inst.data.value : null;
                return JSON.stringify(v === undefined ? null : v);
            }

            /// 使用方（页面或外层组件实例）：triggerEvent 的派发对象、selectOwnerComponent 的结果
            function __ownerOf(inst) {
                var st = inst.__tree || __compState();
                if (inst.__ownerKey) { return st.map[inst.__ownerKey] || null; }
                return st.page || __currentPage;
            }

            // ---------- 属性：类型转换（照 glass-easel data_proxy.ts 的 convertValueToType） ----------
            function __propType(t) {
                if (t === String) { return 'S'; }
                if (t === Number) { return 'N'; }
                if (t === Boolean) { return 'B'; }
                if (t === Object) { return 'O'; }
                if (t === Array) { return 'A'; }
                if (t === Function) { return 'F'; }
                return '*'; // null / 未写 type：不限类型
            }
            function __propFallback(t) {
                if (t === 'S') { return ''; }
                if (t === 'N') { return 0; }
                if (t === 'B') { return false; }
                if (t === 'A') { return []; }
                if (t === 'F') { return function() {}; }
                return null;
            }
            /// properties 里一项的规格：类型、optionalTypes、默认值工厂（每次给新拷贝）
            function __propSpec(def) {
                if (def === null || def === undefined || typeof def === 'function') {
                    var st = __propType(def);
                    return { type: st, optional: null, dflt: function() { return __propFallback(st); } };
                }
                if (typeof def !== 'object') { return { type: '*', optional: null, dflt: function() { return null; } }; }
                var type = __propType(def.type);
                var optional = null;
                if (Array.isArray(def.optionalTypes) && def.optionalTypes.length) {
                    optional = [];
                    for (var i = 0; i < def.optionalTypes.length; i++) { optional.push(__propType(def.optionalTypes[i])); }
                    if (type === '*') { type = optional[0]; }
                }
                var dflt;
                if (typeof def['default'] === 'function') {
                    dflt = function() {
                        var v;
                        try { v = def['default'](); } catch (e) { __native_print('[property.default] ' + e.message); }
                        return v !== undefined ? v : __propFallback(type);
                    };
                } else if (def.value !== undefined) {
                    dflt = function() { return __compDeepCopy(def.value); };
                } else {
                    dflt = function() { return __propFallback(type); };
                }
                return { type: type, optional: optional, dflt: dflt };
            }
            function __propMatch(t, v) {
                if (t === 'S') { return typeof v === 'string'; }
                if (t === 'N') { return typeof v === 'number' && isFinite(v); }
                if (t === 'B') { return typeof v === 'boolean'; }
                if (t === 'O') { return typeof v === 'object' && !Array.isArray(v); }
                if (t === 'A') { return Array.isArray(v); }
                if (t === 'F') { return typeof v === 'function'; }
                return v !== undefined;
            }
            /// 使用方传来的值按声明转换：先看 optionalTypes 能否原样接受，再按主类型转 ——
            /// String 用 String(v)（null 给默认值），Number 不是有限数给默认值，Boolean 取真值，
            /// Array/Object 类型不对给默认值
            function __coerceProp(def, value) {
                var s = __propSpec(def);
                if (s.optional) {
                    for (var i = 0; i < s.optional.length; i++) {
                        if (__propMatch(s.optional[i], value)) { return value; }
                    }
                }
                switch (s.type) {
                    case 'S': return (value === null || value === undefined) ? s.dflt() : String(value);
                    case 'N': return isFinite(value) ? Number(value) : s.dflt();
                    // 只写属性名不写值（`<my-comp disabled>`）在微信里是 true；解析器把它记成空串，分不出
                    // `disabled=""` —— 后者几乎没人写来表示 false，所以空串按 true（原生组件那边也是这么认的）
                    case 'B': return value === '' ? true : !!value;
                    case 'A': return Array.isArray(value) ? value : s.dflt();
                    case 'O': return typeof value === 'object' ? value : s.dflt();
                    case 'F': return typeof value === 'function' ? value : s.dflt();
                    default: return value === undefined ? s.dflt() : value;
                }
            }
            function __compDeepCopy(v, depth) {
                depth = depth || 0;
                if (v === null || typeof v !== 'object' || depth > 32) { return v; }
                var out, i;
                if (Array.isArray(v)) {
                    out = [];
                    for (i = 0; i < v.length; i++) { out.push(__compDeepCopy(v[i], depth + 1)); }
                    return out;
                }
                out = {};
                for (i in v) { if (v.hasOwnProperty(i)) { out[i] = __compDeepCopy(v[i], depth + 1); } }
                return out;
            }

            /// 宿主给的属性与上一帧比（按 JSON），只把真变了的写进去 —— 页面改了无关字段时不该触发 observer
            function __compUpdateProps(inst, props) {
                var raw = inst.__rawProps || (inst.__rawProps = {});
                var changes = null;
                for (var k in props) {
                    if (!props.hasOwnProperty(k)) { continue; }
                    if (raw[k] !== JSON.stringify(props[k])) { (changes || (changes = {}))[k] = props[k]; }
                }
                if (!changes) { return false; }
                __compApplyProps(inst, changes);
                return true;
            }

            /// 写入使用方传来的属性：转换类型 → 写 data → `observers` → 属性自己的 `observer(新值, 旧值)`
            /// （glass-easel 的顺序：数据监听器随写入触发，属性 observer 在之后）。
            /// 没在 properties 里声明的不是属性，微信不会把它写进组件 data。
            function __compApplyProps(inst, props) {
                var entry = __componentDefs[inst.is];
                var defs = (entry && entry.merged.properties) || {};
                var raw = inst.__rawProps || (inst.__rawProps = {});
                var keys = [];
                var fired = [];
                for (var k in props) {
                    if (!props.hasOwnProperty(k)) { continue; }
                    raw[k] = JSON.stringify(props[k]);
                    if (!defs.hasOwnProperty(k)) { continue; }
                    var nv = __coerceProp(defs[k], props[k]);
                    var ov = inst.data[k];
                    inst.data[k] = nv;
                    keys.push(k);
                    if (ov !== nv) { fired.push([k, nv, ov]); }
                }
                if (!keys.length) { return; }
                __runObservers(inst, keys);
                __runPropObservers(inst, defs, fired);
                if (typeof __native_page_update === 'function') { __native_page_update(); }
            }

            /// 组件自己的 setData：属性字段同样做类型转换、值变了触发属性 observer；
            /// `observers` 按「设置了哪些路径」触发（值没变也触发 —— 官方 observer.html 的语义）
            function __compSetData(inst, newData, callback) {
                var entry = __componentDefs[inst.is];
                var defs = (entry && entry.merged.properties) || {};
                var keys = [];
                var olds = {};
                var apply = {};
                if (newData) {
                    for (var k in newData) {
                        if (!newData.hasOwnProperty(k)) { continue; }
                        keys.push(k);
                        if (defs.hasOwnProperty(k)) {
                            olds[k] = inst.data[k];
                            apply[k] = __coerceProp(defs[k], newData[k]);
                        } else {
                            apply[k] = newData[k];
                        }
                    }
                }
                __applySetData(inst, apply, null);
                var fired = [];
                for (var p in olds) {
                    if (olds.hasOwnProperty(p) && inst.data[p] !== olds[p]) { fired.push([p, inst.data[p], olds[p]]); }
                }
                __runObservers(inst, keys);
                __runPropObservers(inst, defs, fired);
                // 与页面的 setData 一样：画出来之后（下一轮事件循环）再回调
                if (typeof callback === 'function') {
                    setTimeout(function() {
                        try { callback.call(inst); } catch (e) { __native_print('[setData.cb] ' + e.message); }
                    }, 0);
                }
            }

            function __runPropObservers(inst, defs, fired) {
                for (var i = 0; i < fired.length; i++) {
                    var d = defs[fired[i][0]];
                    var ob = (d && typeof d === 'object') ? d.observer : null;
                    if (typeof ob === 'string') { ob = inst[ob]; } // `observer: '方法名'`
                    if (typeof ob !== 'function') { continue; }
                    try { ob.call(inst, fired[i][1], fired[i][2]); }
                    catch (e) { __native_print('[property.observer] ' + fired[i][0] + ': ' + e.message); }
                }
            }

            // ---------- 数据监听器 observers ----------
            // 键可以是 'a, b'（多个字段，处理函数按序收到各自的值）、'a.b'、'arr[1]'、
            // 'a.b.**'（本身或任何子字段）、'**'（任何字段）。按 glass-easel 的观察树语义：
            // 设置被监听路径本身或它的上层会触发；设置它的**下层**只有带 `.**` 的才触发。
            function __pathSegs(p) {
                var out = [];
                var re = /([^.\[\]\s]+)|\[(\d+)\]/g;
                var m;
                while ((m = re.exec(String(p))) !== null) { out.push(m[1] !== undefined ? m[1] : m[2]); }
                return out;
            }
            function __obsList(inst) {
                var entry = __componentDefs[inst.is];
                if (!entry) { return []; }
                if (entry.__obsList) { return entry.__obsList; }
                var list = [];
                var obs = entry.merged.observers || {};
                for (var key in obs) {
                    if (!obs.hasOwnProperty(key) || typeof obs[key] !== 'function') { continue; }
                    var parts = key.split(',');
                    var paths = [];
                    for (var i = 0; i < parts.length; i++) {
                        var p = parts[i].trim();
                        if (!p) { continue; }
                        if (p === '**') { paths.push({ segs: [], wild: true }); continue; }
                        var wild = /\.\*\*$/.test(p);
                        if (wild) { p = p.slice(0, -3); }
                        paths.push({ segs: __pathSegs(p), wild: wild });
                    }
                    if (paths.length) { list.push({ fn: obs[key], paths: paths, name: key }); }
                }
                entry.__obsList = list;
                return list;
            }
            function __segPrefix(a, b) {
                if (a.length > b.length) { return false; }
                for (var i = 0; i < a.length; i++) { if (a[i] !== b[i]) { return false; } }
                return true;
            }
            function __getBySegs(d, segs) {
                var cur = d;
                for (var i = 0; i < segs.length; i++) {
                    if (cur === null || cur === undefined) { return undefined; }
                    cur = cur[segs[i]];
                }
                return cur;
            }
            /// `changed`：这次设置的路径数组（setData 的键 / 属性名）。每个监听器最多触发一次
            function __runObservers(inst, changed) {
                var list = __obsList(inst);
                if (!list.length || !changed || !changed.length) { return; }
                var cs = [];
                for (var c = 0; c < changed.length; c++) { cs.push(__pathSegs(changed[c])); }
                for (var i = 0; i < list.length; i++) {
                    var ob = list[i];
                    var hit = false;
                    for (var w = 0; w < ob.paths.length && !hit; w++) {
                        var wp = ob.paths[w];
                        for (var x = 0; x < cs.length; x++) {
                            if (__segPrefix(cs[x], wp.segs) || (wp.wild && __segPrefix(wp.segs, cs[x]))) { hit = true; break; }
                        }
                    }
                    if (!hit) { continue; }
                    var args = [];
                    for (var a = 0; a < ob.paths.length; a++) { args.push(__getBySegs(inst.data, ob.paths[a].segs)); }
                    try { ob.fn.apply(inst, args); }
                    catch (e) { __native_print('[observers] ' + ob.name + ': ' + e.message); }
                }
            }

            // ---------- selectComponent / selectAllComponents ----------
            // 只在调用方**自己的模板**里找（页面找页面模板里的、组件找组件模板里的，
            // slot 内容算使用方的）；`>>>` 跨组件往下找。选择器只认最后一段复合选择器
            // （`tag#id.cls1.cls2`），后代/子代关系不校验。组件定义了 `export()` 时返回它的结果。
            function __parseCompound(s) {
                var m = /^([a-zA-Z][\w-]*)?((?:[#.][\w-]+)*)$/.exec(s);
                if (!m || (!m[1] && !m[2])) { return null; }
                var t = { tag: m[1] || '', id: '', cls: [] };
                var re = /([#.])([\w-]+)/g;
                var p;
                while ((p = re.exec(m[2] || '')) !== null) {
                    if (p[1] === '#') { t.id = p[2]; } else { t.cls.push(p[2]); }
                }
                return t;
            }
            function __matchCompound(t, inst) {
                if (t.tag && t.tag !== inst.__tag) { return false; }
                if (t.id && t.id !== inst.__hostId) { return false; }
                if (t.cls.length) {
                    var have = ' ' + String(inst.__hostClass || '').split(/\s+/).join(' ') + ' ';
                    for (var i = 0; i < t.cls.length; i++) { if (have.indexOf(' ' + t.cls[i] + ' ') < 0) { return false; } }
                }
                return true;
            }
            function __exportOf(inst) {
                var e = __componentDefs[inst.is];
                if (e && e.def && typeof e.def['export'] === 'function') {
                    try { return e.def['export'].call(inst); } catch (er) { __native_print('[export] ' + er.message); }
                }
                return inst;
            }
            function __selectComponentsFrom(st, ownerKey, sel, all) {
                st = st || __compState();
                ownerKey = ownerKey || '';
                var tests = [];
                var groups = String(sel || '').split(',');
                for (var g = 0; g < groups.length; g++) {
                    var s = groups[g].trim();
                    if (!s) { continue; }
                    var parts = s.split(/>>>|>|\s+/).filter(function(x) { return !!x; });
                    var t = parts.length ? __parseCompound(parts[parts.length - 1]) : null;
                    if (t) { tests.push({ t: t, deep: s.indexOf('>>>') >= 0 }); }
                }
                var out = [];
                for (var i = 0; i < st.order.length && (all || !out.length); i++) {
                    var inst = st.map[st.order[i]];
                    if (!inst) { continue; }
                    for (var j = 0; j < tests.length; j++) {
                        var ok = tests[j].deep
                            ? (!ownerKey || String(inst.__key).indexOf(ownerKey + '/') === 0)
                            : (inst.__ownerKey || '') === ownerKey;
                        if (ok && __matchCompound(tests[j].t, inst)) { out.push(__exportOf(inst)); break; }
                    }
                }
                return all ? out : (out.length ? out[0] : null);
            }
