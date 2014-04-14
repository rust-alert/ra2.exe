//! 格式解析：字节进、结构出。不碰 `std::fs`。
//!
//! 按格式族分目录：`mix` / `ini` / `image` / `voxel` / `rules`。
//! 对外仍扁平再导出，保持既有 `ra_assets::*` 路径。
//! `image` 含 PAL / SHP / TMP / Bink 容器与视频解码骨架。

#![deny(missing_docs)]

pub mod image;
pub mod ini;
pub mod mix;
pub mod rules;
pub mod voxel;

pub use image::{
    bink::{
        BINK_FLAG_ALPHA, BINK_FLAG_GRAY, BinkAudioTrack, BinkFile, BinkFrameIndexEntry,
        BinkFramePacket, BinkHeader, BinkVersion, parse_bink_file, parse_bink_header,
    },
    bink_bits::{BitReader, VlcTable, build_fixed_vlc_tables},
    bink_huff::HuffmanTree,
    bink_video::{
        BinkVideoDecoder, BinkVideoError, BinkYuvFrame, yuv420_planes_to_rgba8,
    },
    pal::{Palette, Rgba},
    shp::{ShpFile, ShpFrame, decode_rle_frame},
    tmp::{TILE_HEADER_SIZE, TmpFile, TmpTile, diamond_byte_count},
};
pub use ini::{IniDocument, IniEntry, IniSection, SourceId, SourceSpan};
pub use mix::{
    archive::{MixArchive, MixEntry},
    crypto::blowfish_decrypt_ecb,
    hash::{crc32, mix_hash, westwood_pad},
    vfs::{MixResolveHit, MixVfs},
};
pub use rules::{
    color_schemes::ColorSchemes,
    house_remap::{
        HOUSE_REMAP_COUNT, HOUSE_REMAP_FIRST, Hsv, build_hsv_remap_ramp, build_remap_ramp, hsv_to_rgb, owner_primary_color,
    },
    overlay::OverlayTypeRegistry,
    techno::{TechnoKind, TechnoType, TechnoTypeRegistry},
    warheads::{ARMOR_ORDER, Warhead, WarheadRegistry, armor_index},
};
pub use voxel::{
    hva::HvaFile,
    raster::{
        VxlLayerPose, VxlSprite, rasterize_vxl, rasterize_vxl_frame, rasterize_vxl_layer_poses, rasterize_vxl_layers,
        rasterize_vxl_posed,
    },
    vpl::VplFile,
    vxl::{VxlFile, VxlLimb, VxlVoxel},
};
