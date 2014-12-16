//! 配置来源、合并与诊断。
//!
//! 桌面规范文件为可执行文件同目录下的 `RustAlert.toml`，由 `toml_edit` 读写以保留注释。
//! 本 crate 不解释游戏语义；adaptor 决定读哪些资源及含义。

#![deny(missing_docs)]

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use ra_types::{DisplayMode, PresentFeel};
use serde::Deserialize;
use toml_edit::{DocumentMut, Item, Table, Value};

/// CLI / N-API 一次性启动覆盖（后于 `RustAlert.toml` 生效）。
#[derive(Debug, Clone)]
pub struct LaunchOverride {
    /// 游戏安装目录。
    pub ra2_dir: PathBuf,
    /// 可选版本字符串。
    pub edition: Option<String>,
}

static LAUNCH_OVERRIDE: Mutex<Option<LaunchOverride>> = Mutex::new(None);

/// 设置启动覆盖（`ra2 launch --path`）。
pub fn set_launch_override(override_: LaunchOverride) {
    *LAUNCH_OVERRIDE.lock().expect("launch override lock") = Some(override_);
}

/// 清除启动覆盖。
pub fn clear_launch_override() {
    *LAUNCH_OVERRIDE.lock().expect("launch override lock") = None;
}

fn take_launch_override_snapshot() -> Option<LaunchOverride> {
    LAUNCH_OVERRIDE.lock().expect("launch override lock").clone()
}

/// 规范桌面配置文件名（位于可执行文件同目录）。
pub const RUST_ALERT_TOML: &str = "RustAlert.toml";

/// 一条配置诊断。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    /// 来源标签（如文件路径或 `defaults`）。
    pub source: String,
    /// 人类可读说明。
    pub message: String,
}

/// 扁平字符串配置表（桌面设置等够用；INI 游戏语义不在此解释）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigTable {
    values: BTreeMap<String, String>,
}

impl ConfigTable {
    /// 空表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 按键取值。
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    /// 插入或覆盖。
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    /// 遍历键值（按键排序）。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// 带来源标签的一层配置。
#[derive(Debug, Clone)]
pub struct ConfigLayer {
    /// 层标签。
    pub label: String,
    /// 该层键值。
    pub table: ConfigTable,
}

/// 合并结果：后者覆盖前者同名键。
#[derive(Debug, Clone, Default)]
pub struct MergedConfig {
    /// 合并后的表。
    pub table: ConfigTable,
    /// 合并过程中的诊断。
    pub diagnostics: Vec<ConfigDiagnostic>,
}

impl MergedConfig {
    /// 按顺序合并多层；后层覆盖前层。
    pub fn merge_layers(layers: &[ConfigLayer]) -> Self {
        let mut out = Self::default();
        for layer in layers {
            for (k, v) in layer.table.iter() {
                out.table.insert(k, v);
            }
        }
        out
    }

