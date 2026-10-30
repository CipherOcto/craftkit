//! **Tasks**: work that may outlive the process.
//!
//! # The problem this solves
//!
//! The job pool (§21.1) is for work that runs **here**: a queue, workers,
//! a cancellation flag, and results the caller collects when the work is
//! done. Everything about it is in-process — the work starts when it is
//! spawned and ends when a worker finishes it.
//!
//! Not every job is like that. A generation is enqueued with a provider,
//! gets the provider's own id, is polled, is written to a database so
//! that it survives the application being closed, and is finished by an
//! **event that arrives from outside** — hours later, in a process that
//! has nothing to do with the one that asked for it.
//!
//! There is no way to express that with a pool: the pool owns its work,
//! and the work is over when the worker returns.
//!
//! # What a task is
//!
//! A **task** is work with a **durable handle**:
//!
//! - it is **registered** here, and it may be registered by a process
//!   that is not the one that finishes it;
//! - it has a **state** — pending, running, done, failed, cancelled — and
//!   the state moves in one direction;
//! - it may carry the **outside world's own id** — the provider's job
//!   id — so that the other end can be asked about it;
//! - it is **durable**: the registry is written down and read back, so
//!   the list of tasks survives the process that made it.
//!
//! # The rules
//!
//! - a task is finished by whoever gets there first, and it does not
//!   matter whether that is this process or another one;
//! - a task that is done, failed or cancelled is **finished**: nothing
//!   moves it again;
//! - a progress is a number between 0 and 100, and a number outside that
//!   is refused rather than clamped: the caller has said something wrong
//!   and it should hear about it;
//! - the registry is **bounded**: it holds a fixed number of tasks, and
//!   the oldest finished one goes when it is full. A registry that grows
//!   for ever is a leak.

use crate::serde_json::Value as Json;

/// How many tasks the registry holds.
///
/// A thousand is a lot of jobs, and it is a number rather than a rule.
pub const TASK_LIMIT: usize = 1024;

/// The state of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Registered, and not started.
    Pending,
    /// Started, and not finished.
    Running,
    /// Finished, with a result.
    Done,
    /// Finished, with a reason.
    Failed,
    /// Finished, because it was called off.
    Cancelled,
}

impl TaskState {
    /// The name of the state, as it is written.
    pub fn name(&self) -> &'static str {
        match self {
            TaskState::Pending => "pending",
            TaskState::Running => "running",
            TaskState::Done => "done",
            TaskState::Failed => "failed",
            TaskState::Cancelled => "cancelled",
        }
    }

    /// The state with that name, if there is one.
    pub fn of(name: &str) -> Option<TaskState> {
        match name {
            "pending" => Some(TaskState::Pending),
            "running" => Some(TaskState::Running),
            "done" => Some(TaskState::Done),
            "failed" => Some(TaskState::Failed),
            "cancelled" => Some(TaskState::Cancelled),
            _ => None,
        }
    }

    /// Whether the task is **finished**: it will not move again.
    pub fn is_finished(&self) -> bool {
        matches!(self, TaskState::Done | TaskState::Failed | TaskState::Cancelled)
    }
}

/// A task: work that may outlive the process.
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    /// The registry's own number for the task.
    pub id: u64,
    /// What kind of work it is: `pdf.render`, `artcraft.generate`, …
    pub kind: String,
    /// Where it is.
    pub state: TaskState,
    /// The outside world's own id, if it has one.
    ///
    /// A provider's job id: the thing to ask about it, and the thing
    /// that comes back when it is done.
    pub handle: Option<String>,
    /// How far it has got, as a percent.
    pub progress: u8,
    /// Why it failed, if it failed.
    pub message: Option<String>,
    /// What it produced, if it produced anything.
    pub result: Option<Json>,
}

impl Task {
    /// The task, as it is written.
    pub fn to_json(&self) -> Json {
        serde_json::json!({
            "id": self.id,
            "kind": self.kind,
            "state": self.state.name(),
            "handle": self.handle,
            "progress": self.progress,
            "message": self.message,
            "result": self.result,
        })
    }

