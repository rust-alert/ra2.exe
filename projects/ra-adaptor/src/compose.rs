//! 可组合适配栈：基础环境 × 扩展能力。
//!
//! 心灵终结 3 等具体包不是第三轴：归入 Phobos 扩展下的内容布局（`mo_layout`）。

use std::path::Path;

use ra_types::GameEdition;

/// 基础游戏环境（与扩展正交）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseGame {
    Ra2,
    Yr,
}

/// 扩展能力来源（可多选；实现按需落地）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtensionId {
    Ares,
    Phobos,
    Kratos,
}

impl ExtensionId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ares => "ares",
            Self::Phobos => "phobos",
            Self::Kratos => "kratos",
        }
    }
}

/// 一条能力缺口或冲突报告（不得静默忽略）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityReport {
    pub code: String,
    pub message: String,
}

/// 识别并组合后的适配栈。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptorStack {
    pub base: BaseGame,
    pub extensions: Vec<ExtensionId>,
    /// MO 等内容布局快捷标记（资源表由 `ra-adaptor-phobos` 提供，非独立 adaptor 维度）。
    pub mo_layout: bool,
    /// 已检测到、但引擎尚未提供对应能力的项。
    pub unsupported: Vec<CapabilityReport>,
}

impl AdaptorStack {
    /// 由历史互斥 `GameEdition` 推导初始栈。
    pub fn from_edition(edition: GameEdition) -> Self {
        match edition {
            GameEdition::Ra2 => Self {
                base: BaseGame::Ra2,
                extensions: Vec::new(),
                mo_layout: false,
                unsupported: Vec::new(),
            },
            GameEdition::Yr => Self {
                base: BaseGame::Yr,
                extensions: Vec::new(),
                mo_layout: false,
                unsupported: Vec::new(),
            },
            GameEdition::Mo3 => Self {
                base: BaseGame::Yr,
                extensions: vec![ExtensionId::Phobos],
                mo_layout: true,
                unsupported: vec![phobos_unsupported_report()],
            },
        }
    }

    /// 映射回当前仍在用的 `GameEdition`（扩展细节不完全保留）。
    pub fn to_edition(&self) -> GameEdition {
        if self.mo_layout {
            return GameEdition::Mo3;
        }
        match self.base {
            BaseGame::Ra2 => GameEdition::Ra2,
            BaseGame::Yr => GameEdition::Yr,
        }
    }

    /// 扫描目录中的扩展与 MO 布局痕迹。
    pub fn scan_extensions(mut self, root: &Path) -> Self {
        if ra_adaptor_phobos::looks_like_mo_layout(root) {
            self.mo_layout = true;
            self.ensure_extension(ExtensionId::Phobos);
        }
        if ra_adaptor_phobos::looks_like_phobos(root) {
            self.ensure_extension(ExtensionId::Phobos);
        }

        let probes: &[(ExtensionId, &[&str])] = &[
            (ExtensionId::Ares, &["Ares.dll", "Ares.dll.inject"]),
            (ExtensionId::Kratos, &["Kratos.dll"]),
        ];
        for &(id, names) in probes {
            if names.iter().any(|n| crate::find_ci_file(root, n).is_some()) {
                self.ensure_extension(id);
            }
        }
        self
    }

    fn ensure_extension(&mut self, id: ExtensionId) {
        if self.extensions.contains(&id) {
            return;
        }
        self.extensions.push(id);
        self.unsupported.push(CapabilityReport {
            code: format!("ext.{} unsupported", id.as_str()),
            message: format!(
                "检测到 {} 相关痕迹，当前引擎尚未实现对应适配能力",
                id.as_str()
            ),
        });
    }
}

fn phobos_unsupported_report() -> CapabilityReport {
    CapabilityReport {
        code: "ext.phobos unsupported".into(),
        message: "MO/Phobos 布局已识别，当前引擎尚未实现 Phobos 扩展语义".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mo3_is_yr_plus_phobos_layout() {
        let s = AdaptorStack::from_edition(GameEdition::Mo3);
        assert_eq!(s.base, BaseGame::Yr);
        assert!(s.mo_layout);
        assert_eq!(s.extensions, vec![ExtensionId::Phobos]);
        assert_eq!(s.to_edition(), GameEdition::Mo3);
    }

    #[test]
    fn retail_roundtrip() {
        assert_eq!(
            AdaptorStack::from_edition(GameEdition::Ra2).to_edition(),
            GameEdition::Ra2
        );
        assert_eq!(
            AdaptorStack::from_edition(GameEdition::Yr).to_edition(),
            GameEdition::Yr
        );
    }
}