    /// 读取合并后的键。
    pub fn get(&self, key: &str) -> Option<&str> {
        self.table.get(key)
    }
}

/// 当前可执行文件所在目录；失败时回退为 `"."`。
pub fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

/// `RustAlert.toml` 的规范路径（可执行文件同目录）。
pub fn rust_alert_toml_path() -> PathBuf {
    exe_dir().join(RUST_ALERT_TOML)
}

fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.value().clone()),
        Value::Integer(i) => Some(i.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

/// 用 `toml_edit` 解析文档根级键值为扁平表。
pub fn parse_toml_document(text: &str, source_label: &str) -> (ConfigTable, Vec<ConfigDiagnostic>) {
    let mut table = ConfigTable::new();
    let mut diagnostics = Vec::new();
    let doc: DocumentMut = match text.parse() {
        Ok(d) => d,
        Err(e) => {
            diagnostics.push(ConfigDiagnostic { source: source_label.into(), message: format!("TOML 解析失败: {e}") });
            return (table, diagnostics);
        }
    };
    for (key, item) in doc.iter() {
        match item {
            Item::Value(v) => match value_as_string(v) {
                Some(s) => table.insert(key, s),
                None => diagnostics
                    .push(ConfigDiagnostic {
                        source: format!("{source_label}:{key}"), message: format!("不支持的值类型，已跳过键 `{key}`")
                    }),
            },
            Item::None => {}
            // 结构化段（如 `[present]`）由 `DocumentMut` + serde 读取，不进扁平表。
            Item::Table(_) | Item::ArrayOfTables(_) => {}
        }
    }
    (table, diagnostics)
}

/// 仅反序列化文档中的 `[present]`（其余根键忽略）。
#[derive(Debug, Default, Deserialize)]
struct PresentSectionFile {
    #[serde(default)]
    present: PresentFeel,
}

/// 从完整 TOML 文本读取 `[present]`；缺失则默认，失败则诊断并回退默认。
pub fn present_feel_from_toml_text(text: &str, source_label: &str) -> (PresentFeel, Vec<ConfigDiagnostic>) {
    match toml_edit::de::from_str::<PresentSectionFile>(text) {
        Ok(file) => (file.present.sanitized(), Vec::new()),
        Err(e) => (
            PresentFeel::DEFAULT,
            vec![ConfigDiagnostic { source: source_label.into(), message: format!("[present] 解析失败，已用默认质感: {e}") }],
        ),
    }
}

/// 首次落盘用的默认 `RustAlert.toml` 文本（含注释，写入当前 `ra2_dir`）。
pub fn default_rust_alert_toml_text(ra2_dir: &Path) -> String {
    let dir = ra2_dir.display().to_string().replace('\\', "/");
    let feel = PresentFeel::DEFAULT;
    format!(
        "# RustAlert 桌面启动配置\n\
         # 首次启动时由程序自动生成，可按需修改后持久化。\n\
         # 分辨率、显示与其它启动选项写在本文件中。\n\
         #\n\
         # 未改 `ra2_dir` 时默认即进程相关目录；产品路径请用 `ra2 launch --path`。\n\
         \n\
         ra2_dir = \"{dir}\"\n\
         # display_mode = \"1024x768\"   # 640x480 / 800x600 / 1024x768\n\
         # music_volume = 0.4           # 壳层 BGM，0..1\n\
         # sound_volume = 0.7           # 壳层点击等短音效，0..1\n\
         # load_min_secs = 3.0          # 遭遇战装载页最短展示秒数（0 关闭）\n\
         # edition = \"ra2\"   # 或 \"yr\"；省略则按目录特征自动探测\n\
         # net_url = \"\"      # 预留战网地址\n\
         # net_room = \"\"     # 预留房间名\n\
         \n\
         # 壳层质感呈现（模拟原版 16 位色观感；由 toml_edit + serde 读写本表）\n\
         [present]\n\
         mode = \"{mode}\"                 # off | 16bit\n\
         quantize = \"{quant}\"            # rgb565 | rgb555\n\
         dither = {dither}                 # 量化前有序抖动\n",
        mode = feel.mode.as_str(),
        quant = feel.quantize.as_str(),
        dither = feel.dither,
    )
}

/// 若 `path` 不存在则写入默认模板；返回是否新创建。
pub fn ensure_rust_alert_toml(path: &Path) -> Result<bool, String> {
    if path.is_file() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    let ra2_dir = path.parent().map(Path::to_path_buf).unwrap_or_else(exe_dir);
    let text = default_rust_alert_toml_text(&ra2_dir);
    std::fs::write(path, text).map_err(|e| format!("写入 {} 失败: {e}", path.display()))?;
    Ok(true)
}

/// 可编辑的 `RustAlert.toml`（保留注释与格式）。
#[derive(Debug, Clone)]
pub struct RustAlertDocument {
    path: PathBuf,
    doc: DocumentMut,
}

impl RustAlertDocument {
    /// 打开已有文件。
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let text = std::fs::read_to_string(&path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
        let doc: DocumentMut = text.parse().map_err(|e| format!("解析 {} 失败: {e}", path.display()))?;
        Ok(Self { path, doc })
    }

    /// 打开规范路径；缺失则自动生成默认文件后打开。
    pub fn open_or_create() -> Result<Self, String> {
        let path = rust_alert_toml_path();
        ensure_rust_alert_toml(&path)?;
        Self::open(path)
    }

    /// 打开规范路径；不存在则空文档（尚未落盘）。
    pub fn open_or_empty() -> Result<Self, String> {
        let path = rust_alert_toml_path();
        if path.is_file() { Self::open(path) } else { Ok(Self { path, doc: DocumentMut::new() }) }
    }

    /// 文件路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 读取根级字符串键。
    pub fn get_str(&self, key: &str) -> Option<String> {
        self.doc.get(key).and_then(Item::as_value).and_then(value_as_string)
    }

    /// 设置根级字符串键（覆盖或插入）。
    pub fn set_str(&mut self, key: &str, value: impl AsRef<str>) {
        self.doc[key] = Item::Value(value.as_ref().into());
    }

    /// 设置根级浮点键（覆盖或插入）。
    pub fn set_f64(&mut self, key: &str, value: f64) {
        self.doc[key] = Item::Value(value.into());
    }

    /// 读取 `[present]` 质感表（serde）；缺失则默认。
    pub fn present_feel(&self) -> (PresentFeel, Vec<ConfigDiagnostic>) {
        present_feel_from_toml_text(&self.doc.to_string(), &self.path.display().to_string())
    }

    /// 写入 `[present]` 表（serde → `toml_edit` Item，保留其它根键与注释）。
    pub fn set_present_feel(&mut self, feel: &PresentFeel) -> Result<(), String> {
        let feel = feel.sanitized();
        let generated = toml_edit::ser::to_document(&feel).map_err(|e| format!("序列化 [present] 失败: {e}"))?;
        let mut table = Table::new();
        for (key, item) in generated.as_table().iter() {
            table.insert(key, item.clone());
        }
        // 保留已有表装饰（若有）。
        if let Some(existing) = self.doc.get("present").and_then(Item::as_table) {
            *table.decor_mut() = existing.decor().clone();
        }
        self.doc["present"] = Item::Table(table);
        Ok(())
    }

    /// 移除根级键。
    pub fn remove(&mut self, key: &str) {
        let _ = self.doc.remove(key);
    }

    /// 写回磁盘（创建父目录若需要）。
    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
        }
        std::fs::write(&self.path, self.doc.to_string()).map_err(|e| format!("写入 {} 失败: {e}", self.path.display()))
    }

    /// 导出为扁平表。
    pub fn to_table(&self) -> (ConfigTable, Vec<ConfigDiagnostic>) {
        parse_toml_document(&self.doc.to_string(), &self.path.display().to_string())
    }
}

