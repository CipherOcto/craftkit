//! The **job pool**: work that may take a while.
//!
//! # Why this is not the lane
//!
//! The real-time lane (§ the SDK's `realtime`) is for work that must
//! **finish**: a block of audio, a frame of video. That work may not
//! allocate, may not wait, and may not stop.
//!
//! A **job** is the other thing: work that may take seconds, may
//! allocate as much as it likes, may fail, and may be **cancelled**.
//! Rendering a page of a document is a job. Reading a file is a job.
//! Building a preview is a job. None of them may be done on the lane,
//! and none of them may be done on the thread that draws.
//!
//! The port of a PDF application found this: it has a **render pool** of
//! worker threads with a priority queue, a staleness tag and an inline
//! fallback, because a page of a document takes far too long to draw in
//! the middle of a frame.
//!
//! # What a pool is
//!
//! - a **queue** of jobs, each with a **priority** and a **cancellation
//!   flag**;
//! - **workers** that take the most urgent job and run it;
//! - **results** that the caller collects when it is ready, so that the
//!   caller is never blocked and never has to poll a channel;
//! - **counters**: how many are queued, how many are running, how many
//!   finished, how many failed, how many were cancelled.
//!
//! # What a job is
//!
//! A job is a **name** and a **body**. The body may allocate, may block
//! and may fail, and it is handed the job's **cancellation flag**, so a
//! long job can stop itself when the thing it was doing is no longer
//! wanted. The result is **bytes**, because the results are big: a
//! raster, an encoded file, a chunk of text.
//!
//! # The rules
//!
//! - a job that fails **does not stop the pool**: the failure is a
//!   result, and the worker goes back to the queue;
//! - a job that is cancelled before it starts **does not run at all**;
//! - a job that is cancelled while it runs finds out through its flag,
//!   and the pool cannot take it away — that is what the flag is for;
//! - the pool **does not block the caller**: the caller queues work and
//!   collects results when it is ready.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// The id of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct JobId(pub u64);

/// The flag a job is handed, so that it can stop itself.
#[derive(Debug, Clone, Default)]
pub struct Cancel {
    flag: Arc<AtomicBool>,
}

impl Cancel {
    /// A flag that is not set.
    pub fn new() -> Self {
        Cancel::default()
    }

    /// Whether the job has been cancelled.
    pub fn cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    /// Cancel the job.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }
}

/// How far a job has got.
///
/// A long job that says nothing is a long job that a person watches as a
/// frozen window. The job sets the progress and the caller reads it, and
/// neither of them has to ask the other.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    percent: Arc<AtomicU64>,
}

impl Progress {
    /// A progress that is at nothing.
    pub fn new() -> Self {
        Progress::default()
    }

    /// Say how far the job has got, as a percentage.
    ///
    /// A number over a hundred is a hundred: a job that goes past the end
    /// is a job that finished.
    pub fn set(&self, percent: u32) {
        let percent = u64::from(percent.min(100));
        self.percent.store(percent, Ordering::Relaxed);
    }

    /// How far the job has got, as a percentage.
    pub fn percent(&self) -> u32 {
        self.percent.load(Ordering::Relaxed) as u32
    }
}

/// A piece of work that may take a while.
pub trait Job: Send {
    /// What the job is, for the report.
    fn name(&self) -> &str;

    /// Do the work.
    ///
    /// The bytes are the result. The flag is the job's own: a long job
    /// checks it now and then, and stops when it is set. The progress is
    /// how far it has got, for whoever is waiting.
    fn run(self: Box<Self>, cancel: &Cancel, progress: &Progress) -> Result<Vec<u8>, String>;
}

/// What came of a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobResult {
    /// Which job it was.
    pub id: JobId,
    /// What the job was called.
    pub name: String,
    /// What came of it.
    pub outcome: Outcome,
}

