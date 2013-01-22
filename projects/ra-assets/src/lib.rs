//! 格式解析：字节进、结构出。不碰 `std::fs`。

mod ini;
mod mix;

pub use ini::{IniDocument, IniSection};
pub use mix::{MixArchive, MixEntry};
