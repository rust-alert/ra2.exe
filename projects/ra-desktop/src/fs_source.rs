//! 磁盘松散文件 + 已挂载 MIX 的组合 `AssetSource`。
//!
//! 松散目录与 MIX 走**同一套**优先级比较：松散层为
//! [`ra_adaptor::PRIORITY_USER_OVERRIDE`]，与 `MixVfs` 胜出结果比较后再读字节，
//! 避免「日志说来自 MIX、实际读了磁盘」的分裂。

use std::path::PathBuf;

use ra_adaptor::{
    MountSpec, NestedMountSpec, NestedMountStrategy, PRIORITY_USER_OVERRIDE, find_ci_file,
};
use ra_assets::{MixResolveHit, MixVfs};
use ra_types::{AssetSource, RaError, RaResult};

/// 统一解析胜出来源。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetOrigin {
    /// 安装根旁松散文件。
    Loose {
        /// 实际磁盘路径（显示用）。
        path: PathBuf,
    },
    /// 已挂载 MIX（含嵌套展开）。
    Mix {
        /// 档案挂载名。
        archive: String,
        /// 父档案（根为 `None`）。
        parent: Option<String>,
        /// 内容层 id。
        layer_id: Option<String>,
        /// 胜出优先级。
        priority: i32,
    },
}

/// 一次逻辑名解析结果（读与诊断共用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetHit {
    /// 来源说明。
    pub origin: AssetOrigin,
    /// 完整字节。
    pub bytes: Vec<u8>,
}

impl AssetHit {
    /// 简短诊断串。
    pub fn explain(&self) -> String {
        match &self.origin {
            AssetOrigin::Loose { path } => format!("loose:{}", path.display()),
            AssetOrigin::Mix {
                archive,
                parent,
                layer_id,
                priority,
            } => match (parent, layer_id) {
                (Some(p), Some(l)) => format!("mix:{archive} parent={p} layer={l} prio={priority}"),
                (Some(p), None) => format!("mix:{archive} parent={p} prio={priority}"),
                (None, Some(l)) => format!("mix:{archive} layer={l} prio={priority}"),
                (None, None) => format!("mix:{archive} prio={priority}"),
            },
        }
    }
}

pub struct GameAssetSource {
    pub root: PathBuf,
    pub vfs: MixVfs,
}

impl GameAssetSource {
    pub fn new(root: PathBuf) -> Self {
        Self { root, vfs: MixVfs::new() }
    }

    /// 按挂载计划挂载根 MIX（显式 priority / layer）。返回 `(成功数, 解析跳过数)`。
    pub fn mount_root_plan(&mut self, plan: &[MountSpec]) -> (usize, usize) {
        let mut mounted = 0usize;
        let mut skipped = 0usize;
        for spec in plan {
            let Some(path) = find_ci_file(&self.root, &spec.name)
            else {
                skipped += 1;
                continue;
            };
            let Ok(data) = std::fs::read(&path)
            else {
                skipped += 1;
                continue;
            };
            match self.vfs.mount_bytes_with_meta(
                spec.name.clone(),
                data,
                spec.priority,
                None,
                Some(spec.layer_id.clone()),
            ) {
                Ok(()) => mounted += 1,
                Err(_) => skipped += 1,
            }
        }
        (mounted, skipped)
    }

    /// 兼容旧路径：无 priority 时按列表顺序挂载（同优先级后挂覆盖）。
    #[allow(dead_code)]
    pub fn mount_present_roots(&mut self, present_mixes: &[String]) -> (usize, usize) {
        let plan: Vec<MountSpec> = present_mixes
            .iter()
            .map(|name| MountSpec {
                name: name.clone(),
                priority: 0,
                layer_id: "legacy".to_string(),
            })
            .collect();
        self.mount_root_plan(&plan)
    }

    /// 按组合里的嵌套挂载计划展开子包（当前仅 `AllParents`）。
    pub fn mount_nested_plan(&mut self, plan: &[NestedMountSpec]) -> usize {
        let mut mounted = 0usize;
        for spec in plan {
            match spec.strategy {
                NestedMountStrategy::AllParents => {
                    match self.vfs.mount_nested_all_from_parents(&spec.name) {
                        Ok(n) => mounted += n,
                        Err(_) => {}
                    }
                }
            }
        }
        mounted
    }