impl Outcome {
    /// The bytes, if the job finished.
    pub fn outcome_bytes(&self) -> Vec<u8> {
        match self {
            Outcome::Done(bytes) => bytes.clone(),
            Outcome::Failed(_) => Vec::new(),
            Outcome::Cancelled => Vec::new(),
        }
    }
}

/// What came of a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// It finished, and here is what it made.
    Done(Vec<u8>),
    /// It failed, and here is why.
    Failed(String),
    /// It was cancelled before it ran.
    Cancelled,
}

/// How the pool is doing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// How many are waiting.
    pub queued: usize,
    /// How many are running.
    pub running: usize,
    /// How many finished.
    pub done: u64,
    /// How many failed.
    pub failed: u64,
    /// How many were cancelled.
    pub cancelled: u64,
}

/// A job waiting in the queue.
struct Queued {
    id: JobId,
    priority: u8,
    name: String,
    job: Option<Box<dyn Job>>,
    cancel: Cancel,
}

/// What the workers share.
#[derive(Default)]
struct Inner {
    queue: Mutex<VecDeque<Queued>>,
    wake: Condvar,
    results: Mutex<Vec<JobResult>>,
    /// The cancellation flag of every job that is queued or running.
    cancels: Mutex<HashMap<u64, Cancel>>,
    /// The progress of every job that is queued or running.
    progress: Mutex<HashMap<u64, Progress>>,
    stop: AtomicBool,
    next_id: AtomicU64,
    running: AtomicU64,
    done: AtomicU64,
    failed: AtomicU64,
    cancelled: AtomicU64,
}

/// A pool of workers that run jobs.
///
/// The pool is cheap to make and cheap to drop: dropping it stops the
/// workers, and the work in flight finishes first.
#[derive(Clone)]
pub struct JobPool {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for JobPool {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("JobPool").field("stats", &self.stats()).finish()
    }
}

impl JobPool {
    /// A pool with this many workers.
    ///
    /// Zero workers is a pool that queues work and never runs it, which
    /// is a thing a program may want on purpose — a program that does
    /// its work on its own thread and uses the pool only as a queue.
    pub fn new(workers: usize) -> Self {
        let inner = Arc::new(Inner::default());
        for index in 0..workers {
            let worker_inner = Arc::clone(&inner);
            let _ = std::thread::Builder::new()
                .name(format!("craftkit-job-{index}"))
                .spawn(move || worker(worker_inner));
        }
        JobPool { inner }
    }

    /// Queue a job.
    ///
    /// The priority is a number, and a **bigger** number is more urgent.
    /// A job with the same priority as another runs in the order it was
    /// queued.
    pub fn spawn(&self, job: Box<dyn Job>, priority: u8) -> JobId {
        let id = JobId(self.inner.next_id.fetch_add(1, Ordering::Relaxed));
        let name = job.name().to_string();
        let cancel = Cancel::new();
        let progress = Progress::new();
        let queued = Queued { id, priority, name, job: Some(job), cancel: cancel.clone() };
        {
            let mut cancels = self.inner.cancels.lock().unwrap_or_else(|error| error.into_inner());
            cancels.insert(id.0, cancel);
            let mut progresses = self.inner.progress.lock().unwrap_or_else(|error| error.into_inner());
            progresses.insert(id.0, progress.clone());
        }
        let mut queue = self.inner.queue.lock().unwrap_or_else(|error| error.into_inner());
        // The most urgent job goes first, and the queue stays in the
        // order the jobs were queued in among equals.
        let at = queue.iter().position(|waiting| waiting.priority < priority).unwrap_or(queue.len());
        queue.insert(at, queued);
        drop(queue);
        self.inner.wake.notify_one();
        id
    }

