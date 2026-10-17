use super::*;
use crate::config::fonts::LINUX_FONTS;
use crate::design::TypeScale;
use std::collections::BTreeSet;

fn installed(families: &[&str]) -> BTreeSet<String> {
    families.iter().map(|family| (*family).to_owned()).collect()
}

#[test]
fn linux_ui_font_prefers_adwaita_then_inter_then_dejavu() {
    assert_eq!(LINUX_FONTS.sans, "Adwaita Sans");
    let substitute = |families: &[&str]| LINUX_FONTS.substitute("Adwaita Sans", &installed(families));
    assert_eq!(substitute(&["Adwaita Sans", "Inter"]), None);
    assert_eq!(substitute(&["Inter", "DejaVu Sans"]).as_deref(), Some("Inter"));
    assert_eq!(substitute(&["DejaVu Sans", "Noto Sans"]).as_deref(), Some("DejaVu Sans"));
    // The families that came before keep their turn behind the new ones.
    assert_eq!(substitute(&["Noto Sans"]).as_deref(), Some("Noto Sans"));
}

#[test]
fn main_window_text_defaults_to_13px() -> anyhow::Result<()> {
    // Dialogs keep reading `[ui]` at its own size; the redesigned main window
    // names things one point larger, so the default gives 13, 12 and 11.5.
    for config in [Config::default(), Config::parse(DEFAULT_CONFIG)?] {
        assert_eq!(config.ui.size, 12.);
        let scale = TypeScale::of(&config.ui);
        assert_eq!((scale.name, scale.secondary, scale.detail), (13., 12., 11.5));
    }
    // A configured size moves the whole scale with it.
    let scale = TypeScale::of(&Config::parse("[ui]\nsize = 15")?.ui);
    assert_eq!((scale.name, scale.secondary, scale.detail), (16., 15., 14.5));
    Ok(())
}
