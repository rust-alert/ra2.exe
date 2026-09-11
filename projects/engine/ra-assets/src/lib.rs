//! 格式解析：字节进、结构出。不碰 `std::fs`。
//!
//! 按格式族分目录：`mix` / `ini` / `image` / `voxel` / `rules` / `audio`。
//! 对外仍扁平再导出，保持既有 `ra_assets::*` 路径。
//! `image` 含 PAL / SHP / TMP / Bink 容器与视频解码骨架。

#![deny(missing_docs)]

pub mod audio;
pub mod battle;
pub mod image;
pub mod ini;
pub mod mix;
pub mod mpmodes;
pub mod rules;
pub mod voxel;

pub use audio::{AudioBagEntry, AudioIndex, PcmAudio, WavError, decode_audio_bytes, decode_wav_pcm};
pub use battle::{BattleCampaign, find_battle_campaign, parse_battle_campaigns};

pub use image::{
    bink::{
        BINK_FLAG_ALPHA, BINK_FLAG_GRAY, BinkAudioTrack, BinkColorRange, BinkFile, BinkFrameIndexEntry, BinkFramePacket, BinkHeader,
        BinkVersion, parse_bink_file, parse_bink_header,
    },
    bink_bits::{BitReader, VlcTable, build_fixed_vlc_tables},
    bink_bundle::{
        BinkBundle, BinkSrc, NB_SRC, alloc_bundles, init_bundle_lengths, read_block_types, read_bundle, read_colors, read_dcs,
        read_motion_values, read_patterns, read_runs, take_value, take_value16,
    },
    bink_dct::{decode_inter_dct_block, decode_intra_dct_block, read_dct_coeffs},
    bink_huff::HuffmanTree,
    bink_idct::{BINK_SCAN, bink_idct, idct_add, idct_put},
    bink_patterns::BINK_RUN_PATTERNS,
    bink_quant::{BINK_INTER_QUANT, BINK_INTRA_QUANT},
    bink_residue::read_residue,
    bink_tables::{BINK_RLELENS, BINK_TREE_BITS, BINK_TREE_LENS, DC_START_BITS},
    bink_video::{BinkVideoDecoder, BinkVideoError, BinkYuvFrame, yuv420_planes_to_rgba8},
    csf::{CsfFile, LABEL_MAGIC, STRING_MAGIC},
    fnt::{FONT_MAGIC, FntFile, FntGlyph},
    pal::{Palette, Rgba, default_vga_expand, set_default_vga_expand},
    pcx::{PcxImage, parse_pcx},
    shp::{ShpFile, ShpFrame, decode_rle_frame, shp_body_frame_count, shp_shadow_half_base, shp_shadow_half_populated},
    tmp::{TILE_HEADER_SIZE, TmpFile, TmpTile, diamond_byte_count},
};
pub use ini::{
    IniDeError, IniDocument, IniEntry, IniSection, IniValue, SourceId, SourceSpan, collect_shp_refs, concat_numbered_values, from_section,
    numbered_section_concat,
};
pub use mix::{
    archive::{MixArchive, MixEntry},
    crypto::blowfish_decrypt_ecb,
    hash::{crc32, mix_hash, westwood_pad},
    names::MixNameTable,
    vfs::{MixRawEntry, MixResolveHit, MixVfs},
};
pub use mpmodes::{MpMode, parse_mpmodes};
pub use ra_types::OverlayTypeRegistry;
pub use rules::{
    color_schemes::ColorSchemes,
    countries::{
        CountryDef, CountryRegistry, SideChromeDef, SideGroup, fill_country_ui_gaps, fill_side_chrome_gaps, resolve_country_special_ui_name,
    },
    globals::RulesGlobals,
    house_remap::{HOUSE_REMAP_COUNT, HOUSE_REMAP_FIRST, Hsv, build_hsv_remap_ramp, build_remap_ramp, hsv_to_rgb, owner_primary_color},
    overlay::{harvestable_overlay_name, overlay_types_from_rules, tiberium_overlay_display_hsv, tiberium_type_for_overlay},
    super_weapons::{SuperWeaponType, SuperWeaponTypeRegistry},
    techno::{TechnoKind, TechnoType, TechnoTypeRegistry},
    terrain_spawners::terrain_spawners_from_rules,
    warheads::{ARMOR_ORDER, Warhead, WarheadRegistry, armor_index},
};
pub use voxel::{
    hva::HvaFile,
    raster::{
        VXL_SHADOW_LIGHT_OFFSET_X, VxlLayerPose, VxlSprite, rasterize_vxl, rasterize_vxl_frame, rasterize_vxl_layer_poses,
        rasterize_vxl_layers, rasterize_vxl_posed, rasterize_vxl_shadow_layer_poses,
    },
    vpl::VplFile,
    vxl::{VxlFile, VxlLimb, VxlVoxel},
};
