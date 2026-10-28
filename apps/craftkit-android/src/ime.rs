//! The platform's own keyboard, behind the gate.
//!
//! **This is the unsafe half of the bridge, and it is the second named
//! exception to the tree's one rule** (§236, §257).
//!
//! The tree's rule is that a crate either forbids unsafe code or — the one
//! named exception, the Android package — denies it, and holds nothing but
//! the `no_mangle` attribute on the function the platform looks up. The
//! platform's keyboard needs one more thing: a function the **platform's
//! Java** calls, with the JNI's calling convention, to hand over the text
//! the user typed.
//!
//! So the exception grows by exactly one module, and only behind the gate:
//!
//! - the module is compiled **only** with the `platform-ime` feature, so
//!   the tree's default build — the one that carries no Java and no dex —
//!   holds no unsafe code at all;
//! - the gate `the_unsafe_exception_has_not_grown` knows about the module
//!   and checks that everything outside it is still clean.
//!
//! The Java half is two small files in
//! `apps/craftkit-android/java/org/craftocto/craftkit/`, and the text it
//! hands over goes straight into the app's queue (`craftkit::ime`), which
//! is safe.

use std::sync::Mutex;

use jni::EnvUnowned;
use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::refs::Global;

/// The activity, handed over by the Java half at startup.
///
/// It is held here because a native thread cannot find the tree's own class:
/// the lookup goes through the **system** class loader, which has never heard
/// of it. The activity hands itself over instead, and this keeps that one
/// reference.
static ACTIVITY: Mutex<Option<Global<JObject<'static>>>> = Mutex::new(None);

/// The text the platform's keyboard committed.
///
/// The name is the one the platform looks up: the class, then the method.
/// A `static native` method in Java is found by that name and nothing else.
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_craftocto_craftkit_CraftkitInputConnection_nativeCommit<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    text: JString<'local>,
) {
    // The text is read out of the platform's string and given to the app's
    // queue. A string the platform will not give is a keystroke nobody typed:
    // the policy logs it and the default is the empty string, and an empty
    // string is not typed anywhere.
    let outcome = unowned_env.with_env(|env| {
        let chars = text.mutf8_chars(env)?;
        Ok::<String, jni::errors::Error>(chars.to_string())
    });
    let text = outcome.resolve::<LogErrorAndDefault>();
    if !text.is_empty() {
        craftkit::ime::commit(text);
    }
}

/// The activity, handed over by the Java half.
///
/// The name is the one the platform looks up: the class, then the method.
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_craftocto_craftkit_CraftkitActivity_nativeSetActivity<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    activity: JObject<'local>,
) {
    let outcome = unowned_env.with_env(|env| {
        let reference = env.new_global_ref(activity)?;
        Ok::<Global<JObject<'static>>, jni::errors::Error>(reference)
    });
    let reference = outcome.resolve::<LogErrorAndDefault>();
    let Ok(mut slot) = ACTIVITY.lock() else { return };
    *slot = Some(reference);
}

/// Show or hide the platform's keyboard.
///
/// The tree's frame asks for this when one of its fields has the focus. The
/// call goes to the Java half, which owns the view the platform's keyboard
/// talks to.
pub fn set_keyboard_wanted(wanted: bool) {
    let Ok(slot) = ACTIVITY.lock() else { return };
    let Some(activity) = slot.as_ref() else {
        eprintln!("the platform's keyboard was asked for, but the activity never handed itself over");
        return;
    };
    let Ok(vm) = jni::JavaVM::singleton() else { return };
    let outcome = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let class = env.get_object_class(activity)?;
        let signature = jni::signature::RuntimeMethodSignature::from_str("(Z)V")?;
        env.call_static_method(
            &class,
            jni::jni_str!("setKeyboardWanted"),
            &signature.method_signature(),
            &[JValue::Bool(wanted)],
        )?;
        Ok(())
    });
    if let Err(e) = outcome {
        eprintln!("the platform's keyboard could not be asked for: {e}");
    }
}
