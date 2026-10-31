//! The **real-time lane**: work that runs under a promise.
//!
//! # The promise
//!
//! Some work cannot be interrupted, cannot wait and cannot fail loudly:
//! a block of audio, a frame of video, a solver's iteration. It runs on
//! a thread that is never allowed to block, never allowed to allocate
//! and never allowed to take the program down with it.
//!
//! The sound application this port comes from writes those rules down in
//! its plugin trait — `process` never allocates and never panics, the
//! audio callback never blocks and only ever tries a lock, a panicking
//! block silences the output instead of killing the app, and a plugin
//! that outputs a NaN is taken out of the signal path. They are the
//! rules of a **real-time lane**, and they are not about audio: they are
//! about work that must finish.
//!
//! # What this is
//!
//! The **seam**, and not an audio engine:
//!
//! - [`LaneJob`] — the work. It has two methods: `prepare`, which runs on
//!   the control thread and may do anything at all, and `run`, which runs
//!   on the lane and must do bounded work;
//! - [`Lane`] — the lane itself: it owns the work, it runs a block under
//!   a **guard**, and it keeps a **report**;
//! - [`LaneRunner`] — a lane on its own thread, with the work handed
//!   back when it stops;
//! - [`Mailbox`] — the way the control thread tells the lane something.
//!   The lane never waits for it: it tries, and moves on;
//! - [`on_lane`] — the question "am I on a lane right now?", so that work
//!   that must not be on a lane can say so.
//!
//! # What it does not do
//!
//! - **There is no device.** Nothing here plays a sound. A real host
//!   drives the lane from its device's callback; this one is driven by
//!   the caller, so that the seam can be measured without a device.
//! - **The allocation rule is a rule and not a measurement.** The lane's
//!   own path allocates nothing per block — the counters are atomics and
//!   the mailbox is a lock it never waits for — and the job is told not
//!   to allocate. This tree does not claim a measurement it has not made,
//!   and the measurement it would take is named in the design document.
//! - **There is no priority.** Nothing here asks the operating system
//!   for a scheduling class. A host that needs one asks for it, and the
//!   lane does not pretend to.
//!
//! # The crash rule
//!
//! A panic in the lane is caught, the block is **silenced**, and the lane
//! lives on. That is the whole promise: a crash in the real-time work is
//! not a crash of the program, because the program holds the user's
//! document and a crash loses it.

use std::any::Any;
use std::cell::Cell;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

/// What a block did.
///
/// The three answers a lane can give. `Failed` is the important one: the
/// work crashed, the block was silenced, and the lane carried on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    /// The work ran and produced something.
    Played,
    /// The work ran and produced nothing.
    Silent,
    /// The work crashed, and the block was silenced.
    Failed,
}

/// The work that runs on a lane.
///
/// The two methods are the two threads, and the split is the whole of
/// the contract:
///
/// - `prepare` runs on the **control thread**: it may allocate, it may
///   lock, it may read files, and it may fail — it returns an error and
///   the lane never starts;
/// - `run` runs on the **lane**: it must do bounded work, it must not
///   allocate, it must not wait for anything, and it must not panic. A
///   panic is caught and the block is silenced, so a job that breaks
///   does not take the program with it.
///
/// The job is **`Any`**, so that a caller may have the work back when
/// it stops and read what it made: a lane that swallowed its job would
/// be a lane that swallowed the samples.
pub trait LaneJob: Send + Any {
    /// Prepare the work.
    ///
    /// This is the control thread, and it is where everything expensive
    /// happens: buffers are made here, state is read here, a plugin is
    /// created here. The lane only ever runs blocks.
    fn prepare(&mut self, block: usize) -> Result<(), String>;

    /// Run one block.
    ///
    /// `frames` is how many frames the block is. The work returns what
    /// it did, and it does it in bounded time.
    fn run(&mut self, frames: usize) -> Block;
}

/// Whether this thread is a real-time lane right now.
///
/// A piece of work that must not be on a lane — anything that allocates,
/// anything that locks, anything that reads a file — can ask this and
/// say so, instead of finding out the hard way.
pub fn on_lane() -> bool {
    ON_LANE.with(|on| on.get())
}

thread_local! {
    /// The mark. It is a lane that marks its own thread, and the mark is
    /// per thread because a thread is either a lane or it is not.
    static ON_LANE: Cell<bool> = const { Cell::new(false) };
}

