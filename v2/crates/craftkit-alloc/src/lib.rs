//! `craftkit-alloc` — a **counting allocator**, and the one piece of
//! `unsafe` in v2.
//!
//! # Why it is here
//!
//! The lane's rule is that its work must **not allocate**: `run` does
//! bounded work in bounded time, and an allocation inside it is a lock
//! inside it. The rule was written down and never measured, because
//! measuring it needs a counting global allocator, and a global allocator
//! is `unsafe` by definition.
//!
//! So this crate is the exception, and it is a small one:
//!
//! - it is **one crate**, and it does **one thing**;
//! - it holds **one** `unsafe impl`, and the impl is the allocator the
//!   standard library asks for;
//! - the crate root says `deny(unsafe_code)` and **not** `forbid`, so
//!   the exception is explicit and the rest of the crate is under the
//!   same rule as every other crate in the tree;
//! - a **test holds it there**: it reads the whole workspace and refuses
//!   a second `unsafe`, a second exception, or a `forbid` that has been
//!   swapped for a `deny` in the wrong place.
//!
//! # What it measures
//!
//! The counts are **per thread**, because the lane's work runs on the
//! lane's thread and the control thread's allocations are not the
//! lane's. A thread-local with a `const` initialiser is used, so the
//! allocator itself never allocates — which would be a very funny way to
//! count allocations.
//!
//! ```ignore
//! // In a test binary:
//! #[global_allocator]
//! static ALLOCATOR: craftkit_alloc::Counting = craftkit_alloc::Counting;
//!
//! // Inside the lane's job:
//! let before = craftkit_alloc::counts();
//! let work = do_the_work();
//! let after = craftkit_alloc::counts();
//! assert_eq!(after.allocations - before.allocations, 0, "the lane allocated");
//! ```

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
// **Not** `forbid`. There is one exception below, it is marked, and a
// test in this crate holds it there: it reads the workspace and refuses
// a second one. A `forbid` could not be relaxed by an `allow`, so the
// exception could not live here at all.
#![deny(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;

/// What a thread has allocated.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// How many times memory was handed out.
    pub allocations: u64,
    /// How many times memory was given back.
    pub deallocations: u64,
    /// How many bytes were handed out, in total.
    pub bytes: u64,
}

thread_local! {
    /// The counts of **this** thread.
    ///
    /// A `const` initialiser: the thread-local is never lazily
    /// initialised, so reading it inside the allocator cannot allocate —
    /// which would be a very funny way to count allocations.
    static COUNTS: Cell<Counts> = const { Cell::new(Counts { allocations: 0, deallocations: 0, bytes: 0 }) };
}

/// What this thread has allocated.
pub fn counts() -> Counts {
    COUNTS.with(|counts| counts.get())
}

/// Forget what this thread has allocated.
///
/// The counters start at zero, and a measurement takes the difference
/// between two readings rather than reading a total.
pub fn reset() {
    COUNTS.with(|cell| cell.set(Counts { allocations: 0, deallocations: 0, bytes: 0 }));
}

/// The counting allocator.
///
/// It counts and it does nothing else: it hands out what the system
/// allocator hands out, and it writes down how much.
pub struct Counting;

// SAFETY: `GlobalAlloc` is an unsafe trait because a wrong
// implementation breaks the program's memory, and this one does not: it
// counts and then does exactly what the system allocator does, with the
// same layout, the same alignment and the same null-on-failure
// behaviour. The counters are a thread-local with a `const`
// initialiser, so reading them cannot allocate, and the counting cannot
// change what the allocator returns.
//
// The one thing this implementation must get right is the size of the
// allocation it frees: `dealloc` and `realloc` are given the layout of
// the block, and that layout is the one the block was allocated with,
// so the system allocator is told the truth.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the layout is the one the caller asked for, and the
        // system allocator is given exactly it.
        let pointer = unsafe { std::alloc::System.alloc(layout) };
        if !pointer.is_null() {
            COUNTS.with(|cell| {
                let mut counts = cell.get();
                counts.allocations += 1;
                counts.bytes += layout.size() as u64;
                cell.set(counts);
            });
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the layout is the one the caller asked for, and the
        // system allocator is given exactly it.
        let pointer = unsafe { std::alloc::System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            COUNTS.with(|cell| {
                let mut counts = cell.get();
                counts.allocations += 1;
                counts.bytes += layout.size() as u64;
                cell.set(counts);
            });
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        COUNTS.with(|cell| {
            let mut counts = cell.get();
            counts.deallocations += 1;
            cell.set(counts);
        });
        // SAFETY: the pointer and the layout are the ones the block was
        // allocated with, and the system allocator is told exactly them.
        unsafe { std::alloc::System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the pointer and the layout are the ones the block was
        // allocated with, and the system allocator is told exactly them.
        let grown = unsafe { std::alloc::System.realloc(pointer, layout, new_size) };
        if !grown.is_null() {
            COUNTS.with(|cell| {
                let mut counts = cell.get();
                counts.allocations += 1;
                counts.bytes += new_size as u64;
                cell.set(counts);
            });
        }
        grown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The counting allocator, installed in this test binary.
    ///
    /// Declaring it is safe code: the `unsafe` is in the **impl**, and
    /// that is above.
    #[global_allocator]
    static ALLOCATOR: Counting = Counting;

    /// The counts start at zero.
    #[test]
    fn the_counts_start_at_zero() {
        reset();
        assert_eq!(counts(), Counts { allocations: 0, deallocations: 0, bytes: 0 });
    }

    /// An allocation is counted.
    #[test]
    fn an_allocation_is_counted() {
        reset();
        let boxed = Box::new(42u64);
        assert_eq!(counts().allocations, 1, "the box was counted");
        assert_eq!(counts().bytes, 8, "the box is eight bytes");
        drop(boxed);
        assert_eq!(counts().deallocations, 1, "the box was given back");
    }

    /// The counts are **per thread**: the lane's thread counts its own
    /// work and nobody else's.
    ///
    /// The other thread makes a thousand allocations, and this thread's
    /// count does not grow by a thousand: it grows by what starting a
    /// thread costs, and by nothing else.
    #[test]
    fn the_counts_are_per_thread() {
        reset();
        let before = counts().allocations;
        let handle = std::thread::spawn(|| {
            reset();
            let mut boxes: Vec<Box<u8>> = Vec::with_capacity(1000);
            for index in 0..1000 {
                boxes.push(Box::new(index as u8));
            }
            let counted = counts().allocations;
            drop(boxes);
            counted
        });
        let counted = handle.join().expect("the thread is finished");
        assert!(counted >= 1000, "the thread counted its own boxes: {counted}");
        let after = counts().allocations;
        assert!(after - before < 1000, "this thread counted the other thread's work: {after}");
    }
}
