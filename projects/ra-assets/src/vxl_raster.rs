//! VXL 简易等距正交投影（预览用，无光照）。

use crate::{HvaFile, Palette, VxlFile};

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

/// 无姿态投影（等价于 facing=0、无 HVA）。
pub fn rasterize_vxl(vxl: &VxlFile, palette: &Palette) -> Option<VxlSprite> {
    rasterize_vxl_posed(vxl, palette, None, 0)
}

/// 按朝向投影；若提供 HVA，则用 `facing/32` 选取帧并应用肢节矩阵。
pub fn rasterize_vxl_posed(
    vxl: &VxlFile,
    palette: &Palette,
    hva: Option<&HvaFile>,
    facing: u8,
) -> Option<VxlSprite> {
    if vxl.total_voxels() == 0 {
        return None;
    }

    let frame = match hva {
        Some(h) if h.frame_count > 0 => {
            u32::from(facing / 32) % h.frame_count
        }
        _ => 0,
    };

    let mut points: Vec<(i32, i32, i32, u8)> = Vec::new();
    for (section, limb) in vxl.limbs.iter().enumerate() {
        let matrix = hva.and_then(|h| h.get_transform(frame, section as u32));
        for v in &limb.voxels {
            let (x, y, z) = match matrix {
                Some(m) => apply_matrix(m, f32::from(v.x), f32::from(v.y), f32::from(v.z)),
                None => {
                    // 无 HVA：绕肢节中心做 8 向偏航近似。
                    yaw_point(
                        f32::from(v.x),
                        f32::from(v.y),
                        f32::from(v.z),
                        f32::from(limb.size_x) * 0.5,
                        f32::from(limb.size_y) * 0.5,
                        facing,
                    )
                }
            };
            let xi = x.round() as i32;
            let yi = y.round() as i32;
            let zi = z.round() as i32;
            let sx = xi - yi;
            let sy = (xi + yi) / 2 - zi;
            let depth = xi + yi + zi;
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
        offset_x: -(width as i32) / 2,
        offset_y: -(height as i32) / 2,
        rgba,
    })
}

fn apply_matrix(m: &[f32; 12], x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    (
        m[0] * x + m[1] * y + m[2] * z + m[3],
        m[4] * x + m[5] * y + m[6] * z + m[7],
        m[8] * x + m[9] * y + m[10] * z + m[11],
    )
}

fn yaw_point(x: f32, y: f32, z: f32, cx: f32, cy: f32, facing: u8) -> (f32, f32, f32) {
    let steps = facing / 32;
    if steps == 0 {
        return (x, y, z);
    }
    let angle = f32::from(steps) * std::f32::consts::FRAC_PI_4;
    let (s, c) = angle.sin_cos();
    let dx = x - cx;
    let dy = y - cy;
    (c * dx - s * dy + cx, s * dx + c * dy + cy, z)
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

    #[test]
    fn posed_with_identity_hva_matches() {
        let vxl = limb_with(vec![VxlVoxel {
            x: 2,
            y: 0,
            z: 0,
            color_index: 10,
            normal_index: 0,
        }]);
        let mut colors = [Rgba::transparent(); 256];
        colors[10] = Rgba::rgb(9, 9, 9);
        let pal = Palette { colors };
        let hva = HvaFile {
            frame_count: 1,
            section_count: 1,
            section_names: vec!["body".into()],
            transforms: vec![[
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0,
            ]],
        };
        let a = rasterize_vxl(&vxl, &pal).unwrap();
        let b = rasterize_vxl_posed(&vxl, &pal, Some(&hva), 0).unwrap();
        assert_eq!(a.width, b.width);
        assert_eq!(a.height, b.height);
        assert_eq!(a.rgba, b.rgba);
    }
}
