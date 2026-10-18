use std::path::Path;

/// The X mark (`x.svg`) is the X social network's logo, for the About box's
/// link to it. A close or clear action takes `close.svg`.
#[test]
fn the_x_mark_only_links_to_x() {
    let needle = ["icons/", "x.svg"].concat();
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut found = Vec::new();
    visit(root, root, &needle, &mut found);
    assert_eq!(found, ["about.rs"]);
}

fn visit(root: &Path, dir: &Path, needle: &str, found: &mut Vec<String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            visit(root, &path, needle, found);
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && std::fs::read_to_string(&path).unwrap().contains(needle)
        {
            found.push(path.strip_prefix(root).unwrap().display().to_string());
        }
    }
}
