//! 面板弹着时跟上页面数据：multiSelector 的多列联动。
//!
//! 微信的写法是在 `bindcolumnchange` 里按第 n 列的新值 setData 改 `range`（后面几列的选项），
//! 面板**立刻**换上新选项。面板打开时只拷了一份选项，所以宿主每帧拿渲染器里同一个 picker
//! 的最新登记（id 相同）对一下：列内容变了就换上，选中下标夹回新范围。
//! 选中下标以面板为准（用户正在选），不以属性 `value` 为准 —— 那要等「确定」才写回。

use super::*;

/// 返回是否换了列（要重画面板）
pub fn sync_columns(sheet: &mut Option<PickerSheetState>, renderer: Option<&crate::renderer::WxmlRenderer>) -> bool {
    let Some(s) = sheet.as_mut().filter(|s| s.visible && !s.closing && s.mode == SheetMode::MultiSelector) else {
        return false;
    };
    let Some(b) = renderer.and_then(|r| r.picker_regions().iter().find(|p| p.id == s.id)) else {
        return false;
    };
    if b.multi_range.is_empty() || b.multi_range == s.columns {
        return false;
    }
    s.columns = b.multi_range.clone();
    s.selected.resize(s.columns.len(), 0);
    for (i, col) in s.columns.iter().enumerate() {
        s.selected[i] = s.selected[i].min(col.len().saturating_sub(1));
    }
    true
}
