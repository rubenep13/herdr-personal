//! Values fixed at build time or by the reference design: the product name the
//! OS sees, the tab strip's geometry, and the release identity the updater and
//! any credential store key off.

// Every window carries the product name; the focused space follows it.
// Personal fork: separate development identity; upstream attribution is retained.
pub(crate) const WINDOW_TITLE: &str = "Herdr Personal Dev";

// The macOS bundle identifier, also the Linux app ID and Windows AppUserModelID.
pub(crate) const APP_ID: &str = "dev.ruben.herdr-personal";
pub(crate) const APP_NAMESPACE: &str = "herdr-personal-dev";
pub(crate) const UPDATES_ENABLED: bool = false;

// Even tab cells, as on herdr.dev, so short labels do not collapse to a sliver.
pub(crate) const TAB_WIDTH: f32 = 64.;
// The reference strip is a shallow band: chrome, not a toolbar.
pub(crate) const TAB_HEIGHT: f32 = 24.;

// Release builds embed the same calendar version (YYYYMMDD.COUNTER) used for the
// tag, the bundle, and the downloadable artifacts. Local builds are not releases,
// so they carry no version the updater or an issue report could act on.
pub(crate) const APP_VERSION: &str = match option_env!("HERDR_RELEASE_VERSION") {
    Some(version) => version,
    None => "dev",
};

// Personal development builds never inherit upstream release privileges, even
// when a caller sets HERDR_RELEASE_VERSION. Signing/distribution is separate work.
pub(crate) const RELEASE_BUILD: bool = false;
