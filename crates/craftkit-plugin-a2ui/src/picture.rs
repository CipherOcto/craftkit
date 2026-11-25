//! The **picture** an image names.
//!
//! The tree fetches **nothing** over a network: an image is a **file**
//! the agent names, and the file is a PNG. The tree's own decoder reads
//! it — the same one the image battery uses — and the picture comes back
//! as four bytes a pixel.
//!
//! The read is **bounded**, because the path comes from outside: a file
//! over the limit is refused, and not truncated. And the picture is not
//! stored: the node carries the **path**, and the picture is read again
//! when the tree is drawn, because a document that kept the pixels of
//! every picture would keep a copy of every file it ever showed.
//!
//! A path that is not a file, or a file that is not a PNG, is refused —
//! not fetched, not guessed at.

use craftkit_sdk::picture::Picture;

/// The picture in a file, if the file holds one.
///
/// The file is read up to the limit, and the reader says whether it
/// **stopped**: a file over the limit is over it, and it is not a picture
/// this function can hold.
pub fn picture_of(path: &str) -> Option<Picture> {
    let path = std::path::Path::new(path);
    let (bytes, stopped) =
        craftkit_sdk::files::read_prefix_bounded(path, crate::MAX_A2UI_PICTURE_BYTES).ok()?;
    if stopped {
        return None;
    }
    let picture = craftkit_png::read(&bytes, &craftkit_png::Limits::default()).ok()?;
    Some(Picture::from_samples(picture.width, picture.height, picture.channels, picture.samples))
}
