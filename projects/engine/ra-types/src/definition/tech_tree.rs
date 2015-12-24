//! 科技树相关冻结定义（前置组与默认科技上限）。

/// `[General]` 通用前置组：组内任一存活建筑即可满足对应 token。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrerequisiteGroups {
    /// `PrerequisitePower` → token `POWER`。
    pub power: Vec<String>,
    /// `PrerequisiteFactory` → token `FACTORY`。
    pub factory: Vec<String>,
    /// `PrerequisiteBarracks` → token `BARRACKS`。
    pub barracks: Vec<String>,
    /// `PrerequisiteRadar` → token `RADAR`。
    pub radar: Vec<String>,
    /// `PrerequisiteTech` → token `TECH`。
    pub tech: Vec<String>,
    /// `PrerequisiteProc` → token `PROC`。
    pub proc: Vec<String>,
    /// `PrerequisiteProcAlternate`（并入 `PROC` 判定）。
    pub proc_alternate: Vec<String>,
}

impl PrerequisiteGroups {
    /// 按通用 token 名取类型键列表（大小写不敏感）。未知 token 返回空切片。
    pub fn types_for_token(&self, token: &str) -> &[String] {
        match token.trim().to_ascii_uppercase().as_str() {
            "POWER" => self.power.as_slice(),
            "FACTORY" => self.factory.as_slice(),
            "BARRACKS" => self.barracks.as_slice(),
            "RADAR" => self.radar.as_slice(),
            "TECH" => self.tech.as_slice(),
            "PROC" => {
                // PROC 主列表；alternate 由调用方与主列表一并检查。
                self.proc.as_slice()
            }
            _ => &[],
        }
    }

    /// `PROC` 判定用的全部类型键（主列表 + alternate）。
    pub fn proc_all(&self) -> impl Iterator<Item = &str> {
        self.proc
            .iter()
            .chain(self.proc_alternate.iter())
            .map(String::as_str)
    }
}
