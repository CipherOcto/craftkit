// craftkit's service worker.
//
// One job: let a plan be opened again with no network. Everything else about
// the web build is in the wasm; this file only decides what the browser is
// allowed to keep.
//
// The rules, and why each is here:
//
//   * Only GET. A non-GET request is never a cache hit, and answering one from
//     a cache is how a service worker turns a harmless bug into a data bug.
//   * Only same-origin. Never cache a response from another origin, whatever
//     it claims.
//   * Only 200. An opaque or redirected response is not ours to store.
//   * Network first, cache second. A cached wasm blob that has gone stale is
//     worse than a slow one: the user gets yesterday's application with no
//     indication that anything is wrong. Offline is the case we optimise for,
//     not the case we prefer.
//   * Bounded. The cache is capped and evicted oldest-first, so a session that
//     reloads fifty times cannot fill the origin's quota.
//
// This is JavaScript because browsers only run service workers as JavaScript.
// It is the only script in the project besides the loader in index.html, and
// `cargo xtask web-offline-test` checks it still parses and still obeys the
// rules above.

const CACHE = "craftkit-v1";

/// The app shell: everything needed to start with no network at all.
///
/// The bundle itself is listed here even though it is generated, because the
/// point of the worker is precisely to have it when the network is gone. A
/// cache miss on it falls through to the network and is cached on the way back.
///
/// The two `dist/` paths are the names `cargo xtask bundle` writes, and
/// `cargo xtask web` checks that they are the ones the page imports. A rename
/// that misses this file leaves the page cached and the application missing,
/// which is a blank page for exactly the user who cannot get a new one.
const SHELL = [
  "./",
  "./index.html",
  "./sw.js",
  "./dist/craftkit_web.js",
  "./dist/craftkit_web_bg.wasm",
];

/// How many responses to keep. Small on purpose: this is an app shell, not a
/// media library, and a quota error during a save would be far worse than a
/// cache miss on the next load.
const MAX_ENTRIES = 24;

self.addEventListener("install", (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(CACHE);
      // One at a time, and a missing file is skipped rather than fatal: a
      // developer who has not run `cargo xtask wasm` should still get an app
      // shell that works online.
      await Promise.all(
        SHELL.map((url) => cache.add(url).catch((e) => console.warn("not cached", url, e))),
      );
      await self.skipWaiting();
    })(),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      // Drop every cache that is not ours. A renamed cache must not leave the
      // old one holding megabytes nobody will ever evict.
      const names = await caches.keys();
      await Promise.all(names.filter((n) => n !== CACHE).map((n) => caches.delete(n)));
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;

  if (request.method !== "GET") return;

  let url;
  try {
    url = new URL(request.url);
  } catch {
    // An unparseable URL is not ours to answer.
    return;
  }
  if (url.origin !== self.location.origin) return;

  event.respondWith(
    (async () => {
      try {
        const response = await fetch(request);
        if (response && response.status === 200 && response.type === "basic") {
          const cache = await caches.open(CACHE);
          await cache.put(request, response.clone());
          await trim(cache);
        }
        return response;
      } catch (e) {
        // Offline, or the server is gone. This is the case the worker exists
        // for, so the fallback is the cached shell rather than an error page.
        const cached = await caches.match(request);
        if (cached) return cached;
        if (request.mode === "navigate") {
          const shell = await caches.match("./index.html");
          if (shell) return shell;
        }
        throw e;
      }
    })(),
  );
});

/// Evict oldest-first until the cache is back under [`MAX_ENTRIES`].
async function trim(cache) {
  const keys = await cache.keys();
  if (keys.length <= MAX_ENTRIES) return;
  // Cache keys have no timestamp, so this is insertion order — which is close
  // enough for a shell of two dozen entries and never wrong in a way that
  // matters.
  for (const key of keys.slice(0, keys.length - MAX_ENTRIES)) {
    await cache.delete(key);
  }
}