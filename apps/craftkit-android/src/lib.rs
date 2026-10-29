//! The Android package.
//!
//! Android's `NativeActivity` opens this shared library and looks up **one**
//! symbol in it by name: `android_main`. That is the whole of this crate —
//! everything else is the application, in the library next door.
//!
//! ## The one unsafe attribute
//!
//! A function with a fixed name in a binary needs `#[unsafe(no_mangle)]`, and
//! the tree's `unsafe_code` lint covers unsafe attributes. `forbid` cannot be
//! relaxed by an `allow`, so this crate **denies** unsafe rather than
//! forbidding it, and the exception is **one attribute on one function** —
//! there is no `unsafe` block, no `unsafe` function and no `unsafe` impl in
//! this crate at all.
//!
//! `cargo xtask lists` knows the exception, names it, and checks that it is
//! still exactly that. §236.
//!
//! ## Why the entry point is behind a target
//!
//! The package is a **phone** application: it is built for an Android target
//! and it runs on a phone. The workspace, though, is also built for the
//! machine it is developed on, and `winit` has no `android` module on Linux —
//! so a crate that named it unconditionally would not compile on the
//! developer's own machine, and a gate that runs on the developer's machine
//! would fail for a reason that has nothing to do with the code.
//!
//! So the entry point is behind `cfg(target_os = "android")`, and on every
//! other target this crate is **empty**: it has no symbol the platform could
//! look up, which is correct, because on every other target there is no
//! platform to look it up. §236.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![deny(unsafe_code)]

/// The platform's entry point.
///
/// Android's loader opens this library and calls this, on a thread of its
/// own; the window, the renderer and the input all come from there.
///
/// **The Rust ABI**, because that is what the glue declares: the glue's own
/// `extern "Rust" { pub fn android_main(app: AndroidApp); }` is the other
/// side of this call, and an `extern "C"` here would be a different
/// signature.
///
/// There is no command line on a phone — the launcher starts the application —
/// so it opens the **sample** design, which is what the desktop application
/// opens with `--sample`.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // Who asks the platform for its keyboard: the Java half, which owns the
    // view the platform's keyboard talks to. §257.
    #[cfg(feature = "platform-ime")]
    craftkit::ime::set_asker(Box::new(|wanted| ime::set_keyboard_wanted(wanted)));

    // The control channel, when the tree's own folder says there is one.
    //
    // A phone has no command line, so the port comes from a file in the
    // tree's own folder on the phone: `control.port`. It is how the phone is
    // driven at all — the shell on this device may not inject touches
    // (§256) — and the token goes in the same folder, to be read back the
    // same way the crash reports are read back (§249).
    let flags = match control_port(&app) {
        Some(port) => craftkit::Flags::sample_with_control(port),
        None => craftkit::Flags::sample(),
    };
    let _ = craftkit::run_android(flags, app);
}

/// The port the tree's own folder asks for, if it asks for one.
///
/// The file is the tree's own, in the tree's own folder, and it is read with
/// the tree's own bounded reader: a file that says something that is not a
/// port is not a port.
///
/// It is behind the same target as the entry point: `winit`'s Android
/// module does not exist on any other platform, and a crate that named it
/// unconditionally would not compile on the developer's own machine — and a
/// gate that runs on the developer's own machine would fail for a reason
/// that has nothing to do with the code. §236.
#[cfg(target_os = "android")]
fn control_port(app: &winit::platform::android::activity::AndroidApp) -> Option<u16> {
    let folder = app.internal_data_path()?.join(craftkit_foundation::paths::APP_NAME);
    let text = std::fs::read_to_string(folder.join("control.port")).ok()?;
    let port: u16 = text.trim().parse().ok()?;
    Some(port)
}

/// The platform's own keyboard, behind the gate. §257.
///
/// The module is compiled **only** with the `platform-ime` feature, and it
/// is the second named exception to the tree's one unsafe rule: the bridge
/// the platform's Java calls has to have the JNI's calling convention.
///
/// The `allow` is on the module, not on the crate: the crate still **denies**
/// unsafe code, and everything outside this module is still checked by
/// `the_unsafe_exception_has_not_grown`.
#[cfg(all(target_os = "android", feature = "platform-ime"))]
#[allow(unsafe_code)]
mod ime;
