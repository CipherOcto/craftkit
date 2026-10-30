//! The lane's rule, **measured**.
//!
//! The lane says its work must not allocate: `run` does bounded work in
//! bounded time, and an allocation inside it is a lock inside it. The
//! rule was written down and never measured, because measuring it needs
//! a counting global allocator, and that is `unsafe` by definition.
//!
//! So this test binary installs the counting allocator — the one piece
//! of `unsafe` in v2, in one crate, with a gate that holds it there —
//! and the lane's own job measures itself.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use craftkit_sdk::{Block, Lane, LaneJob, LaneRunner};

/// The counting allocator, installed in this test binary.
///
/// Declaring it is safe code: the `unsafe` is in the **impl**, and that
/// is in the crate that has the exception.
#[global_allocator]
static ALLOCATOR: craftkit_alloc::Counting = craftkit_alloc::Counting;

/// What the job measured.
#[derive(Debug, Default)]
struct Measured {
    /// The allocations before the work.
    before: AtomicU64,
    /// The allocations after the work.
    after: AtomicU64,
    /// The blocks the job ran.
    blocks: AtomicU64,
}

/// A job that measures its own allocations.
///
/// The counters are **per thread**, so the numbers are the lane's
/// thread's numbers and nobody else's.
struct MeasuringJob {
    measured: Arc<Measured>,
}

impl LaneJob for MeasuringJob {
    /// Prepare: the control thread. It may allocate, and it does.
    fn prepare(&mut self, _block: usize) -> Result<(), String> {
        Ok(())
    }

    /// Run: the lane. It must not allocate.
    fn run(&mut self, _frames: usize) -> Block {
        let before = craftkit_alloc::counts().allocations;
        let mut sum = 0u64;
        for index in 0..10_000u64 {
            sum = sum.wrapping_add(index);
        }
        let after = craftkit_alloc::counts().allocations;
        self.measured.before.store(before, Ordering::Relaxed);
        self.measured.after.store(after, Ordering::Relaxed);
        self.measured.blocks.fetch_add(1, Ordering::Relaxed);
        // The work is not the point; the sum is only there so that the
        // compiler cannot throw the loop away.
        if sum == u64::MAX {
            return Block::Failed;
        }
        Block::Played
    }
}

/// The lane's work does not allocate.
///
/// This is the measurement the tree could not make before: the rule is
/// no longer an assertion in a comment, it is a number.
#[test]
fn the_lanes_work_does_not_allocate() {
    let measured = Arc::new(Measured::default());
    let mut lane =
        Lane::new(Box::new(MeasuringJob { measured: Arc::clone(&measured) }), 512).expect("the lane is made");
    for _ in 0..8 {
        lane.block(16);
    }
    let blocks = measured.blocks.load(Ordering::Relaxed);
    assert!(blocks > 0, "the job ran: {blocks}");
    let before = measured.before.load(Ordering::Relaxed);
    let after = measured.after.load(Ordering::Relaxed);
    assert_eq!(after - before, 0, "the lane's work allocated {} time(s)", after - before);
}

/// And the same on a **real lane thread**.
///
/// The counters are per thread, so this measures the lane's thread and
/// not the control thread's: the two are different numbers, and only
/// the lane's matters.
#[test]
fn the_lanes_work_does_not_allocate_on_the_lane_thread() {
    let measured = Arc::new(Measured::default());
    let lane =
        Lane::new(Box::new(MeasuringJob { measured: Arc::clone(&measured) }), 512).expect("the lane is made");
    let runner = LaneRunner::start(lane, 16);
    // Let the lane run a few blocks.
    std::thread::sleep(std::time::Duration::from_millis(50));
    let _job = runner.stop().expect("the lane is stopped");
    let blocks = measured.blocks.load(Ordering::Relaxed);
    assert!(blocks > 0, "the job ran: {blocks}");
    let before = measured.before.load(Ordering::Relaxed);
    let after = measured.after.load(Ordering::Relaxed);
    assert_eq!(after - before, 0, "the lane's work allocated {} time(s)", after - before);
}

/// The lane's thread is marked as a lane.
///
/// The mark is **per thread**: it is the runner's thread that is a lane,
/// and a block that runs on the control thread is not on one. That is
/// the whole point of the mark — work that must not be on a lane can ask
/// whether it is on one.
#[test]
fn the_lanes_thread_is_marked_as_a_lane() {
    let seen = Arc::new(AtomicU64::new(0));
    let marked = Arc::clone(&seen);
    let lane = Lane::new(Box::new(MarkedJob { seen: marked }), 512).expect("the lane is made");
    let runner = LaneRunner::start(lane, 16);
    std::thread::sleep(std::time::Duration::from_millis(50));
    let _job = runner.stop().expect("the lane is stopped");
    assert_eq!(seen.load(Ordering::Relaxed), 1, "the lane's work knows it is on a lane");
}

/// A job that asks whether it is on a lane.
struct MarkedJob {
    seen: Arc<AtomicU64>,
}

impl LaneJob for MarkedJob {
    fn prepare(&mut self, _block: usize) -> Result<(), String> {
        Ok(())
    }

    fn run(&mut self, _frames: usize) -> Block {
        let on_lane = u64::from(craftkit_sdk::realtime::on_lane());
        self.seen.store(on_lane, Ordering::Relaxed);
        Block::Played
    }
}
