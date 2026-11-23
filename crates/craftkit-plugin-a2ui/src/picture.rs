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

/// The most bytes a picture's file may hold.
///
/// A picture on a canvas is a few megabytes at most, and the limit is
/// written down here rather than inherited: a path from outside is a
/// number from outside, and the tree bounds them.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// The picture in a file, if the file holds one.
///
/// The file is read **once more than the limit allows**, so that a file
/// over the limit is known to be over it.
pub fn picture_of(path: &str) -> Option<Picture> {
    let path = std::path::Path::new(path);
    let bytes = craftkit_sdk::files::read_prefix_bounded(path, MAX_BYTES + 1).ok()?;
    if bytes.len() as u64 > MAX_BYTES {
        return None;
    }
    let picture = craftkit_png::read(&bytes, &craftkit_png::Limits::default()).ok()?;
    Some(Picture::from_samples(picture.width, picture.height, picture.channels, picture.samples))
}
