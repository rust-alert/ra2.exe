//! 主菜单选项页：`RT_DIALOG` `0xD5` → `LayoutSnapshot`。
//!
//! 几何权威为 [`dialog_template_0xd5`]；本模块只导出内容控件 id 与运行时入口。

use ra_types::dialog_template_0xd5;

use crate::snapshot::LayoutSnapshot;

use super::from_template::solve_options_dialog;

/// 选项页左栏表单控件 id（与 `0xD5` / `paint_options_dialog_controls` 对齐）。
///
/// 不含右栏 `keyboard` / `network` / `main_menu`，也不含壳层 `title` / `status_help` / `rail_badge`。
pub const OPTIONS_CONTENT_IDS: &[&str] = &[
    "sec_display",
    "caption_detail",
    "value_detail",
    "track_detail",
    "caption_resolution",
    "resolution",
    "sec_game",
    "caption_difficulty",
    "value_difficulty",
    "track_difficulty",
    "sec_ui",
    "check_tooltips",
    "check_scanlines",
    "check_damage",
    "caption_scroll",
    "value_scroll",
    "track_scroll",
    "sec_audio",
    "caption_music",
    "track_music",
    "caption_sound",
    "track_sound",
    "caption_voice",
    "track_voice",
];

/// 用壳层默认 chrome 求解选项整页（`0xD5` 模板 + 左栏表单居中）。
pub fn solve_options_page() -> LayoutSnapshot {
    debug_assert_eq!(dialog_template_0xd5().dialog_id, 0xD5);
    solve_options_dialog()
}
