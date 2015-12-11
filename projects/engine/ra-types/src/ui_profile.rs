//! Adaptor 产出的运行时 UI 配置（数据契约骨架）。
//!
//! 页面代码只消费稳定的控件 / 资源 / 文案键；版本差异在解析阶段折叠进本结构。
//! 几何求解仍由 `ra-layout` 完成，本模块不持有已解析像素矩形。

/// 稳定控件标识（配置字符串，非显示文案）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControlId(pub String);

/// 文案键（如 CSF 标签）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextKey(pub String);

/// 逻辑资源角色（如右栏顶盖、地图预览槽）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetRole(pub String);

/// 控件相对壳层 chrome 的放置策略（求解器解释，页面不写像素公式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlPlacement {
    /// DLU 直接换算为设计像素。
    #[default]
    PreserveDlu,
    /// 右栏按钮列按 tile 格吸附。
    TileSnap,
    /// 相对右栏宽度水平居中并锚到右缘。
    RightPanelAnchor,
    /// 贴底盖上沿的一行按钮格（忽略模板 y）。
    BottomCoverButton,
    /// 地图名底板（贴右缘，底边落在第一根 tile 下沿）。
    MapNamePlate,
    /// DLU 换算后强制下拉面高度（壳层 combo face）。
    ComboFace,
    /// 底栏悬停提示：贴壳层 `lower_strip` 内 tooltip 带（忽略模板 y）。
    ShellTooltip,
}

/// 对话框模板中的单个控件描述（DLU，尚未求解）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogControlDesc {
    /// 控件 id。
    pub id: ControlId,
    /// DLU 左。
    pub dlu_x: i32,
    /// DLU 上。
    pub dlu_y: i32,
    /// DLU 宽。
    pub dlu_w: i32,
    /// DLU 高。
    pub dlu_h: i32,
    /// 放置策略。
    pub placement: ControlPlacement,
}

impl DialogControlDesc {
    /// 构造保留 DLU 的控件。
    pub fn preserve(id: impl Into<String>, x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            id: ControlId(id.into()),
            dlu_x: x,
            dlu_y: y,
            dlu_w: w,
            dlu_h: h,
            placement: ControlPlacement::PreserveDlu,
        }
    }

    /// 构造带放置策略的控件。
    pub fn with_placement(
        id: impl Into<String>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        placement: ControlPlacement,
    ) -> Self {
        Self {
            id: ControlId(id.into()),
            dlu_x: x,
            dlu_y: y,
            dlu_w: w,
            dlu_h: h,
            placement,
        }
    }
}

/// 一份 `RT_DIALOG` 模板摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogTemplate {
    /// 资源对话框 id（如 `0x6B`、`0x102`）。
    pub dialog_id: u16,
    /// 控件表。
    pub controls: Vec<DialogControlDesc>,
}

/// 运行时 UI 能力开关（缺资源时的降级由 adaptor 填入）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiCapabilities {
    /// 是否允许随机地图创建入口。
    pub create_random_map: bool,
    /// 是否具备遭遇战大厅完整控件。
    pub skirmish_lobby: bool,
}

/// Adaptor 解析后的不可变 UI 配置。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeUiProfile {
    /// 资源检索链（逻辑名，非绝对路径）。
    pub resource_chain: Vec<String>,
    /// 对话框模板。
    pub dialog_templates: Vec<DialogTemplate>,
    /// 资源角色别名（角色 → 逻辑资源名）。
    pub asset_aliases: Vec<(AssetRole, String)>,
    /// 调色板绑定（角色 → 逻辑调色板名）。
    pub palette_bindings: Vec<(AssetRole, String)>,
    /// 字体绑定（角色 → 逻辑字体名）。
    pub font_bindings: Vec<(AssetRole, String)>,
    /// 文案目录来源标签。
    pub text_catalog_sources: Vec<String>,
    /// 能力。
    pub ui_capabilities: UiCapabilities,
}

impl RuntimeUiProfile {
    /// 按对话框 id 查找模板。
    pub fn dialog(&self, dialog_id: u16) -> Option<&DialogTemplate> {
        self.dialog_templates.iter().find(|t| t.dialog_id == dialog_id)
    }
}

fn ctrl(id: &str, x: i32, y: i32, w: i32, h: i32, placement: ControlPlacement) -> DialogControlDesc {
    DialogControlDesc::with_placement(id, x, y, w, h, placement)
}

/// 选图对话框 `0x6B` 控件表（壳层共用数据）。
pub fn dialog_template_0x6b() -> DialogTemplate {
    DialogTemplate {
        dialog_id: 0x6B,
        controls: vec![
            ctrl("use_map", 318, 149, 108, 23, ControlPlacement::TileSnap),
            ctrl("create_random", 318, 176, 108, 23, ControlPlacement::TileSnap),
            ctrl("cancel", 318, 269, 108, 23, ControlPlacement::BottomCoverButton),
            ctrl("title", 318, 1, 108, 10, ControlPlacement::RightPanelAnchor),
            ctrl("map_preview", 324, 23, 96, 69, ControlPlacement::RightPanelAnchor),
            ctrl("map_name_plate", 0, 0, 0, 0, ControlPlacement::MapNamePlate),
            ctrl("label_engagement", 23, 20, 257, 12, ControlPlacement::PreserveDlu),
            ctrl("label_game_type", 20, 60, 130, 10, ControlPlacement::PreserveDlu),
            ctrl("label_game_map", 168, 60, 130, 10, ControlPlacement::PreserveDlu),
            ctrl("game_type_list", 20, 78, 130, 160, ControlPlacement::PreserveDlu),
            ctrl("map_list", 168, 78, 130, 160, ControlPlacement::PreserveDlu),
            ctrl("status_help", 10, 282, 303, 12, ControlPlacement::ShellTooltip),
        ],
    }
}

