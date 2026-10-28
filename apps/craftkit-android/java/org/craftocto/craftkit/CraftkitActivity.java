package org.craftocto.craftkit;

import android.app.NativeActivity;
import android.content.Context;
import android.os.Bundle;
import android.util.Log;
import android.view.ViewGroup;
import android.view.inputmethod.InputMethodManager;

/**
 * The activity, with a view that serves the platform's keyboard.
 *
 * <p>The platform's own {@code NativeActivity} serves a connection that gives
 * the commit to nobody: the soft keyboard opens, the keys are tapped, and the
 * field does not change (§236). This activity is the same activity with one
 * thing added — a small view of the tree's own, which the platform asks for
 * the keyboard through.
 *
 * <p>It is Java, and the tree's rule says the package carries none (§236).
 * That rule is what the {@code platform-ime} gate is for: this file is
 * compiled into a dex **only** when the gate is on, and the tree's default
 * build carries no Java at all.
 */
public class CraftkitActivity extends NativeActivity {

    /** The activity, for the keyboard's own calls. */
    private static CraftkitActivity instance;

    /** The view the platform's keyboard talks to. */
    private static CraftkitInputView inputView;

    /**
     * Hand the activity to the tree.
     *
     * <p>The tree asks for the keyboard from its own frame, and it cannot
     * find this class on its own: a native thread that looks a class up finds
     * it through the <em>system</em> class loader, which has never heard of
     * the tree's classes. The activity hands itself over instead, and the
     * tree keeps that one reference.
     */
    private static native void nativeSetActivity(Object activity);

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        // The library, loaded for the class loader.
        //
        // The platform's own activity loads the native code with `dlopen`,
        // and a library that `dlopen` opened is **not** in the class
        // loader's list: the runtime looks for a native method in the
        // libraries the class loader knows, and this one is not among them.
        // The symptom is measured and exact — `UnsatisfiedLinkError: No
        // implementation found for … nativeSetActivity` — even though the
        // symbol is in the library and the tree's own code is already
        // running (§257).
        //
        // Loading it again through the class loader fixes that: `dlopen` on
        // a library that is already open hands back the same handle and
        // counts it once more, and the class loader now knows the library
        // and its symbols. The name is the one the manifest gives the
        // platform's own loader.
        try {
            System.loadLibrary("craftkit_android");
        } catch (UnsatisfiedLinkError e) {
            // The library is already open, so this is the usual case; the
            // class loader has it now either way.
            Log.w("craftkit", "the native library was already loaded", e);
        }

        // The window's own decor has to be focusable, or **nothing** in the
        // window can take the focus: measured, the decor was not focusable,
        // so the tree's view could not be focused and the platform was
        // served the decor instead of the tree's connection (§257).
        //
        // The window's content is the tree's own surface, and a surface is
        // not focusable — so in a package that carries nothing else, the
        // window has no focusable view at all.
        getWindow().getDecorView().setFocusable(true);
        getWindow().getDecorView().setFocusableInTouchMode(true);

        // The view the platform's keyboard talks to.
        //
        // It is **one pixel**, and not zero: the platform's own rule is that
        // a view with no size cannot take the focus, and the focus is what
        // the platform asks the connection of (§257). One pixel is the
        // smallest that works, and it is not a touch of its own — a view
        // that is not clickable does not consume a touch, so the tree's own
        // surface still gets every one.
        CraftkitInputView input = new CraftkitInputView(this);
        input.setFocusable(true);
        input.setFocusableInTouchMode(true);
        getWindow().addContentView(input, new ViewGroup.LayoutParams(1, 1));

        instance = this;
        inputView = input;
        nativeSetActivity(this);
    }

    /**
     * Keep the focus on the tree's own view.
     *
     * <p>The platform asks the view that has the focus for its input
     * connection, and the window's own decor takes the focus back after
     * {@code onCreate} — measured: the platform was served
     * {@code DecorView}, so the tree's connection was never asked for
     * (§257). The focus is asked for again every time the window gains it,
     * which is the only moment the platform will hand it out.
     */
    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus && inputView != null) {
            inputView.requestFocus();
        }
    }

    /**
     * Show or hide the platform's keyboard.
     *
     * <p>The tree calls this when it wants the platform's keyboard and when
     * it does not. The view asked for is the tree's own, so the platform
     * asks <em>it</em> for the connection, and the connection is the tree's.
     */
    public static void setKeyboardWanted(boolean wanted) {
        if (instance == null || inputView == null) {
            Log.w("craftkit", "the tree asked for the keyboard before the activity was ready");
            return;
        }
        InputMethodManager imm =
                (InputMethodManager) instance.getSystemService(Context.INPUT_METHOD_SERVICE);
        if (imm == null) {
            return;
        }
        if (wanted) {
            // The platform says whether it will show its keyboard, and the
            // answer is worth having: a keyboard that was asked for and did
            // not come is a keyboard nobody can type with.
            Log.i("craftkit", "the platform was asked to show its keyboard: "
                    + imm.showSoftInput(inputView, 0));
        } else {
            imm.hideSoftInputFromWindow(inputView.getWindowToken(), 0);
        }
    }
}
