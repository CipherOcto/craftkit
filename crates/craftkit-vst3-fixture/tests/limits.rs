//! The **limits of the vst3 fixture**: the numbers it will not go past.
//!
//! The fixture is a **reference**: it is the plugin the tree's host
//! opens, and it holds a fixed number of slots. A limit that no test
//! names is a limit nothing proves holds.

use craftkit_vst3_fixture::MAX_VST3_INSTANCES;

/// The fixture holds **MAX_VST3_INSTANCES** instances at a time.
///
/// The slots are taken one after another, and a host that asks for one
/// more than the fixture holds gets **nothing**: the fixture answers
/// with a null pointer, and the host says so.
#[test]
fn the_fixture_holds_its_own_number_of_instances() {
    let mut made = 0;
    for _ in 0..MAX_VST3_INSTANCES {
        if !craftkit_vst3_fixture::make_an_instance() {
            break;
        }
        made += 1;
    }
    assert_eq!(made, MAX_VST3_INSTANCES, "the fixture holds its instances");
    assert!(!craftkit_vst3_fixture::make_an_instance(), "and no more than that");
}
