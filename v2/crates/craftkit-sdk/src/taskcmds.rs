//! The commands for the **tasks**: work that may outlive the process.
//!
//! The registry itself is in the SDK's `tasks` module; these are the
//! commands that reach it, so that a person, an agent and a script all
//! reach the same list.

use craftkit_derive::Command;

use crate::tasks::TaskState;
use crate::{EngineError, Number, Outcome, Session, Text};

/// The number of a task.
///
/// It is a number and not a node reference: a task is not in the
/// document, so it has no node.
fn task_id(value: Number) -> Result<u64, EngineError> {
    if !value.0.is_finite() || value.0 < 0.0 || value.0.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: "task".to_string(),
            message: "a task is numbered with a whole number".to_string(),
        });
    }
    Ok(value.0 as u64)
}

/// Register a task.
///
/// It is **pending**: registered, and not started. Whoever finishes it
/// may not be this process — that is the point.
#[derive(Command)]
#[command(id = "task.add", area = "task", icon = "plus", menu("Task"))]
pub struct AddTask {
    /// What kind of work it is: `pdf.render`, `artcraft.generate`, …
    pub kind: Text,
}

impl AddTask {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let kind = self.kind.0.clone();
        if kind.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "task.add".to_string(),
                message: "a task has a kind".to_string(),
            });
        }
        let id = session.tasks_mut().register(kind.clone());
        Ok(Outcome::changed(format!("registered the `{kind}` as task {id}"))
            .with_data(crate::serde_json::json!({ "task": id, "kind": kind })))
    }
}

/// The tasks, and what state they are in.
#[derive(Command)]
#[command(id = "task.list", area = "task", icon = "list", menu("Task"))]
pub struct ListTasks {
    /// Only the tasks in this state.
    pub state: Option<Text>,
}

impl ListTasks {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let wanted = match &self.state {
            Some(state) => match TaskState::of(&state.0) {
                Some(state) => Some(state),
                None => {
                    return Err(EngineError::Failed {
                        id: "task.list".to_string(),
                        message: format!("`{}` is not a state a task can be in", state.0),
                    });
                }
            },
            None => None,
        };
        let tasks: Vec<crate::serde_json::Value> = session
            .tasks()
            .tasks()
            .iter()
            .filter(|task| wanted.is_none_or(|state| task.state == state))
            .map(|task| task.to_json())
            .collect();
        let count = tasks.len();
        Ok(Outcome::new(format!("{count} task(s)")).with_data(crate::serde_json::json!({ "tasks": tasks })))
    }
}

/// Start a task.
#[derive(Command)]
#[command(id = "task.start", area = "task", icon = "play", menu("Task"), mutates, undoable)]
pub struct StartTask {
    /// The task.
    pub task: Number,
}

impl StartTask {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = task_id(self.task)?;
        session
            .tasks_mut()
            .start(id)
            .map_err(|message| EngineError::Failed { id: "task.start".to_string(), message })?;
        Ok(Outcome::changed(format!("task {id} is running")))
    }
}

/// Give a task the outside world's own id.
///
/// A provider's job id: the thing to ask about it, and the thing that
/// comes back when it is done.
#[derive(Command)]
#[command(id = "task.attach", area = "task", icon = "link", menu("Task"), mutates, undoable)]
pub struct AttachTask {
    /// The task.
    pub task: Number,
    /// The other end's own id.
    pub handle: Text,
}

impl AttachTask {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = task_id(self.task)?;
        let handle = self.handle.0.clone();
        if handle.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "task.attach".to_string(),
                message: "a handle is not empty".to_string(),
            });
        }
        session
            .tasks_mut()
            .attach(id, handle.clone())
            .map_err(|message| EngineError::Failed { id: "task.attach".to_string(), message })?;
        Ok(Outcome::changed(format!("task {id} is `{handle}` at the other end"))
            .with_data(crate::serde_json::json!({ "task": id, "handle": handle })))
    }
}

/// Say how far a task has got.
#[derive(Command)]
#[command(id = "task.progress", area = "task", icon = "list", menu("Task"), mutates, undoable)]
pub struct ProgressTask {
    /// The task.
    pub task: Number,
    /// How far, as a percent.
    pub percent: Number,
}

impl ProgressTask {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = task_id(self.task)?;
        if !self.percent.0.is_finite() || self.percent.0 < 0.0 {
            return Err(EngineError::Failed {
                id: "task.progress".to_string(),
                message: "a progress is a percent".to_string(),
            });
        }
        let percent = self.percent.0.min(100.0) as u8;
        session
            .tasks_mut()
            .progress(id, percent)
            .map_err(|message| EngineError::Failed { id: "task.progress".to_string(), message })?;
        Ok(Outcome::changed(format!("task {id} is {percent}% done"))
            .with_data(crate::serde_json::json!({ "task": id, "progress": percent })))
    }
}

