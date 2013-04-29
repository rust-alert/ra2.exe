//! 简易陆地通行格网（预览 / 仿真起步用）。

use crate::{MapEntityKind, MapInfo};

/// 地面单位相邻格允许的最大高度差（`IsoCell.z` / TMP height 粗对齐）。
pub const MAX_GROUND_CLIMB: u8 = 1;

/// 矩形通行表：`true` = 可走；附带每格高度供爬升判定。
#[derive(Debug, Clone)]
pub struct PassGrid {
    pub width: u32,
    pub height: u32,
    passable: Vec<bool>,
    cell_height: Vec<u8>,
}

impl PassGrid {
    /// 全可走空表（高度均为 0）。
    pub fn open(width: u32, height: u32) -> Self {
        let n = (width as usize).saturating_mul(height as usize);
        Self {
            width,
            height,
            passable: vec![true; n],
            cell_height: vec![0; n],
        }
    }

    /// 由地图尺寸建表：灌入 `IsoCell.z`，并用建筑 / 地形物件占用格封死。
    pub fn from_map(map: &MapInfo) -> Self {
        let mut grid = Self::open(map.width.max(1), map.height.max(1));
        for cell in &map.cells {
            if cell.x < 0 || cell.y < 0 {
                continue;
            }
            grid.set_height(cell.x as u16, cell.y as u16, cell.z);
        }
        for e in &map.entities {
            if e.kind == MapEntityKind::Structure {
                grid.set_passable(e.x, e.y, false);
            }
        }
        for t in &map.terrain_objects {
            grid.set_passable(t.x, t.y, false);
        }
        grid
    }

    pub fn in_bounds(&self, x: u16, y: u16) -> bool {
        u32::from(x) < self.width && u32::from(y) < self.height
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if !self.in_bounds(x, y) {
            return None;
        }
        Some((u32::from(y) * self.width + u32::from(x)) as usize)
    }

    pub fn is_passable(&self, x: u16, y: u16) -> bool {
        self.index(x, y)
            .and_then(|i| self.passable.get(i).copied())
            .unwrap_or(false)
    }

    pub fn set_passable(&mut self, x: u16, y: u16, passable: bool) {
        if let Some(i) = self.index(x, y) {
            if let Some(slot) = self.passable.get_mut(i) {
                *slot = passable;
            }
        }
    }

    pub fn cell_height(&self, x: u16, y: u16) -> u8 {
        self.index(x, y)
            .and_then(|i| self.cell_height.get(i).copied())
            .unwrap_or(0)
    }

    pub fn set_height(&mut self, x: u16, y: u16, z: u8) {
        if let Some(i) = self.index(x, y) {
            if let Some(slot) = self.cell_height.get_mut(i) {
                *slot = z;
            }
        }
    }

    /// 相邻迈格高度差是否在地面爬升上限内。
    pub fn climb_ok(&self, from_x: u16, from_y: u16, to_x: u16, to_y: u16) -> bool {
        self.cell_height(from_x, from_y)
            .abs_diff(self.cell_height(to_x, to_y))
            <= MAX_GROUND_CLIMB
    }

    pub fn blocked_count(&self) -> usize {
        self.passable.iter().filter(|p| !**p).count()
    }

    /// 按 TMP `terrain_type` 封死不可走陆地（水/岩/墙）。
    pub fn seal_land_type(&mut self, x: u16, y: u16, terrain_type: u8) {
        if !crate::ground_passable(terrain_type) {
            self.set_passable(x, y, false);
        }
    }

    /// 对一批 `(x,y,terrain_type)` 封格；返回新封死数量（原本已不可走的不计）。
    pub fn seal_land_types(&mut self, cells: &[(u16, u16, u8)]) -> usize {
        let mut n = 0;
        for &(x, y, tt) in cells {
            if !self.in_bounds(x, y) || crate::ground_passable(tt) {
                continue;
            }
            if self.is_passable(x, y) {
                self.set_passable(x, y, false);
                n += 1;
            }
        }
        n
    }

    /// 四邻 BFS；不可达则 `None`。路径含起点，末元为目标。
    pub fn find_path(&self, sx: u16, sy: u16, tx: u16, ty: u16) -> Option<Vec<(u16, u16)>> {
        self.find_path_ex(sx, sy, tx, ty, false)
    }

    /// 八邻 BFS（对角线需两侧正交格可走，避免穿角）。
    pub fn find_path_diag(
        &self,
        sx: u16,
        sy: u16,
        tx: u16,
        ty: u16,
    ) -> Option<Vec<(u16, u16)>> {
        self.find_path_ex(sx, sy, tx, ty, true)
    }