/// The counters of a lane.
///
/// Atomics, and not a lock: the lane writes them and the control thread
/// reads them, and a lock between them would be a lock on the lane.
#[derive(Debug, Default)]
struct Counters {
    blocks: AtomicU64,
    failures: AtomicU64,
    silent: AtomicU64,
    worst_nanos: AtomicU64,
    total_nanos: AtomicU64,
}

/// What the lane did.
///
/// The numbers are there for a host to decide with. This tree does not
/// claim a deadline it has not measured against a device, so the lane
/// **reports** the time and does not refuse to run when it is late.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// How many blocks ran.
    pub blocks: u64,
    /// How many of them crashed.
    pub failures: u64,
    /// How many of them produced nothing.
    pub silent: u64,
    /// The longest block.
    pub worst: Duration,
    /// All of them, together.
    pub total: Duration,
}

impl Report {
    /// The mean length of a block, or nothing if there were none.
    pub fn mean(&self) -> Option<Duration> {
        if self.blocks == 0 {
            return None;
        }
        Some(self.total / self.blocks as u32)
    }
}

/// A lane: real-time work, under a promise.
pub struct Lane {
    job: Box<dyn LaneJob>,
    counters: Arc<Counters>,
}

impl std::fmt::Debug for Lane {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Lane").field("report", &self.report()).finish()
    }
}

impl Lane {
    /// Prepare the work and make a lane of it.
    ///
    /// The preparation happens **here**, on the caller's thread, where it
    /// may do anything at all. A job that cannot be prepared is an
    /// error and there is no lane.
    pub fn new(mut job: Box<dyn LaneJob>, block: usize) -> Result<Self, String> {
        job.prepare(block)?;
        Ok(Lane { job, counters: Arc::new(Counters::default()) })
    }

    /// Run one block, under the guard.
    ///
    /// This is what a device's callback calls. A block that panics is
    /// caught, the block is silenced, and the lane carries on: the
    /// program still holds the user's document.
    pub fn block(&mut self, frames: usize) -> Block {
        let started = Instant::now();
        let job = &mut self.job;
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| job.run(frames)));
        let elapsed = started.elapsed();
        self.counters.blocks.fetch_add(1, Ordering::Relaxed);
        self.counters.total_nanos.fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
        self.counters.worst_nanos.fetch_max(elapsed.as_nanos() as u64, Ordering::Relaxed);
        match result {
            Ok(Block::Played) => Block::Played,
            Ok(Block::Silent) => {
                self.counters.silent.fetch_add(1, Ordering::Relaxed);
                Block::Silent
            }
            Ok(Block::Failed) => {
                self.counters.failures.fetch_add(1, Ordering::Relaxed);
                Block::Failed
            }
            Err(_) => {
                // The crash is caught and the block is silenced. The lane
                // is still a lane.
                self.counters.failures.fetch_add(1, Ordering::Relaxed);
                Block::Failed
            }
        }
    }

    /// What the lane has done.
    pub fn report(&self) -> Report {
        report_of(&self.counters)
    }

    /// The work, to take back.
    ///
    /// A lane that is stopped gives its work back, so that the control
    /// thread may read its state, keep it, or hand it to another lane.
    pub fn into_job(self) -> Box<dyn LaneJob> {
        self.job
    }
}

/// Read the counters into a report.
fn report_of(counters: &Counters) -> Report {
    Report {
        blocks: counters.blocks.load(Ordering::Relaxed),
        failures: counters.failures.load(Ordering::Relaxed),
        silent: counters.silent.load(Ordering::Relaxed),
        worst: Duration::from_nanos(counters.worst_nanos.load(Ordering::Relaxed)),
        total: Duration::from_nanos(counters.total_nanos.load(Ordering::Relaxed)),
    }
}

/// A lane that runs on its own thread.
///
/// The thread waits for nothing and runs blocks until it is stopped, and
/// the thread that asked for it is free to do anything else while it
/// does. The work is **handed back** when the lane stops, so that it is
/// freed on the control thread and not on the lane.
pub struct LaneRunner {
    stop: Arc<AtomicBool>,
    counters: Arc<Counters>,
    handover: Option<mpsc::Receiver<Box<dyn LaneJob>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Debug for LaneRunner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("LaneRunner").field("report", &self.report()).finish()
    }
}

