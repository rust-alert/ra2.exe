//! 集成测试：原 `src/vxl_raster.rs` 内联测试迁出。

use ra_assets::*;

fn limb_with(voxels: Vec<VxlVoxel>) -> VxlFile {
    VxlFile {
        limb_count: 1,
        body_size: 0,
        limbs: vec![VxlLimb {
            name: "body".into(),
            scale: 1.0,
            bounds: [0.0, 0.0, 0.0, 4.0, 4.0, 4.0],
            transform: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0,
            ],
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
    let vxl = limb_with(vec![VxlVoxel { x: 1, y: 1, z: 0, color_index: 10, normal_index: 0 }]);
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
    let vxl = limb_with(vec![VxlVoxel { x: 2, y: 0, z: 0, color_index: 10, normal_index: 0 }]);
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

#[test]
fn yaw_facing_changes_bounds() {
    let voxels: Vec<_> = (0..8).map(|i| VxlVoxel { x: i, y: 0, z: 0, color_index: 10, normal_index: 0 }).collect();
    let vxl = limb_with(voxels);
    let mut colors = [Rgba::transparent(); 256];
    colors[10] = Rgba::rgb(1, 1, 1);
    let pal = Palette { colors };
    let a = rasterize_vxl_posed(&vxl, &pal, None, 0).unwrap();
    let b = rasterize_vxl_posed(&vxl, &pal, None, 32).unwrap();
    assert!(a.width > b.width);
}

#[test]
fn hva_translation_scaled_by_limb_scale() {
    let mut vxl = limb_with(vec![VxlVoxel { x: 0, y: 0, z: 0, color_index: 10, normal_index: 0 }]);
    vxl.limbs[0].scale = 0.5;
    let mut colors = [Rgba::transparent(); 256];
    colors[10] = Rgba::rgb(1, 1, 1);
    let pal = Palette { colors };
    let hva = HvaFile {
        frame_count: 1,
        section_count: 1,
        section_names: vec!["body".into()],
        transforms: vec![[
            1.0, 0.0, 0.0, 10.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ]],
    };
    let s = rasterize_vxl_posed(&vxl, &pal, Some(&hva), 0).unwrap();
    assert_eq!(s.width, 1);
    assert_eq!(s.height, 1);
}

#[test]
fn vpl_shades_by_normal_page() {
    let vxl = limb_with(vec![VxlVoxel { x: 1, y: 1, z: 0, color_index: 10, normal_index: 255 }]);
    let mut colors = [Rgba::transparent(); 256];
    colors[10] = Rgba::rgb(10, 10, 10);
    colors[42] = Rgba::rgb(42, 0, 0);
    let pal = Palette { colors };
    let mut data = Vec::new();
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&31u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 768]);
    let mut page0 = [0u8; 256];
    page0[10] = 10;
    let mut page1 = [0u8; 256];
    page1[10] = 42;
    data.extend_from_slice(&page0);
    data.extend_from_slice(&page1);
    let vpl = VplFile::parse(&data).unwrap();
    let pose = VxlLayerPose { vxl: &vxl, hva: None, facing: 0, frame: 0 };
    let sprite = rasterize_vxl_layer_poses(&[pose], &pal, Some(&vpl)).unwrap();
    assert_eq!(&sprite.rgba[..4], &[42, 0, 0, 255]);
}
