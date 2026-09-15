//! 载具 VXL 分图层诊断：车身 / 炮塔 / 炮管 / 合成 / 落影的尺寸与原点偏移。
//!
//! 用于核对朝向坐标系、模型原点锚点与附属层资源命中，不改变叠画路径。

use ra_assets::{
    HvaFile, Palette, VplFile, VxlFile, VxlLayerPose, VxlSprite, rasterize_vxl_layer_poses, rasterize_vxl_shadow_layer_poses,
};
use ra_types::AssetSource;

use crate::iso_math::{TILE_HEIGHT, TILE_WIDTH};

/// 载具附属 VXL 后缀候选（炮塔 → 炮管）。命中以 `bar` 开头的后缀后停止继续尝试。
pub const MOBILE_VXL_TURRET_SUFFIXES: &[&str] = &["tur", "barl", "barrel"];

/// 单层（或合成 / 落影）诊断行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileVxlLayerDiag {
    /// `body` / `tur` / `barl` / `barrel` / `composed` / `shadow`。
    pub role: String,
    /// 逻辑 VXL 名；合成 / 落影可为空。
    pub vxl_name: String,
    /// 逻辑 HVA 名；无或未用可为空。
    pub hva_name: String,
    /// VXL 是否可读。
    pub vxl_hit: bool,
    /// HVA 是否可读。
    pub hva_hit: bool,
    /// 本层光栅朝向；合成层在车身/炮塔朝向不一致时为 `None`。
    pub facing: Option<u8>,
    /// 精灵宽；未光栅则为 `None`。
    pub width: Option<u32>,
    /// 精灵高。
    pub height: Option<u32>,
    /// 光栅器相对模型原点的 `offset_x`（未加钻石中心）。
    pub offset_x: Option<i32>,
    /// 光栅器相对模型原点的 `offset_y`。
    pub offset_y: Option<i32>,
    /// 叠画用：`offset_x + TILE_WIDTH/2`。
    pub cell_offset_x: Option<i32>,
    /// 叠画用：`offset_y + TILE_HEIGHT/2`。
    pub cell_offset_y: Option<i32>,
    /// 模型原点在精灵内的像素 X（`-offset_x`）；原点锚点路径下应落在精灵内或边界。
    pub origin_px: Option<i32>,
    /// 模型原点在精灵内的像素 Y（`-offset_y`）。
    pub origin_py: Option<i32>,
}

/// 一次载具 VXL 诊断报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileVxlDiagReport {
    /// 资源词干（小写，如 `mtnk`）。
    pub stem: String,
    /// 车身朝向字节。
    pub body_facing: u8,
    /// 炮塔 / 炮管朝向字节。
    pub turret_facing: u8,
    /// HVA 帧。
    pub hva_frame: u32,
    /// 分图层与合成 / 落影行。
    pub layers: Vec<MobileVxlLayerDiag>,
    /// 人类可读备注（缺 body、缺调色板等）。
    pub notes: Vec<String>,
}

/// 已加载的一层移动单位 VXL（供叠画与诊断共用）。
#[derive(Debug)]
pub(crate) struct LoadedMobileVxlLayer {
    pub role: &'static str,
    pub vxl_name: String,
    pub hva_name: String,
    pub vxl: VxlFile,
    pub hva: Option<HvaFile>,
    pub hva_hit: bool,
    pub is_turret: bool,
}

/// 收集车身及附属层；车身 VXL 缺失时返回 `None`。
pub(crate) fn collect_mobile_vxl_layers(source: &dyn AssetSource, stem: &str) -> Option<Vec<LoadedMobileVxlLayer>> {
    let stem = stem.to_ascii_lowercase();
    let body_name = format!("{stem}.vxl");
    let body_bytes = source.read(&body_name).ok()?;
    let body = VxlFile::parse(&body_bytes).ok()?;
    let body_hva_name = format!("{stem}.hva");
    let body_hva_bytes = source.read(&body_hva_name).ok();
    let body_hva_hit = body_hva_bytes.is_some();
    let body_hva = body_hva_bytes.and_then(|b| HvaFile::parse(&b).ok());

    let mut owned = vec![LoadedMobileVxlLayer {
        role: "body",
        vxl_name: body_name,
        hva_name: body_hva_name,
        vxl: body,
        hva: body_hva,
        hva_hit: body_hva_hit,
        is_turret: false,
    }];

    for suffix in MOBILE_VXL_TURRET_SUFFIXES {
        let vxl_name = format!("{stem}{suffix}.vxl");
        let Ok(bytes) = source.read(&vxl_name) else {
            continue;
        };
        let Ok(vxl) = VxlFile::parse(&bytes) else {
            continue;
        };
        let hva_name = format!("{stem}{suffix}.hva");
        let hva_bytes = source.read(&hva_name).ok();
        let hva_hit = hva_bytes.is_some();
        let hva = hva_bytes.and_then(|b| HvaFile::parse(&b).ok());
        owned.push(LoadedMobileVxlLayer {
            role: suffix,
            vxl_name,
            hva_name,
            vxl,
            hva,
            hva_hit,
            is_turret: true,
        });
        if suffix.starts_with("bar") {
            break;
        }
    }
    Some(owned)
}