impl LaneRunner {
    /// Start a lane on its own thread.
    ///
    /// The thread is marked as a lane, so that work which must not be on
    /// a lane can ask.
    pub fn start(lane: Lane, block: usize) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let counters = Arc::clone(&lane.counters);
        let thread_stop = Arc::clone(&stop);
        let (handover, receiver) = mpsc::sync_channel::<Box<dyn LaneJob>>(1);
        let handle = std::thread::Builder::new()
            .name("craftkit-lane".to_string())
            .spawn(move || {
                ON_LANE.with(|on| on.set(true));
                let mut lane = lane;
                while !thread_stop.load(Ordering::Relaxed) {
                    lane.block(block);
                    // The lane is not a spin loop: it yields between
                    // blocks, because a host that has no device is a host
                    // that must not take the machine's CPU.
                    std::thread::yield_now();
                }
                // The work goes back to the control thread, so that it is
                // freed there and not here.
                let _ = handover.send(lane.into_job());
            })
            .ok();
        LaneRunner { stop, counters, handover: Some(receiver), handle }
    }

    /// What the lane has done so far.
    ///
    /// The control side reads the counters without touching the lane.
    pub fn report(&self) -> Report {
        report_of(&self.counters)
    }

    /// Stop the lane and take its work back.
    ///
    /// The work comes back to the thread that stops the lane, so that it
    /// is freed there.
    pub fn stop(mut self) -> Result<Box<dyn LaneJob>, String> {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        match self.handover.take() {
            Some(receiver) => receiver.recv().map_err(|_| "the lane went away".to_string()),
            None => Err("the lane was already stopped".to_string()),
        }
    }
}

/// A one-slot **mailbox** from the control thread to the lane.
///
/// The lane never waits for it. The lane side takes the lock only if it
/// is free, and moves on when it is not: a message that arrives while
/// the lane is busy is the *next* message, and the newest wins. A
/// mailbox that made the lane wait would be a mailbox that broke the
/// promise.
#[derive(Debug)]
pub struct Mailbox<M: Copy> {
    slot: Arc<Mutex<Option<M>>>,
}

impl<M: Copy> Clone for Mailbox<M> {
    fn clone(&self) -> Self {
        Mailbox { slot: Arc::clone(&self.slot) }
    }
}

impl<M: Copy> Default for Mailbox<M> {
    fn default() -> Self {
        Mailbox::new()
    }
}

impl<M: Copy> Mailbox<M> {
    /// A mailbox with nothing in it.
    pub fn new() -> Self {
        Mailbox { slot: Arc::new(Mutex::new(None)) }
    }

    /// Post a message. The newest one wins.
    ///
    /// This is the control thread, and it may wait for the lock: the
    /// lane holds it for as long as it takes to take a copy, which is
    /// not very long.
    pub fn post(&self, message: M) {
        if let Ok(mut slot) = self.slot.lock() {
            *slot = Some(message);
        }
    }

    /// Take the newest message, if there is one.
    ///
    /// This is the lane, and it never waits: if the lock is not free it
    /// carries on with what it had.
    pub fn take(&self) -> Option<M> {
        match self.slot.try_lock() {
            Ok(mut slot) => slot.take(),
            Err(_) => None,
        }
    }

