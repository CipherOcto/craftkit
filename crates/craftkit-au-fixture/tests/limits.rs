//! The **limits of the au fixture**: the numbers it will not go past.
//!
//! The fixture is a **reference**: it is the plugin the tree's host
//! opens, and it holds a fixed number of slots. A limit that no test
//! names is a limit nothing proves holds.

use craftkit_au_fixture::MAX_INSTANCES;

/// The fixture holds **MAX_INSTANCES** instances at a time.
///
/// The slots are taken one after another, and a host that asks for one
/// more than the fixture holds gets **nothing**: the fixture answers
/// with a null pointer, and the host says so.
#[test]
fn the_fixture_holds_its_own_number_of_instances() {
    let mut made = 0;
    for _ in 0..MAX_INSTANCES {
        if !craftkit_au_fixture::make_an_instance() {
            break;
        }
        made += 1;
    }
    assert_eq!(made, MAX_INSTANCES, "the fixture holds its instances");
    assert!(!craftkit_au_fixture::make_an_instance(), "and no more than that");
}
