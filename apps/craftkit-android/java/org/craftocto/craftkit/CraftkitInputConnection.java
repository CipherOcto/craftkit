package org.craftocto.craftkit;

import android.content.Context;
import android.view.View;
import android.view.inputmethod.BaseInputConnection;

/**
 * The input connection the platform's keyboard talks to.
 *
 * <p>The platform's keyboard commits text by calling {@code commitText} on
 * the connection the activity served. The platform's own connection throws
 * that text away (§236); this one hands it to the tree, through one native
 * method, and the tree types it into the field that has the focus.
 *
 * <p>The tree has no composing region and no selection: the text is
 * committed as it is typed, which is what the tree's own keyboard does
 * (§246). The connection answers the platform's other questions with the
 * platform's own defaults.
 */
public class CraftkitInputConnection extends BaseInputConnection {

    /**
     * The connection.
     *
     * <p>The two-argument constructor is the one every platform has; the
     * three-argument one arrived later.
     */
    public CraftkitInputConnection(Context context, View targetView) {
        super(targetView, true);
    }

    /**
     * Hand the text to the tree.
     *
     * <p>The name is the one the platform looks up: the class, then the
     * method. A {@code static native} method is found by that name and by
     * nothing else.
     */
    private static native void nativeCommit(String text);

    /**
     * The platform's keyboard commits the text here.
     *
     * <p>This is the two-argument form, which is the one every platform has:
     * the three-argument form was added later and delegates to this one.
     */
    @Override
    public boolean commitText(CharSequence text, int newCursorPosition) {
        if (text == null || text.length() == 0) {
            return true;
        }
        android.util.Log.i("craftkit", "the platform committed: " + text);
        nativeCommit(text.toString());
        return true;
    }

    @Override
    public boolean setComposingText(CharSequence text, int newCursorPosition) {
        // The tree has no composing region, so the text is committed as it
        // is typed. The platform's own default would hold it back until the
        // word was finished, and the tree's fields have no such rule.
        return commitText(text, newCursorPosition);
    }

    @Override
    public boolean finishComposingText() {
        return true;
    }
}
