//! Embedded development icon; app bundles use their native Info.plist icon.
//! The same artwork identifies the build in the About box on every platform.
// Personal fork: distinct artwork for side-by-side development launches.
pub(super) const PNG: &[u8] = include_bytes!("../../../assets/personal/icon.png");

#[cfg(target_os = "macos")]
const ICNS: &[u8] = include_bytes!("../../../assets/personal/HerdrPersonal.icns");

#[cfg(target_os = "macos")]
pub fn install() {
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::{NSBundle, NSData, NSProcessInfo, NSString};

    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    NSProcessInfo::processInfo()
        .setProcessName(&NSString::from_str(crate::constants::WINDOW_TITLE));
    if NSBundle::mainBundle()
        .objectForInfoDictionaryKey(&NSString::from_str("CFBundleIconFile"))
        .is_some()
    {
        return;
    }
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(ICNS)) else {
        tracing::error!("Unable to decode the embedded Herdr icon");
        return;
    };
    // SAFETY: GPUI has initialized AppKit, the main-thread marker gates access,
    // and the setter receives a live, non-null NSImage (never None).
    #[allow(unsafe_code)]
    unsafe {
        NSApplication::sharedApplication(main_thread).setApplicationIconImage(Some(&image));
    }
}

#[cfg(not(target_os = "macos"))]
pub fn install() {}

#[cfg(all(target_os = "macos", feature = "integration-test"))]
pub fn verify_native() -> anyhow::Result<()> {
    use anyhow::Context as _;
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    let mtm = MainThreadMarker::new().context("icon check must run on the main thread")?;
    let image = NSApplication::sharedApplication(mtm)
        .applicationIconImage()
        .context("NSApplication has no icon")?;
    let size = image.size();
    if !image.isValid() || size.width != 512. || size.height != 512. {
        anyhow::bail!("invalid native icon: {size:?}");
    }
    eprintln!("ICON native PASS: valid NSApplication image, 512x512 points");
    Ok(())
}

#[cfg(test)]
mod tests;
