package org.craftocto.craftkit;

import android.content.Context;
import android.util.Log;
import android.view.View;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;

/**
 * The view the platform's keyboard talks to.
 *
 * <p>The platform asks the view that has the focus for its input connection,
 * and the tree's window has no view of its own: the platform's
 * {@code NativeActivity} puts its native surface there. So the activity adds
 * one small view of its own — one that is **one pixel**, so it never covers
 * a touch and never covers the tree — and asks the platform for the keyboard
 * through it.
 *
 * <p>Two methods are the whole of it: {@link #onCheckIsTextEditor} says the
 * view is a text editor, and {@link #onCreateInputConnection} says which
 * connection the keyboard should use.
 */
public class CraftkitInputView extends View {

    public CraftkitInputView(Context context) {
        super(context);
    }

    /**
     * Ask for the focus when the view is attached to the window.
     *
     * <p>This is the moment the focus can be had: measured, the activity's
     * own {@code onCreate} asked for it and the view was not attached yet
     * ({@code requestFocus} said yes and did nothing), and the window's own
     * focus callback asked again and was refused (§257). A view that is
     * attached can take the focus.
     */
    @Override
    protected void onAttachedToWindow() {
        super.onAttachedToWindow();
        requestFocus();
    }

    /**
     * The connection the platform's keyboard commits into.
     *
     * <p>This is the hook the tree needs and the platform's own activity does
     * not have: the platform's connection gives the commit to nobody (§236).
     */
    @Override
    public InputConnection onCreateInputConnection(EditorInfo attrs) {
        // The platform asked **this** view — the tree's own — for the
        // connection. The line is here because it is the one fact that says
        // the bridge is wired: without the focus on this view the platform
        // asks the window's decor instead, and the tree's connection is
        // never asked for at all (§257).
        Log.i("craftkit", "the platform asked the tree's view for its connection");
        return new CraftkitInputConnection(getContext(), this);
    }

    /**
     * The view is a text editor.
     *
     * <p>Without this the platform does not treat the view as one, and the
     * keyboard is not asked for at all.
     */
    @Override
    public boolean onCheckIsTextEditor() {
        return true;
    }
}