    /// The task, as it is read.
    pub fn from_json(value: &Json) -> Result<Task, String> {
        let id = value.get("id").and_then(|id| id.as_u64()).ok_or_else(|| "a task has no id".to_string())?;
        let kind = value
            .get("kind")
            .and_then(|kind| kind.as_str())
            .ok_or_else(|| "a task has no kind".to_string())?
            .to_string();
        let state = value
            .get("state")
            .and_then(|state| state.as_str())
            .and_then(TaskState::of)
            .ok_or_else(|| format!("the task {id} has a state nobody knows"))?;
        let handle = value.get("handle").and_then(|handle| handle.as_str()).map(|h| h.to_string());
        let progress = value.get("progress").and_then(|progress| progress.as_u64()).unwrap_or(0);
        let progress = u8::try_from(progress.min(100))
            .map_err(|_| format!("the task {id} has a progress of {progress}"))?;
        let message = value.get("message").and_then(|message| message.as_str()).map(|m| m.to_string());
        let result = value.get("result").cloned().filter(|result| !result.is_null());
        Ok(Task { id, kind, state, handle, progress, message, result })
    }
}

/// The tasks a session knows about.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskRegistry {
    tasks: Vec<Task>,
    next_id: u64,
}

impl TaskRegistry {
    /// A registry with nothing in it.
    pub fn new() -> Self {
        TaskRegistry { tasks: Vec::new(), next_id: 1 }
    }

    /// Register a task, and give back its number.
    ///
    /// It is **pending**: registered, and not started.
    pub fn register(&mut self, kind: impl Into<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(Task {
            id,
            kind: kind.into(),
            state: TaskState::Pending,
            handle: None,
            progress: 0,
            message: None,
            result: None,
        });
        self.prune();
        id
    }

