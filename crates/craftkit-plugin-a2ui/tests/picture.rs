//! A **picture** in a file, and the bound on how much of it is read.
//!
//! The picture of a component is a file beside the document, and the
//! file is read up to a bound. A file beyond the bound is **not** a
//! picture this plugin can hold: the reader stops at the bound and says
//! so, and the picture is not read.

use craftkit_plugin_a2ui::MAX_A2UI_PICTURE_BYTES;

// The plugin's anchor, held by the test: without it the linker drops the
// plugin's items and the registry is empty.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_a2ui::CRAFTKIT_PLUGIN_ANCHOR;

/// A **PNG file** of a picture, written with the tree's own writer so the
/// tree carries no artwork.
fn a_png_file(name: &str) -> String {
    let image = craftkit_raster::Image::new(2, 2, craftkit_sdk::document::Colour::WHITE);
    let path =
        craftkit_scratch::file(&format!("craftkit-v2-a2ui-picture-{name}.png")).expect("the scratch is made");
    std::fs::write(&path, image.to_png()).expect("the picture is written");
    path.to_string_lossy().to_string()
}

/// **A picture inside the bound is read.**
#[test]
fn a_picture_inside_the_bound_is_read() {
    let path = a_png_file("whole");
    let picture = craftkit_plugin_a2ui::picture::picture_of(&path).expect("the picture is read");
    assert_eq!(picture.width, 2, "the picture is two pixels across");
    assert_eq!(picture.height, 2, "the picture is two pixels down");
}

/// **A picture beyond the bound is not read.**
///
/// The file is bigger than the bound by one byte, so there is something
/// beyond it to have missed: the reader stops, and the picture is
/// nothing.
#[test]
fn a_picture_beyond_the_bound_is_not_read() {
    let mut bytes = std::fs::read(a_png_file("big")).expect("the picture is read");
    bytes.resize(MAX_A2UI_PICTURE_BYTES as usize + 1, 0);
    let path = craftkit_scratch::file("craftkit-v2-a2ui-picture-big.png").expect("the scratch is made");
    std::fs::write(&path, &bytes).expect("the picture is written");
    let path = path.to_string_lossy().to_string();
    assert!(
        craftkit_plugin_a2ui::picture::picture_of(&path).is_none(),
        "a file beyond the bound is not a picture this plugin can hold"
    );
}