fn metrics_from_sprite(role: &str, vxl_name: &str, hva_name: &str, vxl_hit: bool, hva_hit: bool, facing: Option<u8>, sprite: &VxlSprite) -> MobileVxlLayerDiag {
    MobileVxlLayerDiag {
        role: role.to_string(),
        vxl_name: vxl_name.to_string(),
        hva_name: hva_name.to_string(),
        vxl_hit,
        hva_hit,
        facing,
        width: Some(sprite.width),
        height: Some(sprite.height),
        offset_x: Some(sprite.offset_x),
        offset_y: Some(sprite.offset_y),
        cell_offset_x: Some(sprite.offset_x + TILE_WIDTH / 2),
        cell_offset_y: Some(sprite.offset_y + TILE_HEIGHT / 2),
        origin_px: Some(-sprite.offset_x),
        origin_py: Some(-sprite.offset_y),
    }
}

fn miss_row(role: &str, vxl_name: &str, hva_name: &str, vxl_hit: bool, hva_hit: bool, facing: Option<u8>) -> MobileVxlLayerDiag {
    MobileVxlLayerDiag {
        role: role.to_string(),
        vxl_name: vxl_name.to_string(),
        hva_name: hva_name.to_string(),
        vxl_hit,
        hva_hit,
        facing,
        width: None,
        height: None,
        offset_x: None,
        offset_y: None,
        cell_offset_x: None,
        cell_offset_y: None,
        origin_px: None,
        origin_py: None,
    }
}

/// 对词干做分图层光栅诊断（只读，不写盘）。
///
/// 缺车身 VXL 时仍返回报告，`notes` 说明原因。调色板优先 `unittem.pal`。
pub fn diagnose_mobile_vxl(
    source: &dyn AssetSource,
    stem: &str,
    body_facing: u8,
    turret_facing: u8,
    hva_frame: u32,
) -> MobileVxlDiagReport {
    let stem = stem.to_ascii_lowercase();
    let mut notes = Vec::new();
    let mut layers = Vec::new();

    let Some(owned) = collect_mobile_vxl_layers(source, &stem) else {
        let body_name = format!("{stem}.vxl");
        let hva_name = format!("{stem}.hva");
        let vxl_hit = source.read(&body_name).is_ok();
        let hva_hit = source.read(&hva_name).is_ok();
        notes.push(if vxl_hit {
            format!("无法解析车身 VXL：{body_name}")
        } else {
            format!("缺少车身 VXL：{body_name}")
        });
        layers.push(miss_row("body", &body_name, &hva_name, vxl_hit, hva_hit, Some(body_facing)));
        for suffix in MOBILE_VXL_TURRET_SUFFIXES {
            let vxl_name = format!("{stem}{suffix}.vxl");
            let hva_name = format!("{stem}{suffix}.hva");
            let vxl_hit = source.read(&vxl_name).is_ok();
            let hva_hit = source.read(&hva_name).is_ok();
            if vxl_hit || hva_hit {
                layers.push(miss_row(suffix, &vxl_name, &hva_name, vxl_hit, hva_hit, Some(turret_facing)));
            }
            if vxl_hit && suffix.starts_with("bar") {
                break;
            }
        }
        return MobileVxlDiagReport { stem, body_facing, turret_facing, hva_frame, layers, notes };
    };

    let pal = match source.read("unittem.pal").ok().and_then(|b| Palette::parse(&b).ok()) {
        Some(p) => p,
        None => {
            notes.push("缺少或无法解析 unittem.pal，跳过光栅".into());
            for layer in &owned {
                let facing = if layer.is_turret { turret_facing } else { body_facing };
                layers.push(miss_row(layer.role, &layer.vxl_name, &layer.hva_name, true, layer.hva_hit, Some(facing)));
            }
            return MobileVxlDiagReport { stem, body_facing, turret_facing, hva_frame, layers, notes };
        }
    };
    let vpl = source.read("voxels.vpl").ok().and_then(|b| VplFile::parse(&b).ok());
    if vpl.is_none() {
        notes.push("缺少或无法解析 voxels.vpl，继续无 VPL 光栅".into());
    }

    let poses: Vec<VxlLayerPose<'_>> = owned
        .iter()
        .map(|layer| VxlLayerPose {
            vxl: &layer.vxl,
            hva: layer.hva.as_ref(),
            facing: if layer.is_turret { turret_facing } else { body_facing },
            frame: hva_frame,
        })
        .collect();

    for (layer, pose) in owned.iter().zip(poses.iter()) {
        match rasterize_vxl_layer_poses(std::slice::from_ref(pose), &pal, vpl.as_ref()) {
            Some(sprite) => {
                layers.push(metrics_from_sprite(
                    layer.role,
                    &layer.vxl_name,
                    &layer.hva_name,
                    true,
                    layer.hva_hit,
                    Some(pose.facing),
                    &sprite,
                ));
            }
            None => {
                notes.push(format!("{} 光栅为空：{}", layer.role, layer.vxl_name));
                layers.push(miss_row(layer.role, &layer.vxl_name, &layer.hva_name, true, layer.hva_hit, Some(pose.facing)));
            }
        }
    }

    let composed_facing = if body_facing == turret_facing { Some(body_facing) } else { None };
    match rasterize_vxl_layer_poses(&poses, &pal, vpl.as_ref()) {
        Some(sprite) => {
            layers.push(metrics_from_sprite("composed", "", "", true, true, composed_facing, &sprite));
        }
        None => {
            notes.push("合成光栅为空".into());
            layers.push(miss_row("composed", "", "", true, true, composed_facing));
        }
    }

    if let Some(body_pose) = poses.first() {
        match rasterize_vxl_shadow_layer_poses(std::slice::from_ref(body_pose)) {
            Some(sprite) => {
                layers.push(metrics_from_sprite("shadow", &owned[0].vxl_name, &owned[0].hva_name, true, owned[0].hva_hit, Some(body_facing), &sprite));
            }
            None => {
                notes.push("落影光栅为空".into());
                layers.push(miss_row("shadow", &owned[0].vxl_name, &owned[0].hva_name, true, owned[0].hva_hit, Some(body_facing)));
            }
        }
    }

    let tur_hit = owned.iter().any(|l| l.role == "tur");
    let bar_hit = owned.iter().any(|l| l.role.starts_with("bar"));
    if !tur_hit {
        notes.push("未命中炮塔后缀 tur".into());
    }
    if tur_hit && !bar_hit {
        notes.push("已命中炮塔但未命中炮管后缀 barl/barrel".into());
    }

    MobileVxlDiagReport { stem, body_facing, turret_facing, hva_frame, layers, notes }
}