    /// The task with that number.
    pub fn task(&self, id: u64) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == id)
    }

    /// The task with that number, to change.
    pub fn task_mut(&mut self, id: u64) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|task| task.id == id)
    }

    /// The task with that **handle**: the outside world's own id for it.
    ///
    /// The handle is how the other end names the work: a provider's job
    /// comes back with its job id, and the only way to find the task is
    /// by that id. So the registry can be searched by it.
    ///
    /// The first task with the handle is found, and a handle is meant to
    /// be **unique**: it is the other end's name for one piece of work.
    pub fn find_by_handle(&self, handle: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.handle.as_deref() == Some(handle))
    }

    /// The task with that handle, to change.
    pub fn find_by_handle_mut(&mut self, handle: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|task| task.handle.as_deref() == Some(handle))
    }

    /// Finish the task with that **handle**, with what it produced.
    ///
    /// This is the door the outside world comes in by: the event that
    /// says the work is done names the work by the id the other end
    /// knows it by, and this is where that id is turned back into the
    /// task.
    pub fn complete_by_handle(&mut self, handle: &str, result: Json) -> Result<(), String> {
        let Some(task) = self.find_by_handle_mut(handle) else {
            return Err(format!("there is no task with the handle `{handle}`"));
        };
        if task.state.is_finished() {
            return Err(format!("the task {} is {}, and it is finished", task.id, task.state.name()));
        }
        task.state = TaskState::Done;
        task.progress = 100;
        task.result = Some(result);
        Ok(())
    }

    /// Finish the task with that **handle**, with the reason it did not
    /// finish.
    ///
    /// The other door the outside world comes in by.
    pub fn fail_by_handle(&mut self, handle: &str, message: impl Into<String>) -> Result<(), String> {
        let Some(task) = self.find_by_handle_mut(handle) else {
            return Err(format!("there is no task with the handle `{handle}`"));
        };
        if task.state.is_finished() {
            return Err(format!("the task {} is {}, and it is finished", task.id, task.state.name()));
        }
        task.state = TaskState::Failed;
        task.message = Some(message.into());
        Ok(())
    }

    /// Every task.
    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    /// The tasks in one state.
    pub fn with_state(&self, state: TaskState) -> Vec<&Task> {
        self.tasks.iter().filter(|task| task.state == state).collect()
    }

    /// Start a task.
    ///
    /// A task that is not pending has already moved, and moving it again
    /// is refused: the state of a task is a fact, not a wish.
    pub fn start(&mut self, id: u64) -> Result<(), String> {
        let task = self.task_mut(id).ok_or_else(|| format!("there is no task {id}"))?;
        if task.state != TaskState::Pending {
            return Err(format!("the task {id} is {}, and it is not pending", task.state.name()));
        }
        task.state = TaskState::Running;
        Ok(())
    }

    /// Give the task the outside world's own id.
    pub fn attach(&mut self, id: u64, handle: impl Into<String>) -> Result<(), String> {
        let task = self.task_mut(id).ok_or_else(|| format!("there is no task {id}"))?;
        if task.state.is_finished() {
            return Err(format!("the task {id} is {}, and it is finished", task.state.name()));
        }
        task.handle = Some(handle.into());
        Ok(())
    }

    /// Say how far the task has got.
    pub fn progress(&mut self, id: u64, percent: u8) -> Result<(), String> {
        let task = self.task_mut(id).ok_or_else(|| format!("there is no task {id}"))?;
        if task.state != TaskState::Running {
            return Err(format!("the task {id} is {}, and it is not running", task.state.name()));
        }
        task.progress = percent.min(100);
        Ok(())
    }

    /// Finish the task, with what it produced.
    ///
    /// **Whoever gets there first finishes it.** This process may not
    /// have started the task at all: the event that says it is done may
    /// come from a server, from another process, or from a timer.
    pub fn complete(&mut self, id: u64, result: Json) -> Result<(), String> {
        let task = self.task_mut(id).ok_or_else(|| format!("there is no task {id}"))?;
        if task.state.is_finished() {
            return Err(format!("the task {id} is {}, and it is finished", task.state.name()));
        }
        task.state = TaskState::Done;
        task.progress = 100;
        task.result = Some(result);
        Ok(())
    }

    /// Finish the task, with the reason it did not finish.
    pub fn fail(&mut self, id: u64, message: impl Into<String>) -> Result<(), String> {
        let task = self.task_mut(id).ok_or_else(|| format!("there is no task {id}"))?;
        if task.state.is_finished() {
            return Err(format!("the task {id} is {}, and it is finished", task.state.name()));
        }
        task.state = TaskState::Failed;
        task.message = Some(message.into());
        Ok(())
    }

    /// Call the task off.
    pub fn cancel(&mut self, id: u64) -> Result<(), String> {
        let task = self.task_mut(id).ok_or_else(|| format!("there is no task {id}"))?;
        if task.state.is_finished() {
            return Err(format!("the task {id} is {}, and it is finished", task.state.name()));
        }
        task.state = TaskState::Cancelled;
        Ok(())
    }

    /// The registry, as it is written.
    ///
    /// The tasks and nothing else: the registry is a list of tasks, and
    /// it is written down so that the list survives the process.
    pub fn to_json(&self) -> Json {
        serde_json::json!({ "tasks": self.tasks.iter().map(|task| task.to_json()).collect::<Vec<_>>() })
    }

    /// The registry, as it is read.
    pub fn from_json(value: &Json) -> Result<TaskRegistry, String> {
        let tasks = value.get("tasks").and_then(|tasks| tasks.as_array());
        let Some(tasks) = tasks else {
            return Err("there are no tasks in this".to_string());
        };
        let mut registry = TaskRegistry::new();
        for task in tasks {
            let task = Task::from_json(task)?;
            registry.next_id = registry.next_id.max(task.id + 1);
            registry.tasks.push(task);
        }
        Ok(registry)
    }

    /// Drop the oldest finished tasks when the registry is full.
    ///
    /// A task that is still pending or running is **not** dropped: it is
    /// work that has not been accounted for, and losing it would lose the
    /// handle that the other end knows it by.
    fn prune(&mut self) {
        while self.tasks.len() > TASK_LIMIT {
            let Some(victim) = self.tasks.iter().find(|task| task.state.is_finished()).map(|task| task.id)
            else {
                break;
            };
            self.tasks.retain(|task| task.id != victim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A registry with one task in it.
    fn registry() -> TaskRegistry {
        let mut registry = TaskRegistry::new();
        registry.register("pdf.render");
        registry
    }

    /// A task moves from pending to running to done.
    #[test]
    fn a_task_moves_from_pending_to_running_to_done() {
        let mut registry = registry();
        let id = registry.tasks()[0].id;
        assert_eq!(registry.task(id).expect("the task is there").state, TaskState::Pending);
        registry.start(id).expect("the task starts");
        assert_eq!(registry.task(id).expect("the task is there").state, TaskState::Running);
        registry.complete(id, serde_json::json!({ "pages": 3 })).expect("the task is done");
        let task = registry.task(id).expect("the task is there");
        assert_eq!(task.state, TaskState::Done);
        assert_eq!(task.progress, 100);
        assert_eq!(task.result, Some(serde_json::json!({ "pages": 3 })));
    }

    /// A finished task does not move again.
    #[test]
    fn a_finished_task_does_not_move_again() {
        let mut registry = registry();
        let id = registry.tasks()[0].id;
        registry.complete(id, serde_json::json!(null)).expect("the task is done");
        assert!(registry.complete(id, serde_json::json!(null)).is_err(), "it was finished twice");
        assert!(registry.start(id).is_err(), "a finished task started");
        assert!(registry.fail(id, "no").is_err(), "a finished task failed");
    }

    /// A task may be finished without this process having started it.
    ///
    /// That is the whole point: the event that says it is done may come
    /// from a server, from another process, or from a timer.
    #[test]
    fn a_task_may_be_finished_without_being_started_here() {
        let mut registry = registry();
        let id = registry.tasks()[0].id;
        registry.complete(id, serde_json::json!("from outside")).expect("the task is done");
        assert_eq!(registry.task(id).expect("the task is there").state, TaskState::Done);
    }

    /// The outside world's own id is kept.
    #[test]
    fn the_outside_worlds_own_id_is_kept() {
        let mut registry = registry();
        let id = registry.tasks()[0].id;
        registry.attach(id, "provider-42").expect("the handle is attached");
        assert_eq!(registry.task(id).expect("the task is there").handle.as_deref(), Some("provider-42"));
    }

    /// A progress is a number between 0 and 100.
    #[test]
    fn a_progress_is_a_number_between_0_and_100() {
        let mut registry = registry();
        let id = registry.tasks()[0].id;
        registry.start(id).expect("the task starts");
        registry.progress(id, 40).expect("the progress is set");
        assert_eq!(registry.task(id).expect("the task is there").progress, 40);
        assert!(registry.progress(id, 200).is_ok(), "a progress over 100 is clamped");
        assert_eq!(registry.task(id).expect("the task is there").progress, 100);
    }

    /// The registry is written down and read back.
    #[test]
    fn the_registry_is_written_down_and_read_back() {
        let mut registry = registry();
        let id = registry.tasks()[0].id;
        registry.start(id).expect("the task starts");
        registry.attach(id, "provider-42").expect("the handle is attached");
        registry.progress(id, 25).expect("the progress is set");
        let written = registry.to_json();
        let read = TaskRegistry::from_json(&written).expect("the registry is read");
        assert_eq!(read, registry, "the tasks came back");
    }

    /// A registry of nothing is not a registry.
    #[test]
    fn a_registry_of_nothing_is_refused() {
        let error = TaskRegistry::from_json(&serde_json::json!({})).expect_err("it is refused");
        assert!(error.contains("no tasks"), "{error}");
    }

    /// A task is found by the **handle**: the outside world's own name
    /// for it.
    ///
    /// This is how the other end names the work: the event that says the
    /// work is done comes back with the provider's job id, and the only
    /// way to find the task is by that id.
    #[test]
    fn a_task_is_found_by_its_handle() {
        let mut registry = registry();
        let id = registry.tasks()[0].id;
        registry.attach(id, "provider-42").expect("the handle is attached");
        let found = registry.find_by_handle("provider-42").expect("the task is there");
        assert_eq!(found.id, id);
        assert!(registry.find_by_handle("provider-99").is_none(), "there is no such task");
    }

    /// A task is finished by its handle, by whoever gets there first.
    #[test]
    fn a_task_is_finished_by_its_handle() {
        let mut registry = registry();
        let id = registry.tasks()[0].id;
        registry.attach(id, "provider-42").expect("the handle is attached");
        registry
            .complete_by_handle("provider-42", serde_json::json!({ "media": "m_1" }))
            .expect("it is done");
        let task = registry.task(id).expect("the task is there");
        assert_eq!(task.state, TaskState::Done);
        assert_eq!(task.result, Some(serde_json::json!({ "media": "m_1" })));
    }

    /// And it is failed by its handle too.
    #[test]
    fn a_task_is_failed_by_its_handle() {
        let mut registry = registry();
        let id = registry.tasks()[0].id;
        registry.attach(id, "provider-42").expect("the handle is attached");
        registry.fail_by_handle("provider-42", "the model is gone").expect("it failed");
        let task = registry.task(id).expect("the task is there");
        assert_eq!(task.state, TaskState::Failed);
        assert_eq!(task.message.as_deref(), Some("the model is gone"));
    }

    /// A handle nobody knows finishes nothing.
    #[test]
    fn a_handle_nobody_knows_finishes_nothing() {
        let mut registry = registry();
        let error =
            registry.complete_by_handle("provider-99", serde_json::json!(null)).expect_err("it is refused");
        assert!(error.contains("no task with the handle"), "{error}");
        let error = registry.fail_by_handle("provider-99", "no").expect_err("it is refused");
        assert!(error.contains("no task with the handle"), "{error}");
    }
}
