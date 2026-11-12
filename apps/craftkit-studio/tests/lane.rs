//! The **mix** does not allocate.
//!
//! The lane's rule is that its work must not allocate, and the work of
//! a sound application is the mixing: the tracks are summed into the
//! block, block after block, and an allocation inside it is a lock
//! inside it.
//!
//! This is the measurement of that rule on the **real** work — the
//! sound battery's mix job, on a **real lane thread** — and the tree
//! has no device, so a caller runs the blocks itself.

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

/// A job that measures its own allocations and then does the work.
///
/// The counters are **per thread**, so the numbers are the lane's
/// thread's numbers and nobody else's.
struct Measuring<T> {
    /// The work.
    inner: T,
    /// What it measured.
    measured: Arc<Measured>,
}

impl<T: LaneJob> LaneJob for Measuring<T> {
    fn prepare(&mut self, block: usize) -> Result<(), String> {
        self.inner.prepare(block)
    }

    fn run(&mut self, frames: usize) -> Block {
        let before = craftkit_alloc::counts().allocations;
        let block = self.inner.run(frames);
        let after = craftkit_alloc::counts().allocations;
        self.measured.before.store(before, Ordering::Relaxed);
        self.measured.after.store(after, Ordering::Relaxed);
        self.measured.blocks.fetch_add(1, Ordering::Relaxed);
        block
    }
}

/// The tracks of a small mix: one at the start and one later.
#[cfg(feature = "sound")]
fn tracks() -> Vec<craftkit_audio::Track> {
    vec![
        craftkit_audio::Track { start: 0, samples: vec![1.0, 1.0], gain: 1.0 },
        craftkit_audio::Track { start: 1, samples: vec![0.5, 0.5], gain: 1.0 },
    ]
}

/// The mix's work does not allocate on the **lane thread**.
///
/// The counters are per thread, so this measures the lane's thread and
/// not the control thread's: the two are different numbers, and only
/// the lane's matters.
#[cfg(feature = "sound")]
#[test]
fn the_mix_does_not_allocate_on_the_lane_thread() {
    let measured = Arc::new(Measured::default());
    let job =
        Measuring { inner: craftkit_plugin_sound::MixJob::new(tracks()), measured: Arc::clone(&measured) };
    let lane = Lane::new(Box::new(job), 512).expect("the lane is made");
    let runner = LaneRunner::start(lane, 16);
    // Let the lane run a few blocks.
    std::thread::sleep(std::time::Duration::from_millis(50));
    let _job = runner.stop().expect("the lane is stopped");
    let blocks = measured.blocks.load(Ordering::Relaxed);
    assert!(blocks > 0, "the job ran: {blocks}");
    let before = measured.before.load(Ordering::Relaxed);
    let after = measured.after.load(Ordering::Relaxed);
    assert_eq!(after - before, 0, "the mixing allocated {} time(s)", after - before);
}

/// The mix's work is the **sum of the tracks**.
///
/// The lane's block is the sum of the tracks at that point in the mix:
/// the first track is at the start and the second is two frames in.
#[cfg(feature = "sound")]
#[test]
fn the_mix_is_the_sum_of_the_tracks() {
    let job = craftkit_plugin_sound::MixJob::new(tracks());
    let mut lane = Lane::new(Box::new(job), 512).expect("the lane is made");
    lane.block(4);
    let job: Box<dyn std::any::Any> = lane.into_job();
    let job = job.downcast::<craftkit_plugin_sound::MixJob>().expect("the work is the mix's job");
    let samples = job.samples();
    assert_eq!(samples.len(), 512, "the block is the lane's block");
    assert!((samples[0] - 1.0).abs() < 1e-6, "the first track is there: {}", samples[0]);
    assert!((samples[1] - 1.5).abs() < 1e-6, "and the second is added to it: {}", samples[1]);
    assert!((samples[2] - 0.5).abs() < 1e-6, "and it goes on alone: {}", samples[2]);
    assert!((samples[4] - 0.0).abs() < 1e-6, "and nothing after the block");
}

/// The work is had **back** when the lane stops.
///
/// The lane that swallowed its job would be a lane that swallowed the
/// samples.
#[cfg(feature = "sound")]
#[test]
fn the_work_is_had_back_when_the_lane_stops() {
    let job = craftkit_plugin_sound::MixJob::new(tracks());
    let lane = Lane::new(Box::new(job), 512).expect("the lane is made");
    let runner = LaneRunner::start(lane, 16);
    std::thread::sleep(std::time::Duration::from_millis(50));
    let job: Box<dyn std::any::Any> = runner.stop().expect("the lane is stopped");
    let job = job.downcast::<craftkit_plugin_sound::MixJob>().expect("the work is the mix's job");
    assert!(job.played() > 0, "and it mixed something: {}", job.played());
}