/// 常用朝向扫表：`0, 32, …, 224`（八个字节桶中心，便于与 SHP 八向对照）。
pub fn mobile_vxl_diag_facing_sweep_bytes() -> [u8; 8] {
    [0, 32, 64, 96, 128, 160, 192, 224]
}

/// 固定炮塔朝向、扫车身朝向。
pub fn diagnose_mobile_vxl_sweep_body(
    source: &dyn AssetSource,
    stem: &str,
    turret_facing: u8,
    hva_frame: u32,
) -> Vec<MobileVxlDiagReport> {
    mobile_vxl_diag_facing_sweep_bytes()
        .into_iter()
        .map(|body_facing| diagnose_mobile_vxl(source, stem, body_facing, turret_facing, hva_frame))
        .collect()
}

/// 固定车身朝向、扫炮塔朝向。
pub fn diagnose_mobile_vxl_sweep_turret(
    source: &dyn AssetSource,
    stem: &str,
    body_facing: u8,
    hva_frame: u32,
) -> Vec<MobileVxlDiagReport> {
    mobile_vxl_diag_facing_sweep_bytes()
        .into_iter()
        .map(|turret_facing| diagnose_mobile_vxl(source, stem, body_facing, turret_facing, hva_frame))
        .collect()
}

/// 默认 HVA 帧扫表：`0, 1, 2`（核对帧变化时脚点 / 落影是否漂移）。
pub fn mobile_vxl_diag_hva_sweep_frames() -> [u32; 3] {
    [0, 1, 2]
}

/// 固定车身 / 炮塔朝向、扫 HVA 帧（`0..frame_count`，至少扫一帧）。
pub fn diagnose_mobile_vxl_sweep_hva(
    source: &dyn AssetSource,
    stem: &str,
    body_facing: u8,
    turret_facing: u8,
    frame_count: u32,
) -> Vec<MobileVxlDiagReport> {
    let n = frame_count.max(1);
    (0..n).map(|hva_frame| diagnose_mobile_vxl(source, stem, body_facing, turret_facing, hva_frame)).collect()
}
