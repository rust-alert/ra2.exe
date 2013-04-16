//! 简易陆地通行格网（预览 / 仿真起步用）。

use crate::{MapEntityKind, MapInfo};

/// 矩形通行表：`true` = 可走。
#[derive(Debug, Clone)]
pub struct PassGrid {
    pub width: u32,
    pub height: u32,
    passable: Vec<bool>,
}

impl PassGrid {
    /// 全可走空表。
    pub fn open(width: u32, height: u32) -> Self {
        let n = (width as usize).saturating_mul(height as usize);
        Self {
            width,
            height,
            passable: vec![true; n],
        }
    }

    /// 由地图尺寸建表，并用建筑 / 地形物件占用格封死。
    pub fn from_map(map: &MapInfo) -> Self {
        let mut grid = Self::open(map.width.max(1), map.height.max(1));
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

    pub fn is_passable(&self, x: u16, y: u16) -> bool {
        if !self.in_bounds(x, y) {
            return false;
        }
        let i = (u32::from(y) * self.width + u32::from(x)) as usize;
        self.passable.get(i).copied().unwrap_or(false)
    }

    pub fn set_passable(&mut self, x: u16, y: u16, passable: bool) {
        if !self.in_bounds(x, y) {
            return;
        }
        let i = (u32::from(y) * self.width + u32::from(x)) as usize;
        if let Some(slot) = self.passable.get_mut(i) {
            *slot = passable;
        }
    }

    pub fn blocked_count(&self) -> usize {
        self.passable.iter().filter(|p| !**p).count()
    }

    /// 四邻 BFS；不可达则 `None`。路径含起点，末元为目标。
    pub fn find_path(&self, sx: u16, sy: u16, tx: u16, ty: u16) -> Option<Vec<(u16, u16)>> {
        if !self.is_passable(sx, sy) || !self.is_passable(tx, ty) {
            return None;
        }
        if sx == tx && sy == ty {
            return Some(vec![(sx, sy)]);
        }
        let w = self.width as usize;
        let h = self.height as usize;
        let idx = |x: u16, y: u16| -> usize { y as usize * w + x as usize };
        let mut prev: Vec<Option<(u16, u16)>> = vec![None; w * h];
        let mut visited = vec![false; w * h];
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((sx, sy));
        visited[idx(sx, sy)] = true;
        let dirs: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        while let Some((x, y)) = queue.pop_front() {
            for (dx, dy) in dirs {
                let nx = i32::from(x) + dx;
                let ny = i32::from(y) + dy;
                if nx < 0 || ny < 0 || nx as u32 >= self.width || ny as u32 >= self.height {
                    continue;
                }
                let nx = nx as u16;
                let ny = ny as u16;
                let i = idx(nx, ny);
                if visited[i] || !self.is_passable(nx, ny) {
                    continue;
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
    use crate::{MapEntity, MapEntityKind};
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
}
