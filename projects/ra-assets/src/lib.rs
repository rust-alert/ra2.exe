//! 格式解析：字节进、结构出。不碰 `std::fs`。

mod house_remap;
mod hva;
mod ini;
mod mix;
mod mix_crypto;
mod mix_hash;
mod mix_vfs;
mod pal;
mod shp;
mod tmp;
mod vxl;
mod vxl_raster;

pub use house_remap::{
    build_hsv_remap_ramp, build_remap_ramp, hsv_to_rgb, owner_primary_color, Hsv,
    HOUSE_REMAP_COUNT, HOUSE_REMAP_FIRST,
};
pub use hva::HvaFile;
pub use ini::{IniDocument, IniSection};
pub use mix::{MixArchive, MixEntry};
pub use mix_hash::mix_hash;
pub use mix_vfs::MixVfs;
pub use pal::{Palette, Rgba};
pub use shp::{ShpFile, ShpFrame};
pub use tmp::{TmpFile, TmpTile};
pub use vxl::{VxlFile, VxlLimb, VxlVoxel};
pub use vxl_raster::{
    rasterize_vxl, rasterize_vxl_frame, rasterize_vxl_layer_poses, rasterize_vxl_layers,
    rasterize_vxl_posed, VxlLayerPose, VxlSprite,
};