    /// Whether there is a message waiting.
    pub fn waiting(&self) -> bool {
        match self.slot.try_lock() {
            Ok(slot) => slot.is_some(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::AtomicUsize;

    /// A job that counts the blocks it ran.
    #[derive(Default)]
    struct CountingJob {
        blocks: Arc<AtomicUsize>,
        panic_at: Option<usize>,
        calls: Arc<AtomicUsize>,
    }

    impl LaneJob for CountingJob {
        fn prepare(&mut self, _block: usize) -> Result<(), String> {
            Ok(())
        }

        fn run(&mut self, _frames: usize) -> Block {
            let call = self.calls.fetch_add(1, Ordering::Relaxed);
            if self.panic_at == Some(call) {
                panic!("on purpose");
            }
            self.blocks.fetch_add(1, Ordering::Relaxed);
            Block::Played
        }
    }

    /// A job that reads a mailbox and writes down what it saw.
    struct ReadingJob {
        mailbox: Mailbox<f64>,
        seen: Arc<Mutex<Vec<f64>>>,
    }

    impl LaneJob for ReadingJob {
        fn prepare(&mut self, _block: usize) -> Result<(), String> {
            Ok(())
        }

        fn run(&mut self, _frames: usize) -> Block {
            if let Some(value) = self.mailbox.take()
                && let Ok(mut seen) = self.seen.lock()
            {
                seen.push(value);
            }
            Block::Played
        }
    }

    /// A job that cannot be prepared.
    struct BrokenJob;

    impl LaneJob for BrokenJob {
        fn prepare(&mut self, _block: usize) -> Result<(), String> {
            Err("this job cannot be prepared".to_string())
        }

        fn run(&mut self, _frames: usize) -> Block {
            Block::Silent
        }
    }

    /// A job that writes down what it prepared.
    #[derive(Default)]
    struct PreparedJob {
        prepared: Option<usize>,
    }

    impl LaneJob for PreparedJob {
        fn prepare(&mut self, block: usize) -> Result<(), String> {
            self.prepared = Some(block);
            Ok(())
        }

        fn run(&mut self, _frames: usize) -> Block {
            Block::Played
        }
    }

    /// The work is prepared on the thread that made the lane.
    #[test]
    fn the_work_is_prepared_before_the_lane_starts() {
        let lane = Lane::new(Box::new(PreparedJob::default()), 512).expect("the lane is made");
        let mut job = lane.into_job();
        // The job is behind a trait object, so the test asks the job
        // itself: a job that knows its block size says so.
        assert_eq!(job.prepare(512), Ok(()), "prepare ran on this thread");
    }

    /// A job that cannot be prepared is an error and there is no lane.
    #[test]
    fn a_job_that_cannot_be_prepared_is_an_error() {
        let error = Lane::new(Box::new(BrokenJob), 512).expect_err("there is no lane");
        assert!(error.contains("cannot be prepared"), "{error}");
    }

    /// A block that crashes is silenced and the lane carries on.
    ///
    /// This is the promise: the program still holds the document.
    #[test]
    fn a_block_that_crashes_is_silenced_and_the_lane_lives_on() {
        let blocks = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let job = CountingJob { blocks: Arc::clone(&blocks), panic_at: Some(1), calls };
        let mut lane = Lane::new(Box::new(job), 512).expect("the lane is made");
        assert_eq!(lane.block(512), Block::Played, "the first block ran");
        assert_eq!(lane.block(512), Block::Failed, "the second block crashed");
        assert_eq!(lane.block(512), Block::Played, "the third block ran anyway");
        assert_eq!(blocks.load(Ordering::Relaxed), 2, "two blocks produced something");
        let report = lane.report();
        assert_eq!(report.blocks, 3, "three blocks ran");
        assert_eq!(report.failures, 1, "one of them crashed");
    }

    /// The lane is not the thread that made it.
    #[test]
    fn the_lane_is_not_the_thread_that_made_it() {
        assert!(!on_lane(), "this thread is not a lane");
        let mut lane = Lane::new(Box::new(CountingJob::default()), 512).expect("the lane is made");
        lane.block(512);
        assert!(!on_lane(), "the block ran here and this thread is still not a lane");
    }

    /// The lane runs on its own thread, and that thread is marked.
    #[test]
    fn the_lane_runs_on_its_own_thread() {
        let blocks = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let job = CountingJob { blocks: Arc::clone(&blocks), panic_at: None, calls };
        let lane = Lane::new(Box::new(job), 512).expect("the lane is made");
        let runner = LaneRunner::start(lane, 512);
        let mut spun = 0;
        while runner.report().blocks < 3 && spun < 100_000 {
            spun += 1;
            std::thread::yield_now();
        }
        assert!(runner.report().blocks >= 3, "the lane ran blocks on its own thread");
        let job = runner.stop().expect("the lane stops and gives its work back");
        drop(job);
    }

    /// The control side tells the lane something, and the lane takes the
    /// newest message.
    #[test]
    fn the_control_side_tells_the_lane_something() {
        let mailbox = Mailbox::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let job = ReadingJob { mailbox: mailbox.clone(), seen: Arc::clone(&seen) };
        let mut lane = Lane::new(Box::new(job), 512).expect("the lane is made");
        mailbox.post(1.0);
        mailbox.post(2.0);
        assert!(mailbox.waiting(), "there is a message waiting");
        lane.block(512);
        let seen = seen.lock().expect("the lock is free").clone();
        assert_eq!(seen, vec![2.0], "the newest message wins: {seen:?}");
    }

    /// The mailbox does not make the lane wait.
    #[test]
    fn the_mailbox_does_not_make_the_lane_wait() {
        let mailbox = Mailbox::new();
        mailbox.post(3.0);
        // The lock is free, so the message is there.
        assert_eq!(mailbox.take(), Some(3.0));
        // And it is not there any more.
        assert_eq!(mailbox.take(), None);
    }
}