/// 遭遇战大厅对话框 `0x102` 关键控件表（壳层共用数据）。
pub fn dialog_template_0x102() -> DialogTemplate {
    let mut controls = vec![
        ctrl("start", 318, 149, 108, 23, ControlPlacement::TileSnap),
        ctrl("choose_map", 318, 176, 108, 23, ControlPlacement::TileSnap),
        ctrl("back", 318, 269, 108, 23, ControlPlacement::BottomCoverButton),
        ctrl("title", 318, 1, 108, 10, ControlPlacement::RightPanelAnchor),
        ctrl("map_preview", 324, 23, 96, 69, ControlPlacement::RightPanelAnchor),
        ctrl("map_name_plate", 0, 0, 0, 0, ControlPlacement::MapNamePlate),
        ctrl("game_type", 327, 103, 90, 10, ControlPlacement::RightPanelAnchor),
        ctrl("map_label", 327, 116, 90, 20, ControlPlacement::RightPanelAnchor),
        ctrl("player_name", 35, 11, 100, 12, ControlPlacement::ComboFace),
        ctrl("checkbox_quick", 35, 145, 100, 10, ControlPlacement::PreserveDlu),
        ctrl("checkbox_1", 35, 162, 100, 10, ControlPlacement::PreserveDlu),
        ctrl("checkbox_2", 35, 179, 100, 10, ControlPlacement::PreserveDlu),
        ctrl("checkbox_3", 35, 197, 103, 10, ControlPlacement::PreserveDlu),
        ctrl("checkbox_4", 146, 196, 166, 11, ControlPlacement::PreserveDlu),
        ctrl("track_speed", 214, 145, 85, 13, ControlPlacement::PreserveDlu),
        ctrl("track_credits", 214, 162, 85, 13, ControlPlacement::PreserveDlu),
        ctrl("track_units", 214, 179, 85, 13, ControlPlacement::PreserveDlu),
        ctrl("label_speed", 146, 145, 60, 10, ControlPlacement::PreserveDlu),
        ctrl("label_credits", 146, 162, 60, 10, ControlPlacement::PreserveDlu),
        ctrl("label_units", 146, 179, 60, 10, ControlPlacement::PreserveDlu),
        ctrl("status_help", 10, 282, 303, 12, ControlPlacement::ShellTooltip),
    ];
    // 行 y DLU：本地 11，其后每行 +16。
    for i in 0..8 {
        let y = 11 + (i as i32) * 16;
        controls.push(ctrl(
            &format!("flag_{i}"),
            143,
            y,
            32,
            12,
            ControlPlacement::ComboFace,
        ));
        controls.push(ctrl(
            &format!("side_face_{i}"),
            180,
            y,
            78,
            74,
            ControlPlacement::ComboFace,
        ));
        controls.push(ctrl(
            &format!("color_face_{i}"),
            264,
            y,
            35,
            73,
            ControlPlacement::ComboFace,
        ));
    }
    for i in 0..7 {
        let y = 11 + ((i + 1) as i32) * 16;
        controls.push(ctrl(
            &format!("ai_face_{i}"),
            35,
            y,
            100,
            74,
            ControlPlacement::ComboFace,
        ));
    }
    DialogTemplate {
        dialog_id: 0x102,
        controls,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_profile_has_no_dialogs() {
        let profile = RuntimeUiProfile::default();
        assert!(profile.dialog(0x6B).is_none());
    }

    #[test]
    fn dialog_lookup_by_id() {
        let profile = RuntimeUiProfile {
            dialog_templates: vec![DialogTemplate {
                dialog_id: 0x6B,
                controls: vec![DialogControlDesc::with_placement(
                    "use_map",
                    318,
                    122,
                    108,
                    23,
                    ControlPlacement::TileSnap,
                )],
            }],
            ..RuntimeUiProfile::default()
        };
        assert_eq!(profile.dialog(0x6B).map(|t| t.controls.len()), Some(1));
        assert_eq!(
            profile.dialog(0x6B).unwrap().controls[0].placement,
            ControlPlacement::TileSnap
        );
    }

    #[test]
    fn shell_dialog_templates_expose_expected_ids() {
        assert!(dialog_template_0x6b().controls.iter().any(|c| c.id.0 == "map_list"));
        assert!(dialog_template_0x102().controls.iter().any(|c| c.id.0 == "start"));
        assert!(dialog_template_0x102().controls.iter().any(|c| c.id.0 == "flag_0"));
        assert!(dialog_template_0x102().controls.iter().any(|c| c.id.0 == "ai_face_6"));
    }
}
