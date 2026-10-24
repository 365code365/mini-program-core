# fontdue 0.9.3（mini-render 改过的一份）

上游：https://github.com/mooman219/fontdue ，版本 0.9.3，许可证见同目录 `LICENSE-*`（MIT / Apache-2.0 / Zlib，原样保留）。

## 为什么要改

上游 `Font::from_bytes` 会把 cmap 映射到的每个字形的几何（线段表、包围盒）一次性展开。一个 CJK 字面 3 万个字形，
展开后要 300~400MB，而一个页面通常只用到几百个字。这一项占了引擎内存的 98%（`doc/引擎测试说明.md` 第五之二节）。

## 改了什么（只动了 `src/font.rs`，外加 `lib.rs` 里一条 lint 放行）

- 字形改成**第一次用到时**才展开（`Font::glyph`）。展开的代码、参数、顺序和上游 `generate_glyph` 逐行一致，
  所以同一个字形的度量和光栅化结果逐位相同（gallery 65 张、全量回归逐字节对照）。
- 上游在加载时就不展开的字形（cmap / gsub 都到不了的那些），这里照样给空字形（`loadable` 位图）。
- 字体留着文件字节（`FontBytes = Arc<dyn AsRef<[u8]>>`）和解析好的 `Face`。
  `from_shared` 直接接收 mmap 进来的文件或内嵌字节，同一个 TTC 的几个字面可以共用一份。
  `from_bytes` 的签名保持不变（内部拷一份字节）。
- 整个文件的哈希（`file_hash`）改成用到时才算。现算的话要把文件每一页都读一遍，mmap 进来的字体就整个进了物理内存。
- 字符 → 字形表不在加载时建。上游那张表三万多项，每个字面约 1MB。现在用到哪个字查哪个
  （`lookup_glyph_index` → `cmap_lookup`：从最后一个子表往前找第一个非 0 映射，等价于上游「按顺序插入、后插覆盖、0 不插」的建表规则），
  BMP 内的结果记在一张 `AtomicU16` 表里，热路径上只是一次原子读。`chars()` 用到时才建上游那张整表。
- 字形格子里只留度量（前进宽、包围盒）。光栅化要的轮廓线段放进一个按字节封顶的缓存（`GEOMETRY_BUDGET` = 1MB / 字面），
  超了就先进先出丢掉，再用到时重新展开，结果与第一次逐位相同。
- 对照测试：`tests/font_lazy_e2e.rs` 把这一份和 crates.io 上的上游 0.9.3（dev-dependency `fontdue_upstream`）放在一起比：
  - 每个 Unicode 码位的字形编号都比；
  - Arial / Monaco 的全部字形在 4 个字号下比度量和位图，SourceHanSans 比常用汉字；
  - 光栅化都走两遍，第二遍时轮廓线段已被丢掉、要重新展开。
- 去掉了 `Clone`，`parallel` 特性不再起作用（展开只在用到时、一次一个）。

升级上游时，把上面几处重新套上去。
