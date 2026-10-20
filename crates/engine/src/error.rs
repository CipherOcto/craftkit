//! Errors, and the one thing every one of them must do.
//!
//! # The message is the interface
//!
//! An agent reading a failure sees only [`EngineError::to_string`]. So a message
//! says **what was refused, by what rule, and what to do instead**. "invalid
//! input" is not an error message; it is a shrug.

use craftkit_core::{CoreError, ParamError};
use craftkit_eval::EvalError;
use craftkit_kernel::KernelError;
use craftkit_model::DocError;
use serde_json::Value as Json;

/// Everything a command can fail with.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EngineError {
    #[error("there is no command `{0}`; run `craftkit-cli commands` for the list")]
    Unknown(String),

    #[error("`{id}` is not available right now: {reason}")]
    Disabled { id: String, reason: String },

    #[error("`{id}` rejected its parameters: {source}")]
    Param { id: String, source: ParamError },

    #[error("{0}")]
    Core(CoreError),

    #[error("{0}")]
    Doc(DocError),

    #[error("{0}")]
    Eval(EvalError),

    #[error("{0}")]
    Kernel(KernelError),

    /// A panic escaped a command. The document has been restored.
    #[error("`{id}` failed unexpectedly ({detail}); your document is unchanged")]
    Internal { id: String, detail: String },
}

impl EngineError {
    /// Whether this is a crash rather than a refusal.
    ///
    /// The distinction drives everything downstream: an internal error is worth
    /// a crash report and a bug, a refusal is worth a better message.
    pub fn is_internal(&self) -> bool {
        match self {
            EngineError::Internal { .. } => true,
            EngineError::Kernel(k) => k.is_internal(),
            _ => false,
        }
    }

    /// A stable, machine-readable kind. An agent should branch on this, never
    /// on the message text.
    pub fn kind(&self) -> &'static str {
        match self {
            EngineError::Unknown(_) => "unknown-command",
            EngineError::Disabled { .. } => "disabled",
            EngineError::Param { .. } => "bad-parameter",
            EngineError::Core(_) => "invalid-value",
            EngineError::Doc(_) => "invalid-document",
            EngineError::Eval(_) => "cannot-evaluate",
            EngineError::Kernel(_) => "kernel",
            EngineError::Internal { .. } => "internal",
        }
    }

    /// The command that failed, when one is known.
    pub fn command(&self) -> Option<&str> {
        match self {
            EngineError::Disabled { id, .. }
            | EngineError::Param { id, .. }
            | EngineError::Internal { id, .. } => Some(id),
            _ => None,
        }
    }

    /// Fill in the command id on an error raised by a helper that did not know
    /// it.
    ///
    /// Command bodies read their parameters through small shared helpers, and
    /// threading an id through every one of them would be noise in every
    /// command. `Session::execute` fills the id in instead, once, so every error
    /// a caller sees names its command.
    pub fn with_command(self, id: &str) -> Self {
        match self {
            EngineError::Disabled { reason, .. } => EngineError::Disabled { id: id.to_string(), reason },
            EngineError::Param { source, .. } => EngineError::Param { id: id.to_string(), source },
            EngineError::Internal { detail, .. } => EngineError::Internal { id: id.to_string(), detail },
            other => other,
        }
    }

    /// The JSON an API surface returns.
    pub fn to_json(&self) -> Json {
        serde_json::json!({
            "ok": false,
            "kind": self.kind(),
            "internal": self.is_internal(),
            "command": self.command(),
            "message": self.to_string(),
        })
    }
}

impl From<ParamError> for EngineError {
    /// The id is filled in by `Session::execute`, which is the only place that
    /// knows which command was running.
    fn from(e: ParamError) -> Self {
        EngineError::Param { id: String::new(), source: e }
    }
}

impl From<CoreError> for EngineError {
    fn from(e: CoreError) -> Self {
        EngineError::Core(e)
    }
}

impl From<DocError> for EngineError {
    fn from(e: DocError) -> Self {
        EngineError::Doc(e)
    }
}

impl From<EvalError> for EngineError {
    fn from(e: EvalError) -> Self {
        EngineError::Eval(e)
    }
}

impl From<KernelError> for EngineError {
    fn from(e: KernelError) -> Self {
        EngineError::Kernel(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_command_names_the_id_and_where_to_look() {
        let e = EngineError::Unknown("node.frobnicate".to_string());
        let msg = e.to_string();
        assert!(msg.contains("node.frobnicate"), "{msg}");
        assert!(msg.contains("commands"), "{msg}");
        assert_eq!(e.kind(), "unknown-command");
        assert!(!e.is_internal());
    }

    #[test]
    fn a_bad_parameter_names_the_key() {
        let e = EngineError::Param {
            id: "node.add".to_string(),
            source: ParamError::new("kind", "expected text, found a number"),
        };
        let msg = e.to_string();
        assert!(msg.contains("node.add"), "{msg}");
        assert!(msg.contains("kind"), "{msg}");
        assert_eq!(e.command(), Some("node.add"));
        assert_eq!(e.kind(), "bad-parameter");
    }

    #[test]
    fn a_recovered_kernel_panic_counts_as_internal() {
        let e = EngineError::Kernel(KernelError::Internal { what: "applying a gain", detail: "boom".into() });
        assert!(e.is_internal());
        assert_eq!(e.kind(), "kernel");
    }

    #[test]
    fn the_json_carries_the_kind_so_an_agent_can_branch_on_it() {
        let j = EngineError::Unknown("x".into()).to_json();
        assert_eq!(j["ok"], Json::Bool(false));
        assert_eq!(j["kind"], Json::from("unknown-command"));
        assert!(j["message"].is_string());
    }
}
