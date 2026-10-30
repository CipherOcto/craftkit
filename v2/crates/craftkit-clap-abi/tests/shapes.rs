//! The **shapes** of the format, and their layout.
//!
//! A binary contract is a set of shapes, and the shapes are only a
//! contract if the two sides agree on them **byte for byte**. So the
//! offsets of the fields are checked here: a field that moved would be
//! a field the host reads at the wrong place, and the wrong place is
//! a crash in the middle of somebody's audio.

use std::mem::{align_of, offset_of, size_of};

use craftkit_clap_abi::{
    ClapPluginDescriptor, ClapPluginEntry, ClapPluginFactory, ClapVersion, ENTRY_SYMBOL, PLUGIN_FACTORY_ID,
};

/// The entry symbol is the name the host looks for.
#[test]
fn the_entry_symbol_is_the_names_the_format_uses() {
    assert_eq!(ENTRY_SYMBOL, "clap_entry", "the entry is found by name");
}

/// The factory is asked for by the format's own id.
#[test]
fn the_factory_is_asked_for_by_its_own_id() {
    assert_eq!(PLUGIN_FACTORY_ID, "clap.plugin-factory", "the factory has one id");
}

/// The version rule is the format's own: the major versions must
/// agree.
#[test]
fn the_major_version_decides_compatibility() {
    let host = ClapVersion::CURRENT;
    assert!(host.is_compatible_with(host), "the same version is compatible");
    assert!(
        host.is_compatible_with(ClapVersion { major: 1, minor: 9, revision: 7 }),
        "the minor version does not decide"
    );
    assert!(
        !host.is_compatible_with(ClapVersion { major: 2, minor: 0, revision: 0 }),
        "a newer major version is refused"
    );
    assert!(
        !host.is_compatible_with(ClapVersion { major: 0, minor: 9, revision: 7 }),
        "an older major version is refused"
    );
}

/// The entry's layout: the version first, and then the three pointers
/// the format puts there.
#[cfg(target_pointer_width = "64")]
#[test]
fn the_entry_has_the_layout_the_format_has() {
    assert_eq!(offset_of!(ClapPluginEntry, clap_version), 0, "the version is first");
    assert_eq!(offset_of!(ClapPluginEntry, init), 16, "then `init`");
    assert_eq!(offset_of!(ClapPluginEntry, deinit), 24, "then `deinit`");
    assert_eq!(offset_of!(ClapPluginEntry, get_factory), 32, "then `get_factory`");
    assert_eq!(size_of::<ClapPluginEntry>(), 40, "and nothing after them");
    assert_eq!(align_of::<ClapPluginEntry>(), 8, "aligned to the pointers");
}

/// The factory's layout: three pointers, in the format's order.
#[cfg(target_pointer_width = "64")]
#[test]
fn the_factory_has_the_layout_the_format_has() {
    assert_eq!(offset_of!(ClapPluginFactory, get_plugin_count), 0, "the count is first");
    assert_eq!(offset_of!(ClapPluginFactory, get_plugin_descriptor), 8, "then the descriptor");
    assert_eq!(offset_of!(ClapPluginFactory, create_plugin), 16, "then the making");
    assert_eq!(size_of::<ClapPluginFactory>(), 24, "and nothing after them");
}

/// The descriptor's layout: the version, then the nine pointers.
#[cfg(target_pointer_width = "64")]
#[test]
fn the_descriptor_has_the_layout_the_format_has() {
    assert_eq!(offset_of!(ClapPluginDescriptor, clap_version), 0, "the version is first");
    assert_eq!(offset_of!(ClapPluginDescriptor, id), 16, "then the id");
    assert_eq!(offset_of!(ClapPluginDescriptor, name), 24, "then the name");
    assert_eq!(offset_of!(ClapPluginDescriptor, vendor), 32, "then the vendor");
    assert_eq!(offset_of!(ClapPluginDescriptor, url), 40, "then the url");
    assert_eq!(offset_of!(ClapPluginDescriptor, manual_url), 48, "then the manual");
    assert_eq!(offset_of!(ClapPluginDescriptor, support_url), 56, "then the support");
    assert_eq!(offset_of!(ClapPluginDescriptor, version), 64, "then the version");
    assert_eq!(offset_of!(ClapPluginDescriptor, description), 72, "then the description");
    assert_eq!(offset_of!(ClapPluginDescriptor, features), 80, "then the features");
    assert_eq!(size_of::<ClapPluginDescriptor>(), 88, "and nothing after them");
}