    /// Cancel a job.
    ///
    /// A job that has not started **does not run at all**. A job that is
    /// running finds out through its flag, and the pool cannot take it
    /// away: that is what the flag is for.
    ///
    /// It says whether the job was still known — queued or running.
    pub fn cancel(&self, id: JobId) -> bool {
        // The flag first: a job that is running must find out.
        let flag = {
            let cancels = self.inner.cancels.lock().unwrap_or_else(|error| error.into_inner());
            match cancels.get(&id.0) {
                Some(flag) => {
                    flag.cancel();
                    Some(flag.clone())
                }
                None => None,
            }
        };
        let Some(_flag) = flag else { return false };
        // Then, if it is still waiting, it does not run at all.
        let mut queue = self.inner.queue.lock().unwrap_or_else(|error| error.into_inner());
        let Some(position) = queue.iter().position(|waiting| waiting.id == id) else {
            return true;
        };
        let Some(waiting) = queue.remove(position) else {
            return true;
        };
        drop(queue);
        let mut results = self.inner.results.lock().unwrap_or_else(|error| error.into_inner());
        results.push(JobResult { id, name: waiting.name, outcome: Outcome::Cancelled });
        self.inner.cancelled.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Take the results that have arrived.
    ///
    /// The caller is never blocked: it takes what is there and comes
    /// back later for the rest.
    pub fn drain(&self) -> Vec<JobResult> {
        let mut results = self.inner.results.lock().unwrap_or_else(|error| error.into_inner());
        std::mem::take(&mut *results)
    }

    /// The progress of a job, if it is still known.
    ///
    /// A job that has finished is not in here: it is in the results.
    pub fn progress(&self, id: JobId) -> Option<Progress> {
        let progress = self.inner.progress.lock().unwrap_or_else(|error| error.into_inner());
        progress.get(&id.0).cloned()
    }

    /// How the pool is doing.
    pub fn stats(&self) -> Stats {
        let queue = self.inner.queue.lock().unwrap_or_else(|error| error.into_inner());
        Stats {
            queued: queue.len(),
            running: self.inner.running.load(Ordering::Relaxed) as usize,
            done: self.inner.done.load(Ordering::Relaxed),
            failed: self.inner.failed.load(Ordering::Relaxed),
            cancelled: self.inner.cancelled.load(Ordering::Relaxed),
        }
    }

    /// Stop the pool.
    ///
    /// The workers finish what they are doing and stop. The jobs that are
    /// still waiting are dropped.
    pub fn shutdown(&self) {
        self.inner.stop.store(true, Ordering::Release);
        self.inner.wake.notify_all();
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.wake.notify_all();
    }
}

/// A worker: take the most urgent job and run it.
fn worker(inner: Arc<Inner>) {
    loop {
        let job = {
            let mut queue = inner.queue.lock().unwrap_or_else(|error| error.into_inner());
            loop {
                if inner.stop.load(Ordering::Acquire) {
                    return;
                }
                if let Some(waiting) = queue.pop_front() {
                    break waiting;
                }
                let (guard, timeout) = inner
                    .wake
                    .wait_timeout(queue, std::time::Duration::from_millis(50))
                    .unwrap_or_else(|error| error.into_inner());
                queue = guard;
                if timeout.timed_out() && inner.stop.load(Ordering::Acquire) {
                    return;
                }
            }
        };
        // A job that was cancelled while it waited does not run.
        if job.cancel.cancelled() {
            inner.cancelled.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        let Some(body) = job.job else { continue };
        inner.running.fetch_add(1, Ordering::Relaxed);
        let progress = {
            let progresses = inner.progress.lock().unwrap_or_else(|error| error.into_inner());
            progresses.get(&job.id.0).cloned().unwrap_or_default()
        };
        let outcome = match body.run(&job.cancel, &progress) {
            Ok(bytes) => {
                inner.done.fetch_add(1, Ordering::Relaxed);
                Outcome::Done(bytes)
            }
            Err(message) => {
                inner.failed.fetch_add(1, Ordering::Relaxed);
                Outcome::Failed(message)
            }
        };
        inner.running.fetch_sub(1, Ordering::Relaxed);
        {
            let mut cancels = inner.cancels.lock().unwrap_or_else(|error| error.into_inner());
            cancels.remove(&job.id.0);
            let mut progresses = inner.progress.lock().unwrap_or_else(|error| error.into_inner());
            progresses.remove(&job.id.0);
        }
        let mut results = inner.results.lock().unwrap_or_else(|error| error.into_inner());
        results.push(JobResult { id: job.id, name: job.name, outcome });
    }
}

/// The body of a job made of a closure.
type JobBody = Box<dyn FnOnce(&Cancel, &Progress) -> Result<Vec<u8>, String> + Send>;

/// A job made of a closure.
///
/// The common case: a name and something to do.
pub struct FnJob {
    name: String,
    body: JobBody,
}

impl FnJob {
    /// A job with a name and a body.
    pub fn new(
        name: impl Into<String>,
        body: impl FnOnce(&Cancel, &Progress) -> Result<Vec<u8>, String> + Send + 'static,
    ) -> Self {
        FnJob { name: name.into(), body: Box::new(body) }
    }
}

impl Job for FnJob {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(self: Box<Self>, cancel: &Cancel, progress: &Progress) -> Result<Vec<u8>, String> {
        (self.body)(cancel, progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::AtomicUsize;

    /// Wait for a condition, for a while.
    fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
        for _ in 0..100_000 {
            if done() {
                return;
            }
            std::thread::yield_now();
        }
        panic!("{what} did not happen");
    }

    /// Take the results until there are at least this many.
    fn collect(pool: &JobPool, wanted: usize) -> Vec<JobResult> {
        let mut all = Vec::new();
        for _ in 0..100_000 {
            all.extend(pool.drain());
            if all.len() >= wanted {
                return all;
            }
            std::thread::yield_now();
        }
        panic!("{wanted} result(s) did not come");
    }

    /// A job runs and its result comes back.
    #[test]
    fn a_job_runs_and_its_result_comes_back() {
        let pool = JobPool::new(2);
        pool.spawn(Box::new(FnJob::new("answer", |_cancel, _progress| Ok(b"forty two".to_vec()))), 0);
        let results = collect(&pool, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "answer");
        assert_eq!(results[0].outcome, Outcome::Done(b"forty two".to_vec()));
    }

    /// A job that fails does not stop the pool.
    #[test]
    fn a_job_that_fails_does_not_stop_the_pool() {
        let pool = JobPool::new(1);
        pool.spawn(Box::new(FnJob::new("bad", |_cancel, _progress| Err("it did not work".to_string()))), 0);
        pool.spawn(Box::new(FnJob::new("good", |_cancel, _progress| Ok(b"fine".to_vec()))), 0);
        let results = collect(&pool, 2);
        assert!(
            results.iter().any(|result| result.outcome == Outcome::Failed("it did not work".to_string()))
        );
        assert!(results.iter().any(|result| result.outcome == Outcome::Done(b"fine".to_vec())));
    }

    /// A job that is cancelled before it runs does not run.
    #[test]
    fn a_cancelled_job_does_not_run() {
        // No workers: nothing runs, so the job is still waiting.
        let pool = JobPool::new(0);
        let ran = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&ran);
        let id = pool.spawn(
            Box::new(FnJob::new("never", move |_cancel, _progress| {
                counter.fetch_add(1, Ordering::Relaxed);
                Ok(Vec::new())
            })),
            0,
        );
        assert!(pool.cancel(id), "the job was waiting");
        let results = pool.drain();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, Outcome::Cancelled);
        assert_eq!(ran.load(Ordering::Relaxed), 0, "the job did not run");
    }

    /// The most urgent job runs first.
    #[test]
    fn the_most_urgent_job_runs_first() {
        let pool = JobPool::new(1);
        // One worker, and a job that holds it until it is let go.
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let job_gate = Arc::clone(&gate);
        pool.spawn(
            Box::new(FnJob::new("gate", move |_cancel, _progress| {
                let (lock, wake) = &*job_gate;
                let mut open = lock.lock().unwrap_or_else(|error| error.into_inner());
                while !*open {
                    open = wake.wait(open).unwrap_or_else(|error| error.into_inner());
                }
                Ok(Vec::new())
            })),
            0,
        );
        // Wait for the worker to be busy, so that the two below are both
        // waiting in the queue.
        wait_for("the worker to start", || pool.stats().running == 1);
        let order = Arc::new(Mutex::new(Vec::new()));
        let low_order = Arc::clone(&order);
        let high_order = Arc::clone(&order);
        pool.spawn(
            Box::new(FnJob::new("low", move |_cancel, _progress| {
                low_order.lock().unwrap_or_else(|error| error.into_inner()).push("low");
                Ok(Vec::new())
            })),
            1,
        );
        pool.spawn(
            Box::new(FnJob::new("high", move |_cancel, _progress| {
                high_order.lock().unwrap_or_else(|error| error.into_inner()).push("high");
                Ok(Vec::new())
            })),
            9,
        );
        // Let the gate go.
        let (lock, wake) = &*gate;
        *lock.lock().unwrap_or_else(|error| error.into_inner()) = true;
        wake.notify_all();
        wait_for("both jobs", || order.lock().unwrap_or_else(|error| error.into_inner()).len() >= 2);
        let order = order.lock().unwrap_or_else(|error| error.into_inner()).clone();
        assert_eq!(order, vec!["high", "low"], "the urgent job ran first: {order:?}");
    }

    /// A job can see that it has been cancelled.
    #[test]
    fn a_job_can_see_that_it_is_cancelled() {
        let pool = JobPool::new(1);
        let seen = Arc::new(AtomicUsize::new(0));
        let job_seen = Arc::clone(&seen);
        pool.spawn(
            Box::new(FnJob::new("watcher", move |cancel, _progress| {
                while !cancel.cancelled() {
                    std::thread::yield_now();
                }
                job_seen.fetch_add(1, Ordering::Relaxed);
                Ok(Vec::new())
            })),
            0,
        );
        wait_for("the job to start", || pool.stats().running == 1);
        // The job is running, so cancelling it does not take it away —
        // but it finds out through its flag.
        let _ = pool.cancel(JobId(0));
        wait_for("the job to see it", || seen.load(Ordering::Relaxed) == 1);
    }

    /// A job says how far it has got.
    #[test]
    fn a_job_says_how_far_it_has_got() {
        let pool = JobPool::new(1);
        let id = pool.spawn(
            Box::new(FnJob::new("slow", |_cancel, progress| {
                progress.set(40);
                std::thread::sleep(std::time::Duration::from_millis(20));
                progress.set(100);
                Ok(Vec::new())
            })),
            0,
        );
        wait_for("the job to start", || pool.stats().running == 1);
        // The job is running, but it may not have said anything yet: wait
        // for the progress itself, not for the job to be picked up.
        wait_for("the job to say something", || {
            pool.progress(id).map(|progress| progress.percent() > 0).unwrap_or(false)
        });
        let progress = pool.progress(id).expect("the job is known");
        assert!(progress.percent() > 0, "the job said something: {}", progress.percent());
        collect(&pool, 1);
    }

    /// The pool counts what it has done.
    #[test]
    fn the_pool_counts_what_it_has_done() {
        let pool = JobPool::new(2);
        pool.spawn(Box::new(FnJob::new("a", |_cancel, _progress| Ok(Vec::new()))), 0);
        pool.spawn(Box::new(FnJob::new("b", |_cancel, _progress| Err("no".to_string()))), 0);
        wait_for("both jobs", || {
            let stats = pool.stats();
            stats.done + stats.failed >= 2
        });
        let stats = pool.stats();
        assert_eq!(stats.done, 1);
        assert_eq!(stats.failed, 1);
    }
}
