//! VXL 简易等距正交投影（预览用，无光照 / 无 HVA）。

use crate::{Palette, VxlFile};

/// 投影后的精灵。
#[derive(Debug, Clone)]
pub struct VxlSprite {
    pub width: u32,
    pub height: u32,
    /// 相对投影原点：包围盒中心对齐 (0,0)。
    pub offset_x: i32,
    pub offset_y: i32,
    pub rgba: Vec<u8>,
}

/// 把全部肢节投影成一张 RGBA。
///
/// 投影：`sx = x - y`，`sy = (x + y) / 2 - z`。按深度画家算法绘制。
pub fn rasterize_vxl(vxl: &VxlFile, palette: &Palette) -> Option<VxlSprite> {
    if vxl.total_voxels() == 0 {
        return None;
    }

    let mut points: Vec<(i32, i32, i32, u8)> = Vec::new();
    for limb in &vxl.limbs {
        for v in &limb.voxels {
            let x = i32::from(v.x);
            let y = i32::from(v.y);
            let z = i32::from(v.z);
            let sx = x - y;
            let sy = (x + y) / 2 - z;
            let depth = x + y + z;
            points.push((sx, sy, depth, v.color_index));
        }
    }
    if points.is_empty() {
        return None;
    }

    let mut min_sx = i32::MAX;
    let mut min_sy = i32::MAX;
    let mut max_sx = i32::MIN;
    let mut max_sy = i32::MIN;
    for &(sx, sy, _, _) in &points {
        min_sx = min_sx.min(sx);
        min_sy = min_sy.min(sy);
        max_sx = max_sx.max(sx);
        max_sy = max_sy.max(sy);
    }

    let width = (max_sx - min_sx + 1).clamp(1, 512) as u32;
    let height = (max_sy - min_sy + 1).clamp(1, 512) as u32;
    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];

    // 远 → 近。
    points.sort_by_key(|&(sx, sy, depth, _)| (depth, sy, sx));
    for (sx, sy, _, color_index) in points {
        let c = palette.colors[color_index as usize];
        if c.a == 0 {
            continue;
        }
        let px = sx - min_sx;
        let py = sy - min_sy;
        if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
            continue;
        }
        let di = ((py as u32 * width + px as u32) * 4) as usize;
        rgba[di] = c.r;
        rgba[di + 1] = c.g;
        rgba[di + 2] = c.b;
        rgba[di + 3] = c.a;
    }

    Some(VxlSprite {
        width,
        height,
        // 包围盒中心对准放置点；壳层再平移到钻石中心。
        offset_x: -(width as i32) / 2,
        offset_y: -(height as i32) / 2,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rgba, VxlLimb, VxlVoxel};

    fn limb_with(voxels: Vec<VxlVoxel>) -> VxlFile {
        VxlFile {
            limb_count: 1,
            body_size: 0,
            limbs: vec![VxlLimb {
                name: "body".into(),
                scale: 1.0,
                size_x: 4,
                size_y: 4,
                size_z: 4,
                normals_mode: 4,
                voxels,
            }],
        }
    }

    #[test]
    fn raster_one_voxel_opaque() {
        let vxl = limb_with(vec![VxlVoxel {
            x: 1,
            y: 1,
            z: 0,
            color_index: 10,
            normal_index: 0,
        }]);
        let mut colors = [Rgba::transparent(); 256];
        colors[10] = Rgba::rgb(1, 2, 3);
        let pal = Palette { colors };
        let sprite = rasterize_vxl(&vxl, &pal).unwrap();
        assert_eq!(sprite.width, 1);
        assert_eq!(sprite.height, 1);
        assert_eq!(&sprite.rgba[..4], &[1, 2, 3, 255]);
    }
}