/// 解析 0..1 音量；非法或非有限值返回 `None`（调用方保留默认）。
pub fn parse_unit_volume(raw: &str) -> Option<f32> {
    let v: f32 = raw.trim().parse().ok()?;
    if v.is_finite() { Some(v.clamp(0.0, 1.0)) } else { None }
}

/// 桌面启动设置（由合并后的键值填充）。
#[derive(Debug, Clone)]
pub struct DesktopSettings {
    /// 游戏安装目录（默认：可执行文件所在目录）。
    pub ra2_dir: PathBuf,
    /// 正式版本字符串（可选）。
    pub edition: Option<String>,
    /// 客户区显示分辨率档（离散，非自由宽高）。
    pub display_mode: DisplayMode,
    /// 壳层 BGM 音量（0..1）。
    pub music_volume: f32,
    /// 壳层短音效音量（0..1）。
    pub sound_volume: f32,
    /// 壳层质感呈现（`[present]` 表）。
    pub present: PresentFeel,
    /// 遭遇战装载页最短展示秒数（后台已完成也等到点再切页；`0` 关闭）。
    pub load_min_secs: f64,
    /// 预留目标战网连接地址（协议未落地前可空置，不建 socket）。
    pub net_url: Option<String>,
    /// 预留房间名。
    pub net_room: Option<String>,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            ra2_dir: exe_dir(),
            edition: None,
            display_mode: DisplayMode::DEFAULT,
            music_volume: 0.4,
            sound_volume: 0.7,
            present: PresentFeel::DEFAULT,
            load_min_secs: 3.0,
            net_url: None,
            net_room: None,
        }
    }
}

impl DesktopSettings {
    /// 从合并配置填充字段；缺省 `ra2_dir` 时用可执行文件目录。
    pub fn from_merged(merged: &MergedConfig) -> Self {
        let mut s = Self::default();
        if let Some(v) = merged.get("ra2_dir").or_else(|| merged.get("game_dir")) {
            s.ra2_dir = PathBuf::from(v);
        }
        if let Some(v) = merged.get("edition").filter(|v| !v.is_empty()) {
            s.edition = Some(v.to_string());
        }
        if let Some(v) = merged.get("display_mode").or_else(|| merged.get("resolution")).filter(|v| !v.is_empty()) {
            match DisplayMode::parse(v) {
                Ok(mode) => s.display_mode = mode,
                Err(_) => {
                    // 非法档位忽略，保留默认；诊断由上层合并日志可选扩展。
                }
            }
        }
        if let Some(v) = merged.get("music_volume").or_else(|| merged.get("score_volume")).and_then(parse_unit_volume) {
            s.music_volume = v;
        }
        if let Some(v) = merged.get("sound_volume").or_else(|| merged.get("sfx_volume")).and_then(parse_unit_volume) {
            s.sound_volume = v;
        }
        if let Some(v) = merged.get("load_min_secs").and_then(|raw| raw.trim().parse::<f64>().ok()).filter(|v| v.is_finite()) {
            s.load_min_secs = v.max(0.0);
        }
        if let Some(v) = merged.get("net_url").or_else(|| merged.get("battlenet_url")).filter(|v| !v.is_empty()) {
            s.net_url = Some(v.to_string());
        }
        if let Some(v) = merged.get("net_room").or_else(|| merged.get("room")).filter(|v| !v.is_empty()) {
            s.net_room = Some(v.to_string());
        }
        s
    }

