package org.craftocto.craftkit;

import android.inputmethodservice.InputMethodService;
import android.util.Log;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;

/**
 * The instrument: a keyboard of the tree's own, used to measure the bridge.
 *
 * <p>The bridge is: the platform's keyboard → the view that has the focus →
 * the connection that view serves → the tree. On a phone that will not let
 * the shell inject a touch (§256) there is no way to press a key on the
 * platform's own keyboard, so the platform's half of the path is measured
 * with a keyboard of the tree's own instead.
 *
 * <p>It is the <em>same</em> path: the platform asks the focused view for its
 * connection and hands that connection to whichever keyboard is active, so
 * this service commits through exactly the connection the platform's own
 * keyboard would commit through — {@link CraftkitInputConnection}, the tree's
 * own.
 *
 * <p>It types a word of its own a moment after the platform starts it, so a
 * measurement needs nothing but the shell: the tree's own log says the word
 * arrived (§257).
 *
 * <p>It is in the **gated** package only. The tree's default package carries
 * no Java at all (§236).
 */
public class CraftkitTestIme extends InputMethodService {

    /** The word this instrument types. */
    private static final String PROBE = "probe";

    @Override
    public void onCreate() {
        super.onCreate();
        Log.i("craftkit", "the tree's instrument keyboard is up");
    }

    /**
     * The connection the platform's keyboard talks to.
     *
     * <p>The platform asks the <em>focused view</em> for the connection and
     * hands it to whichever keyboard is active, so this service never asks
     * for one: it uses the one it is given, and that one is the tree's own
     * ({@link CraftkitInputConnection}), because the tree's view is the one
     * with the focus.
     */

    /**
     * Type the probe word.
     *
     * <p>The platform starts the keyboard when the focused view asks for it,
     * and the tree's view asks for it the moment the tree wants the platform's
     * keyboard. The word is typed a moment later, because the connection is
     * only handed over once the keyboard is really up.
     */
    @Override
    public void onStartInput(EditorInfo attribute, boolean restarting) {
        super.onStartInput(attribute, restarting);
        android.os.Handler handler = new android.os.Handler(getMainLooper());
        handler.postDelayed(
                new Runnable() {
                    @Override
                    public void run() {
                        InputConnection connection = getCurrentInputConnection();
                        if (connection == null) {
                            Log.w("craftkit", "the instrument keyboard has no connection to type into");
                            return;
                        }
                        Log.i("craftkit", "the instrument keyboard types " + PROBE);
                        connection.commitText(PROBE, 1);
                    }
                },
                1500);
    }
}
