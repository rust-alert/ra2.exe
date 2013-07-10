use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use ra_logger::{info, init, warn};

#[test]
fn writes_append_file() {
    let dir = std::env::temp_dir()
        .join(format!("ra-logger-test-{}", SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)));
    let _ = fs::remove_dir_all(&dir);
    let path = init(&dir, false).unwrap();
    info("hello");
    warn("careful");
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("INFO hello"));
    assert!(text.contains("WARN careful"));
    let _ = fs::remove_dir_all(&dir);
}