    /// 加载桌面配置：默认（exe 目录）← `RustAlert.toml` 覆盖。
    /// 若规范路径缺失，则自动生成默认文件以便持久化。
    pub fn load_or_default() -> (Self, Vec<ConfigDiagnostic>) {
        let exe = exe_dir();
        let defaults = ConfigLayer {
            label: "defaults".into(),
            table: {
                let mut t = ConfigTable::new();
                t.insert("ra2_dir", exe.to_string_lossy());
                t.insert("display_mode", DisplayMode::DEFAULT.as_str());
                t.insert("music_volume", "0.4");
                t.insert("sound_volume", "0.7");
                t.insert("load_min_secs", "3.0");
                t
            },
        };
        let mut layers = vec![defaults];
        let mut diagnostics = Vec::new();
        let path = rust_alert_toml_path();
        match ensure_rust_alert_toml(&path) {
            Ok(true) => diagnostics
                .push(ConfigDiagnostic { source: path.display().to_string(), message: "已自动生成默认配置以便持久化".into() }),
            Ok(false) => {}
            Err(e) => {
                diagnostics.push(ConfigDiagnostic { source: path.display().to_string(), message: format!("自动生成配置失败: {e}") })
            }
        }
        if path.is_file() {
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    let label = path.display().to_string();
                    let (table, mut diags) = parse_toml_document(&text, &label);
                    diagnostics.append(&mut diags);
                    layers.push(ConfigLayer { label: label.clone(), table });
                    let (present, mut present_diags) = present_feel_from_toml_text(&text, &label);
                    diagnostics.append(&mut present_diags);
                    let mut merged = MergedConfig::merge_layers(&layers);
                    merged.diagnostics.append(&mut diagnostics);
                    let mut settings = Self::from_merged(&merged);
                    settings.present = present;
                    if let Some(over) = take_launch_override_snapshot() {
                        settings.ra2_dir = over.ra2_dir;
                        if over.edition.is_some() {
                            settings.edition = over.edition;
                        }
                        merged.diagnostics.push(ConfigDiagnostic {
                            source: "launch-override".into(),
                            message: format!("CLI/N-API 覆盖 ra2_dir={}", settings.ra2_dir.display()),
                        });
                    }
                    return (settings, merged.diagnostics);
                }
                Err(e) => diagnostics.push(ConfigDiagnostic { source: path.display().to_string(), message: format!("读取失败: {e}") }),
            }
        }
        let mut merged = MergedConfig::merge_layers(&layers);
        merged.diagnostics.append(&mut diagnostics);
        let mut settings = Self::from_merged(&merged);
        if let Some(over) = take_launch_override_snapshot() {
            settings.ra2_dir = over.ra2_dir;
            if over.edition.is_some() {
                settings.edition = over.edition;
            }
            merged.diagnostics.push(ConfigDiagnostic {
                source: "launch-override".into(),
                message: format!("CLI/N-API 覆盖 ra2_dir={}", settings.ra2_dir.display()),
            });
        }
        (settings, merged.diagnostics)
    }

    /// 将 `display_mode` 写回规范路径上的 `RustAlert.toml`（保留其它键）。
    pub fn persist_display_mode(mode: DisplayMode) -> Result<(), String> {
        let mut doc = RustAlertDocument::open_or_create()?;
        doc.set_str("display_mode", mode.as_str());
        doc.save()
    }

    /// 将壳层音量写回规范路径上的 `RustAlert.toml`（保留其它键）。
    pub fn persist_audio_volumes(music_volume: f32, sound_volume: f32) -> Result<(), String> {
        let music = music_volume.clamp(0.0, 1.0) as f64;
        let sound = sound_volume.clamp(0.0, 1.0) as f64;
        let mut doc = RustAlertDocument::open_or_create()?;
        doc.set_f64("music_volume", music);
        doc.set_f64("sound_volume", sound);
        doc.save()
    }

    /// 将 `[present]` 质感表写回规范路径上的 `RustAlert.toml`（保留其它键与注释）。
    pub fn persist_present_feel(feel: PresentFeel) -> Result<(), String> {
        let mut doc = RustAlertDocument::open_or_create()?;
        doc.set_present_feel(&feel)?;
        doc.save()
    }
}
