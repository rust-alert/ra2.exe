//! 格式解析：字节进、结构出。不碰 `std::fs`。

#![deny(missing_docs)]

mod color_schemes;
mod house_remap;
mod hva;
mod ini;
mod mix;
mod mix_crypto;
mod mix_hash;
mod mix_vfs;
mod overlay_types;
mod pal;
mod shp;
mod techno_types;
mod tmp;
mod vpl;
mod vxl;
mod vxl_raster;
mod warheads;

pub use color_schemes::ColorSchemes;
pub use house_remap::{
    HOUSE_REMAP_COUNT, HOUSE_REMAP_FIRST, Hsv, build_hsv_remap_ramp, build_remap_ramp, hsv_to_rgb, owner_primary_color,
};
pub use hva::HvaFile;
pub use ini::{IniDocument, IniSection};
pub use mix::{MixArchive, MixEntry};
pub use mix_crypto::blowfish_decrypt_ecb;
pub use mix_hash::{crc32, mix_hash, westwood_pad};
pub use mix_vfs::MixVfs;
pub use overlay_types::OverlayTypeRegistry;
pub use pal::{Palette, Rgba};
pub use shp::{ShpFile, ShpFrame, decode_rle_frame};
pub use techno_types::{TechnoKind, TechnoType, TechnoTypeRegistry};
pub use tmp::{TILE_HEADER_SIZE, TmpFile, TmpTile, diamond_byte_count};
pub use vpl::VplFile;
pub use vxl::{VxlFile, VxlLimb, VxlVoxel};
pub use vxl_raster::{
    VxlLayerPose, VxlSprite, rasterize_vxl, rasterize_vxl_frame, rasterize_vxl_layer_poses, rasterize_vxl_layers,
    rasterize_vxl_posed,
};
pub use warheads::{ARMOR_ORDER, Warhead, WarheadRegistry, armor_index};