    /// 按名单从**所有**已挂载父档展开同名嵌套包，继承各父档内容层优先级。
    ///
    /// 新路径请优先 [`Self::mount_nested_plan`]。
    pub fn mount_nested_names(&mut self, names: &[&str]) -> usize {
        let plan: Vec<NestedMountSpec> = names
            .iter()
            .map(|name| NestedMountSpec {
                name: (*name).to_string(),
                strategy: NestedMountStrategy::AllParents,
            })
            .collect();
        self.mount_nested_plan(&plan)
    }

    /// 统一解析：松散层与 MIX 比较优先级后得出唯一胜出。
    pub fn resolve(&self, relative: &str) -> Option<AssetHit> {
        let loose = find_ci_file(&self.root, relative).and_then(|path| {
            let bytes = std::fs::read(&path).ok()?;
            Some((
                PRIORITY_USER_OVERRIDE,
                AssetHit {
                    origin: AssetOrigin::Loose { path },
                    bytes,
                },
            ))
        });

        let mix = self.vfs.resolve_hit(relative).map(|h: MixResolveHit<'_>| {
            (
                h.priority,
                AssetHit {
                    origin: AssetOrigin::Mix {
                        archive: h.archive_name.to_string(),
                        parent: h.parent.map(str::to_string),
                        layer_id: h.layer_id.map(str::to_string),
                        priority: h.priority,
                    },
                    bytes: h.bytes.to_vec(),
                },
            )
        });

        match (loose, mix) {
            (Some((lp, lh)), Some((mp, mh))) => {
                if lp >= mp {
                    Some(lh)
                } else {
                    Some(mh)
                }
            }
            (Some((_, h)), None) | (None, Some((_, h))) => Some(h),
            (None, None) => None,
        }
    }
}

impl AssetSource for GameAssetSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        self.resolve(relative)
            .map(|h| h.bytes)
            .ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_assets::mix_hash;
    use ra_types::AssetSource;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn old_mix(id: i32, body: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(&id.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(body);
        data
    }

    fn scratch(tag: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("ra-desktop-fs-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn loose_overrides_mix_and_explain_matches_read() {
        let dir = scratch("loose");
        let mix_bytes = old_mix(mix_hash("rules.ini"), b"FROM-MIX");
        std::fs::write(dir.join("base.mix"), &mix_bytes).unwrap();
        std::fs::write(dir.join("rules.ini"), b"FROM-LOOSE").unwrap();

        let mut src = GameAssetSource::new(dir.clone());
        src.vfs
            .mount_bytes_with_meta("base.mix", mix_bytes, 0, None, Some("base".into()))
            .unwrap();

        let hit = src.resolve("rules.ini").unwrap();
        assert!(matches!(hit.origin, AssetOrigin::Loose { .. }));
        assert_eq!(hit.bytes, b"FROM-LOOSE");
        assert_eq!(src.read("rules.ini").unwrap(), hit.bytes);
        assert!(hit.explain().starts_with("loose:"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nested_expand_leaf_overlays_base_leaf() {
        let nested_base = old_mix(mix_hash("leaf.bin"), b"BASE-LEAF");
        let nested_exp = old_mix(mix_hash("leaf.bin"), b"EXP-LEAF");
        let outer_base = old_mix(mix_hash("cache.mix"), &nested_base);
        let outer_exp = old_mix(mix_hash("cache.mix"), &nested_exp);

        let dir = scratch("nested");
        let mut src = GameAssetSource::new(dir.clone());
        src.vfs
            .mount_bytes_with_meta("base.mix", outer_base, 0, None, Some("base".into()))
            .unwrap();
        src.vfs
            .mount_bytes_with_meta(
                "expand01.mix",
                outer_exp,
                101,
                None,
                Some("expansion.plain.01".into()),
            )
            .unwrap();
        assert_eq!(src.mount_nested_names(&["cache.mix"]), 2);

        let hit = src.resolve("leaf.bin").unwrap();
        assert_eq!(hit.bytes, b"EXP-LEAF");
        match hit.origin {
            AssetOrigin::Mix {
                parent: Some(p),
                priority,
                ..
            } => {
                assert_eq!(p, "expand01.mix");
                assert_eq!(priority, 101);
            }
            other => panic!("expected mix hit, got {other:?}"),
        }
        assert_eq!(src.read("leaf.bin").unwrap(), b"EXP-LEAF");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
