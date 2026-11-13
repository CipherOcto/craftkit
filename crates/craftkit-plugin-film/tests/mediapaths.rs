//! The **path** of a media file.
//!
//! A file that names another file may name it in three ways, and the
//! tree's own rules say what each one means.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A file of this test's own, in a **folder** of its own.
fn a_file(name: &str, text: &str) -> String {
    let folder = craftkit_scratch::dir(&format!("craftkit-v2-mediapaths-{name}-{}", std::process::id()))
        .expect("the directory is made");
    let path = folder.join(format!("{name}.otio"));
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A **time**: the rate and the frames.
fn time(rate: f64, frames: f64) -> Json {
    json!({ "OTIO_SCHEMA": "RationalTime.1", "rate": rate, "value": frames })
}

/// A **range**: where it begins and how long it is.
fn range(rate: f64, start: f64, duration: f64) -> Json {
    json!({
        "OTIO_SCHEMA": "TimeRange.1",
        "start_time": time(rate, start),
        "duration": time(rate, duration),
    })
}

/// An OTIO file with **one clip** whose media is at the given url.
fn an_otio(url: &str) -> String {
    json!({
        "OTIO_SCHEMA": "Timeline.1",
        "name": "the timeline",
        "global_start_time": time(25.0, 0.0),
        "tracks": {
            "OTIO_SCHEMA": "Stack.1",
            "name": "the tracks",
            "children": [{
                "OTIO_SCHEMA": "Track.1",
                "name": "the track",
                "kind": "Video",
                "children": [{
                    "OTIO_SCHEMA": "Clip.2",
                    "name": "the clip",
                    "source_range": range(25.0, 0.0, 100.0),
                    "effects": [],
                    "markers": [],
                    "enabled": true,
                    "media_references": {
                        "DEFAULT_MEDIA": {
                            "OTIO_SCHEMA": "ExternalReference.1",
                            "name": "the clip",
                            "target_url": url,
                            "available_range": range(25.0, 0.0, 100.0),
                            "metadata": {}
                        }
                    },
                    "active_media_reference_key": "DEFAULT_MEDIA",
                    "metadata": {}
                }]
            }]
        }
    })
    .to_string()
}

/// The **path** of the media of the clip that was read.
///
/// It is a **path** and not a text: how a path is spelled is the
/// platform's business, and one path may be spelled in more than one
/// way. A path is compared with a path, never with a text.
fn media_path(session: &mut Session, path: &str) -> std::path::PathBuf {
    session.execute("film.import.otio", &json!({ "path": path })).expect("the file is read");
    std::path::PathBuf::from(
        session
            .document()
            .walk()
            .find(|node| node.kind == "film.item")
            .and_then(|node| node.data.get("path"))
            .and_then(|path| path.as_str())
            .unwrap_or_default(),
    )
}

/// The **path** of a media file that is named by a **relative** path:
/// the folder of the file that names it, and the two parts of the
/// name.
fn joined(folder: &std::path::Path, name: &str, file: &str) -> std::path::PathBuf {
    folder.join(name).join(file)
}

/// A **file URL** is a path in disguise: the scheme is taken off and
/// the host `localhost` is dropped.
#[test]
fn a_file_url_is_a_path_in_disguise() {
    let mut session = session();
    let path = a_file("url", &an_otio("file:///media/the%20clip.mov"));
    assert_eq!(
        media_path(&mut session, &path),
        std::path::PathBuf::from("/media/the clip.mov"),
        "the URL is a path"
    );
    let path = a_file("host", &an_otio("file://localhost/media/the%20clip.mov"));
    assert_eq!(
        media_path(&mut session, &path),
        std::path::PathBuf::from("/media/the clip.mov"),
        "and the host is dropped"
    );
}

/// An **absolute path** is a path already, and it stays as it is.
#[test]
fn an_absolute_path_stays_as_it_is() {
    let mut session = session();
    let path = a_file("absolute", &an_otio("/media/the clip.mov"));
    assert_eq!(
        media_path(&mut session, &path),
        std::path::PathBuf::from("/media/the clip.mov"),
        "the path is the path"
    );
}

/// A **relative path** is relative to the folder of the file that
/// names it.
#[test]
fn a_relative_path_is_relative_to_the_folder_of_the_file() {
    let mut session = session();
    let path = a_file("relative", &an_otio("media/the clip.mov"));
    let expected = joined(
        std::path::Path::new(&path).parent().expect("the file is in a folder"),
        "media",
        "the clip.mov",
    );
    assert_eq!(media_path(&mut session, &path), expected, "the path is joined to the folder of the file");
}

/// A `..` in a relative path is **one folder up**.
#[test]
fn a_dot_dot_is_one_folder_up() {
    let mut session = session();
    let path = a_file("up", &an_otio("../media/the clip.mov"));
    let expected = joined(
        std::path::Path::new(&path)
            .parent()
            .and_then(|folder| folder.parent())
            .expect("the folder of the file is in a folder"),
        "media",
        "the clip.mov",
    );
    assert_eq!(media_path(&mut session, &path), expected, "the path goes up one folder");
}
