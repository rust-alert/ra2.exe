//! 从 `rules.ini` 的 `[OverlayTypes]` 建立 id → 名称表。

use crate::ini::IniDocument;

/// Overlay 类型注册表（按规则编号键）。
#[derive(Debug, Clone, Default)]
pub struct OverlayTypeRegistry {
    /// `overlay_id` → 类型名（大写）。
    names: Vec<Option<String>>,
}

impl OverlayTypeRegistry {
    /// 解析 `[OverlayTypes]`；键为十进制 id。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let Some(section) = rules.sections.get("OverlayTypes") else {
            return Self::default();
        };
        let mut max_id = 0usize;
        let mut pairs: Vec<(usize, String)> = Vec::new();
        for (key, value) in &section.order {
            let Ok(id) = key.parse::<usize>() else {
                continue;
            };
            let name = value.trim();
            if name.is_empty() {
                continue;
            }
            max_id = max_id.max(id);
            pairs.push((id, name.to_ascii_uppercase()));
        }
        let mut names = vec![None; max_id.saturating_add(1)];
        for (id, name) in pairs {
            if id < names.len() {
                names[id] = Some(name);
            }
        }
        Self { names }
    }

    pub fn len(&self) -> usize {
        self.names.iter().filter(|n| n.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn name(&self, id: u8) -> Option<&str> {
        self.names
            .get(usize::from(id))
            .and_then(|n| n.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sparse_ids() {
        let doc = IniDocument::parse(
            b"[OverlayTypes]\n3=GAWALL\n105=TIB01\n1=GASAND\n",
        )
        .unwrap();
        let reg = OverlayTypeRegistry::from_rules(&doc);
        assert_eq!(reg.len(), 3);
        assert_eq!(reg.name(1), Some("GASAND"));
        assert_eq!(reg.name(3), Some("GAWALL"));
        assert_eq!(reg.name(105), Some("TIB01"));
        assert_eq!(reg.name(2), None);
    }
}
