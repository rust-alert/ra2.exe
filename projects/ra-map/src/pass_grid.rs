//! 简易陆地通行格网（预览 / 仿真起步用）。

use crate::{MapEntityKind, MapInfo};

/// 地面单位相邻格允许的最大高度差（`IsoCell.z` / TMP height 粗对齐）。
pub const MAX_GROUND_CLIMB: u8 = 1;

/// 矩形通行表：`true` = 可走；附带每格高度供爬升判定。
#[derive(Debug, Clone)]
pub struct PassGrid {
    /// 宽（格）。
    pub width: u32,
    /// 高（格）。
    pub height: u32,
    passable: Vec<bool>,
    cell_height: Vec<u8>,
}

impl PassGrid {
    /// 全可走空表（高度均为 0）。
    pub fn open(width: u32, height: u32) -> Self {
        let n = (width as usize).saturating_mul(height as usize);
        Self { width, height, passable: vec![true; n], cell_height: vec![0; n] }
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

    /// 格子是否在表内。
    pub fn in_bounds(&self, x: u16, y: u16) -> bool {
        u32::from(x) < self.width && u32::from(y) < self.height
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if !self.in_bounds(x, y) {
            return None;
        }
        Some((u32::from(y) * self.width + u32::from(x)) as usize)
    }

    /// 该格是否可走。
    pub fn is_passable(&self, x: u16, y: u16) -> bool {
        self.index(x, y).and_then(|i| self.passable.get(i).copied()).unwrap_or(false)
    }

    /// 设置该格可否通行。
    pub fn set_passable(&mut self, x: u16, y: u16, passable: bool) {
        if let Some(i) = self.index(x, y) {
            if let Some(slot) = self.passable.get_mut(i) {
                *slot = passable;
            }
        }
    }

    /// 该格高度档。
    pub fn cell_height(&self, x: u16, y: u16) -> u8 {
        self.index(x, y).and_then(|i| self.cell_height.get(i).copied()).unwrap_or(0)
    }

    /// 设置该格高度档。
    pub fn set_height(&mut self, x: u16, y: u16, z: u8) {
        if let Some(i) = self.index(x, y) {
            if let Some(slot) = self.cell_height.get_mut(i) {
                *slot = z;
            }
        }
    }

    /// 相邻迈格高度差是否在地面爬升上限内。
    pub fn climb_ok(&self, from_x: u16, from_y: u16, to_x: u16, to_y: u16) -> bool {
        self.cell_height(from_x, from_y).abs_diff(self.cell_height(to_x, to_y)) <= MAX_GROUND_CLIMB
    }

    /// 不可走格子数量。
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
    pub fn find_path_diag(&self, sx: u16, sy: u16, tx: u16, ty: u16) -> Option<Vec<(u16, u16)>> {
        self.find_path_ex(sx, sy, tx, ty, true)
    }

    fn find_path_ex(&self, sx: u16, sy: u16, tx: u16, ty: u16, diagonal: bool) -> Option<Vec<(u16, u16)>> {
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
        let dirs_8: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)];
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
