//! adaptor 壳层 profile 与 `ra-layout` 模板解析对齐。

use ra_adaptor::{dialog_template_0x102, dialog_template_0x6b};
use ra_layout::{
    dialog_template_0x102 as layout_dialog_0x102, dialog_template_0x6b as layout_dialog_0x6b,
    resolve_dialog_template, RightPanelChrome,
};

#[test]
fn adaptor_and_layout_templates_resolve_identically_for_0x6b() {
    let chrome = RightPanelChrome::shell_defaults();
    let from_adaptor = resolve_dialog_template(&dialog_template_0x6b(), chrome);
    let from_layout = resolve_dialog_template(&layout_dialog_0x6b(), chrome);
    assert_eq!(from_adaptor, from_layout);
}

#[test]
fn adaptor_and_layout_templates_resolve_identically_for_0x102() {
    let chrome = RightPanelChrome::shell_defaults();
    let from_adaptor = resolve_dialog_template(&dialog_template_0x102(), chrome);
    let from_layout = resolve_dialog_template(&layout_dialog_0x102(), chrome);
    assert_eq!(from_adaptor, from_layout);
}
