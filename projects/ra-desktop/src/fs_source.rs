//! 磁盘松散文件 + 已挂载 MIX 的组合 `AssetSource`。

use std::path::PathBuf;

use ra_adaptor::{MountSpec, PRIORITY_NESTED, find_ci_file};
use ra_assets::MixVfs;
use ra_types::{AssetSource, RaError, RaResult};

pub struct GameAssetSource {
    pub root: PathBuf,
    pub vfs: MixVfs,
}

impl GameAssetSource {
    pub fn new(root: PathBuf) -> Self {
        Self { root, vfs: MixVfs::new() }
    }

    /// 按挂载计划挂载根 MIX（显式 priority）。返回 `(成功数, 解析跳过数)`。
    pub fn mount_root_plan(&mut self, plan: &[MountSpec]) -> (usize, usize) {
        let mut mounted = 0usize;
        let mut skipped = 0usize;
        for spec in plan {
            let Some(path) = find_ci_file(&self.root, &spec.name)
            else {
                continue;
            };
            let Ok(data) = std::fs::read(&path)
            else {
                skipped += 1;
                continue;
            };
            match self.vfs.mount_bytes_with_priority(spec.name.clone(), data, spec.priority) {
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

    /// 尝试挂载嵌套 MIX 名列表（优先级 [`PRIORITY_NESTED`]）。返回成功挂载数。
    pub fn mount_nested_names(&mut self, names: &[&str]) -> usize {
        let mut mounted = 0usize;
        for name in names {
            match self.vfs.mount_nested_with_priority(name, PRIORITY_NESTED) {
                Ok(true) => mounted += 1,
                Ok(false) | Err(_) => {}
            }
        }
        mounted
    }
}

impl AssetSource for GameAssetSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        if let Some(path) = find_ci_file(&self.root, relative) {
            return std::fs::read(&path).map_err(|e| RaError::Io(format!("{}: {e}", path.display())));
        }
        self.vfs.read(relative).ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}
