use super::PNG;

#[cfg(target_os = "macos")]
#[test]
#[allow(clippy::expect_used)]
fn dock_icons_decode_with_all_standard_and_retina_representations() {
    use objc2::AnyThread;
    use objc2_app_kit::NSImage;
    use objc2_foundation::NSData;

    let stable = include_bytes!("../../../../assets/icons/Herdr.icns").as_slice();
    let worktree = include_bytes!("../../../../assets/icons/Herdr-worktree.icns").as_slice();
    let personal = include_bytes!("../../../../assets/personal/HerdrPersonal.icns").as_slice();
    assert_ne!(stable, worktree);
    assert_eq!(super::ICNS, personal);
    for bytes in [stable, worktree, personal] {
        let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(bytes))
            .expect("embedded ICNS must decode in AppKit");
        assert!(image.isValid());
        assert_eq!(image.size().width, 512.);
        assert_eq!(image.size().height, 512.);
        let representations = image.representations();
        assert_eq!(representations.len(), 10);
        for points in [16, 32, 128, 256, 512] {
            for scale in [1, 2] {
                assert!(
                    representations.iter().any(|rep| {
                        rep.pixelsWide() == points * scale
                            && rep.pixelsHigh() == points * scale
                            && rep.size().width == points as f64
                            && rep.size().height == points as f64
                    }),
                    "missing {points}pt @{scale}x representation"
                );
            }
        }
    }
}

#[test]
fn embedded_icon_is_a_nonempty_1024_square_png() {
    let stable = include_bytes!("../../../../assets/icons/herdr-ui-icon-clean.png").as_slice();
    let worktree = include_bytes!("../../../../assets/icons/herdr-worktree-1024.png").as_slice();
    assert_ne!(stable, worktree);
    let personal = include_bytes!("../../../../assets/personal/icon.png").as_slice();
    for png in [stable, worktree, personal] {
        assert!(png.len() > 33);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..20], &1024_u32.to_be_bytes());
        assert_eq!(&png[20..24], &1024_u32.to_be_bytes());
    }
    assert_eq!(PNG, personal);
}