    fn find_path_ex(
        &self,
        sx: u16,
        sy: u16,
        tx: u16,
        ty: u16,
        diagonal: bool,
    ) -> Option<Vec<(u16, u16)>> {
        if !self.is_passable(sx, sy) || !self.is_passable(tx, ty) {
            return None;
        }
        if sx == tx && sy == ty {
            return Some(vec![(sx, sy)]);
        }
        let w = self.width as usize;
        let idx = |x: u16, y: u16| -> usize { y as usize * w + x as usize };
        let mut prev: Vec<Option<(u16, u16)>> = vec![None; w * self.height as usize];
        let mut visited = vec![false; w * self.height as usize];
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((sx, sy));
        visited[idx(sx, sy)] = true;
        let dirs_4: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        let dirs_8: [(i32, i32); 8] = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];
        while let Some((x, y)) = queue.pop_front() {
            let dirs: &[(i32, i32)] = if diagonal { &dirs_8 } else { &dirs_4 };
            for &(dx, dy) in dirs {
                let nx = i32::from(x) + dx;
                let ny = i32::from(y) + dy;
                if nx < 0 || ny < 0 || nx as u32 >= self.width || ny as u32 >= self.height {
                    continue;
                }
                let nx = nx as u16;
                let ny = ny as u16;
                let i = idx(nx, ny);
                if visited[i] || !self.is_passable(nx, ny) || !self.climb_ok(x, y, nx, ny) {
                    continue;
                }
                // 对角线：两侧正交格也须可走，且爬升可达。
                if dx != 0 && dy != 0 {
                    let ox = (i32::from(x) + dx) as u16;
                    let oy = (i32::from(y) + dy) as u16;
                    if !self.is_passable(ox, y)
                        || !self.is_passable(x, oy)
                        || !self.climb_ok(x, y, ox, y)
                        || !self.climb_ok(x, y, x, oy)
                    {
                        continue;
                    }
                }
                visited[i] = true;
                prev[i] = Some((x, y));
                if nx == tx && ny == ty {
                    let mut path = vec![(tx, ty)];
                    let mut cur = (tx, ty);
                    while cur != (sx, sy) {
                        cur = prev[idx(cur.0, cur.1)].unwrap();
                        path.push(cur);
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back((nx, ny));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IsoCell, MapEntity, MapEntityKind};
    use ra_types::GameEdition;

    #[test]
    fn structures_block_and_bfs_detours() {
        let mut map = MapInfo::empty(GameEdition::Ra2, "t");
        map.width = 5;
        map.height = 3;
        map.entities.push(MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Neutral".into(),
            type_id: "GAWALL".into(),
            health: 256,
            x: 2,
            y: 1,
            facing: 0,
            sub_cell: 0,
        });
        let grid = PassGrid::from_map(&map);
        assert!(!grid.is_passable(2, 1));
        assert_eq!(grid.blocked_count(), 1);
        let path = grid.find_path(0, 1, 4, 1).unwrap();
        assert_eq!(path.first(), Some(&(0, 1)));
        assert_eq!(path.last(), Some(&(4, 1)));
        assert!(!path.iter().any(|&(x, y)| x == 2 && y == 1));
    }

    #[test]
    fn diagonal_path_is_shorter() {
        let grid = PassGrid::open(5, 5);
        let ortho = grid.find_path(0, 0, 2, 2).unwrap();
        let diag = grid.find_path_diag(0, 0, 2, 2).unwrap();
        assert_eq!(ortho.len(), 5); // (0,0)(1,0)(2,0)(2,1)(2,2) or similar
        assert_eq!(diag.len(), 3); // (0,0)(1,1)(2,2)
    }

    #[test]
    fn seal_water_land_types() {
        let mut grid = PassGrid::open(3, 3);
        let sealed = grid.seal_land_types(&[(1, 1, 3), (0, 0, 0)]);
        assert_eq!(sealed, 1);
        assert!(!grid.is_passable(1, 1));
        assert!(grid.is_passable(0, 0));
    }

    #[test]
    fn cliff_blocks_path_ramp_allows() {
        let mut cliff = PassGrid::open(3, 1);
        cliff.set_height(0, 0, 0);
        cliff.set_height(1, 0, 2);
        cliff.set_height(2, 0, 2);
        assert!(cliff.find_path(0, 0, 2, 0).is_none());

        let mut ramp = PassGrid::open(3, 1);
        ramp.set_height(0, 0, 0);
        ramp.set_height(1, 0, 1);
        ramp.set_height(2, 0, 2);
        assert!(ramp.find_path(0, 0, 2, 0).is_some());
    }

    #[test]
    fn from_map_loads_iso_heights() {
        let mut map = MapInfo::empty(GameEdition::Ra2, "t");
        map.width = 2;
        map.height = 1;
        map.cells.push(IsoCell {
            x: 0,
            y: 0,
            tile_num: 0,
            sub_tile: 0,
            z: 3,
            flags: 0,
        });
        map.cells.push(IsoCell {
            x: 1,
            y: 0,
            tile_num: 0,
            sub_tile: 0,
            z: 4,
            flags: 0,
        });
        let grid = PassGrid::from_map(&map);
        assert_eq!(grid.cell_height(0, 0), 3);
        assert_eq!(grid.cell_height(1, 0), 4);
        assert!(grid.find_path(0, 0, 1, 0).is_some());
    }
}