/// Finish a task, with what it produced.
///
/// **Whoever gets there first finishes it.** This process may not have
/// started the task at all.
#[derive(Command)]
#[command(id = "task.complete", area = "task", icon = "check", menu("Task"), mutates, undoable)]
pub struct CompleteTask {
    /// The task.
    pub task: Number,
    /// What it produced, if it produced anything.
    pub result: Option<crate::serde_json::Value>,
}

impl CompleteTask {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = task_id(self.task)?;
        let result = match &self.result {
            Some(result) if !result.is_null() => result.clone(),
            _ => crate::serde_json::Value::Null,
        };
        session
            .tasks_mut()
            .complete(id, result)
            .map_err(|message| EngineError::Failed { id: "task.complete".to_string(), message })?;
        Ok(Outcome::changed(format!("task {id} is done")))
    }
}

/// Finish a task, with the reason it did not finish.
#[derive(Command)]
#[command(id = "task.fail", area = "task", icon = "cross", menu("Task"), mutates, undoable)]
pub struct FailTask {
    /// The task.
    pub task: Number,
    /// Why.
    pub message: Text,
}

impl FailTask {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = task_id(self.task)?;
        let message = self.message.0.clone();
        session
            .tasks_mut()
            .fail(id, message.clone())
            .map_err(|message| EngineError::Failed { id: "task.fail".to_string(), message })?;
        Ok(Outcome::changed(format!("task {id} failed: {message}")))
    }
}

/// Call a task off.
#[derive(Command)]
#[command(id = "task.cancel", area = "task", icon = "minus", menu("Task"), mutates, undoable)]
pub struct CancelTask {
    /// The task.
    pub task: Number,
}

impl CancelTask {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = task_id(self.task)?;
        session
            .tasks_mut()
            .cancel(id)
            .map_err(|message| EngineError::Failed { id: "task.cancel".to_string(), message })?;
        Ok(Outcome::changed(format!("task {id} is called off")))
    }
}

/// Write the tasks down, so that they survive this process.
///
/// The registry is a list of tasks and nothing else, and it is written
/// as JSON: a program that is closed and opened again finds the tasks it
/// left behind, and can go on waiting for them.
#[derive(Command)]
#[command(id = "task.save", area = "task", icon = "save", menu("Task"), mutates, undoable)]
pub struct SaveTasks {
    /// Where to write them.
    pub path: Text,
}

impl SaveTasks {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let text = crate::serde_json::to_string_pretty(&session.tasks().to_json()).map_err(|error| {
            EngineError::Failed {
                id: "task.save".to_string(),
                message: format!("the tasks could not be written: {error}"),
            }
        })?;
        std::fs::write(&path, text).map_err(|error| EngineError::Failed {
            id: "task.save".to_string(),
            message: format!("`{path}` could not be written: {error}"),
        })?;
        let count = session.tasks().tasks().len();
        Ok(Outcome::changed(format!("wrote {count} task(s) to `{path}`"))
            .with_data(crate::serde_json::json!({ "path": path, "tasks": count })))
    }
}

/// Read the tasks back.
///
/// The list survives the process that made it: this is what makes a task
/// that outlives the program possible.
#[derive(Command)]
#[command(id = "task.load", area = "task", icon = "open", menu("Task"), mutates, undoable)]
pub struct LoadTasks {
    /// Where to read them from.
    pub path: Text,
}

impl LoadTasks {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let text = std::fs::read_to_string(&path).map_err(|error| EngineError::Failed {
            id: "task.load".to_string(),
            message: format!("`{path}` could not be read: {error}"),
        })?;
        let value: crate::serde_json::Value =
            crate::serde_json::from_str(&text).map_err(|error| EngineError::Failed {
                id: "task.load".to_string(),
                message: format!("`{path}` is not a list of tasks: {error}"),
            })?;
        let registry = crate::tasks::TaskRegistry::from_json(&value)
            .map_err(|message| EngineError::Failed { id: "task.load".to_string(), message })?;
        let count = registry.tasks().len();
        *session.tasks_mut() = registry;
        Ok(Outcome::changed(format!("read {count} task(s) from `{path}`"))
            .with_data(crate::serde_json::json!({ "path": path, "tasks": count })))
    }
}
