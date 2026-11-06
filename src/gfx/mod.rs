//! 基础图形类型：颜色、几何（点 / 矩形 / 整数像素框）、画笔、路径。画布本身在 `canvas/`。
//! 这几个类型在 crate 根有再导出（`mini_render::Color` / `Rect` / `Point` / `Path` …），外部照旧从根上用。
pub(crate) mod color;
pub(crate) mod geometry;
pub(crate) mod paint;
pub(crate) mod path;
