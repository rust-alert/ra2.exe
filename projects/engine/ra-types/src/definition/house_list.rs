//! 阵营 / 房屋允许名单（`Owner=` / `RequiredHouses=` / `ForbiddenHouses=`）。

/// 装载期解析后的房屋名单；空名单语义由字段约定（见各字段文档）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HouseAllowList {
    /// 大写房屋 id；空 = 本名单无约束项。
    houses: Vec<String>,
}

impl HouseAllowList {
    /// 空名单。
    pub fn empty() -> Self {
        Self { houses: Vec::new() }
    }

    /// 由已归一化（大写、非空）房屋 id 构造。
    pub fn from_houses(houses: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut houses: Vec<String> = houses
            .into_iter()
            .map(|h| h.into().trim().to_ascii_uppercase())
            .filter(|h| !h.is_empty())
            .collect();
        houses.sort_unstable();
        houses.dedup();
        Self { houses }
    }

    /// 解析 `Owner=` 原文：按 `,` / `;` / `|` 拆分；空串 → 空名单（表示不限）。
    pub fn parse_owner(raw: &str) -> Self {
        Self::parse_delimited(raw)
    }

    /// 解析 `RequiredHouses=` / `ForbiddenHouses=` 逗号列表（已 trim 的 token 亦可直接传入拼接串）。
    pub fn parse_csv(raw: &str) -> Self {
        Self::parse_delimited(raw)
    }

    /// 由已拆好的 token 列表构造（装载 registry 已大写拆分时用）。
    pub fn from_tokens(tokens: &[String]) -> Self {
        Self::from_houses(tokens.iter().cloned())
    }

    fn parse_delimited(raw: &str) -> Self {
        let raw = raw.trim();
        if raw.is_empty() {
            return Self::empty();
        }
        Self::from_houses(
            raw.split(|c| c == ',' || c == ';' || c == '|')
                .map(str::trim)
                .filter(|s| !s.is_empty()),
        )
    }

    /// 是否为空名单。
    pub fn is_empty(&self) -> bool {
        self.houses.is_empty()
    }

    /// 名单长度。
    pub fn len(&self) -> usize {
        self.houses.len()
    }

    /// 迭代房屋 id。
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.houses.iter().map(String::as_str)
    }

    /// `Owner=` 语义：空名单 = 不限；否则 house 须命中其一。
    pub fn owner_allows(&self, house: &str) -> bool {
        if self.houses.is_empty() {
            return true;
        }
        self.contains(house)
    }

    /// `RequiredHouses=` 语义：空名单 = 不限制；非空则须命中。
    pub fn required_allows(&self, house: &str) -> bool {
        if self.houses.is_empty() {
            return true;
        }
        self.contains(house)
    }

    /// `ForbiddenHouses=` 语义：命中任一则禁止；空名单 = 不禁止。
    pub fn forbids(&self, house: &str) -> bool {
        !self.houses.is_empty() && self.contains(house)
    }

    fn contains(&self, house: &str) -> bool {
        let want = house.trim().to_ascii_uppercase();
        self.houses.iter().any(|h| h == &want)
    }
}
