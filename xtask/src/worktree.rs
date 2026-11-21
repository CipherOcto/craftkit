//! `cargo xtask worktree` — the tree has **one** worktree, and this checks it.
//!
//! # The rule
//!
//! There is one worktree on disk, and it is `main`. A second checkout is a
//! second place where the tree can be changed, and the second place is
//! where a change goes that nobody reviews.
//!
//! # Why this is a gate and not a habit
//!
//! The survey of the cost of the tree read the runtime's own record of
//! every session, and it found the tree being worked in **two** places: the
//! root tree, and a second checkout at `../craftkit-v2` on the branch
//! `exp/meta-v2`, from the 29th of October to the 13th of November —
//! **two weeks** of work in a second worktree, with no gate, and no notice.
//!
//! The rule was in `AGENTS.md` the whole time. A rule that is only written
//! down is a rule that is a wish, and this one could be checked in a second:
//! git knows every worktree of the repository.
//!
//! # What it does not say
//!
//! - **It does not check the branches.** A branch that is not `main` is the
//!   owner's business, and a checkout on a build machine is often detached.
//! - **It does not say a branch may not exist.** The rule that every other
//!   branch is deleted is a rule about the tree's history, and it is the
//!   owner's to keep.

use std::path::Path;

/// One **worktree** of the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    /// Where it is on disk.
    pub path: String,
    /// The branch it is on, when it is on one.
    pub branch: Option<String>,
}

/// The worktrees of the repository.
pub fn the_worktrees(root: &Path) -> Result<Vec<Worktree>, String> {
    let output = crate::command("git", &["worktree", "list", "--porcelain"], root)
        .map_err(|e| format!("git could not say what the worktrees are: {e}"))?;
    if !output.ok() {
        // A repository with no commits has no worktree list; that is not an
        // error, it is a tree that has not started.
        return Ok(Vec::new());
    }
    Ok(parse(&output.stdout))
}

/// Read the worktrees out of git's own answer.
pub fn parse(text: &str) -> Vec<Worktree> {
    let mut worktrees = Vec::new();
    let mut path: Option<String> = None;
    let mut branch: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("worktree ") {
            if let Some(previous) = path.take() {
                worktrees.push(Worktree { path: previous, branch: branch.take() });
            }
            path = Some(rest.to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("branch ") {
            branch = Some(rest.trim_start_matches("refs/heads/").to_string());
        }
    }
    if let Some(previous) = path {
        worktrees.push(Worktree { path: previous, branch });
    }
    worktrees
}

/// The **problems** the worktrees have.
pub fn problems(worktrees: &[Worktree], root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    if worktrees.len() > 1 {
        let others: Vec<String> = worktrees
            .iter()
            .filter(|worktree| Path::new(&worktree.path) != root)
            .map(|worktree| worktree.path.clone())
            .collect();
        problems.push(format!(
            "the tree has {} worktrees, and there is one: {}",
            worktrees.len(),
            others.join(", ")
        ));
    }
    problems
}

/// Run the command.
pub fn run(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    let worktrees = the_worktrees(root)?;
    println!("   {}", worktrees.len());
    for worktree in &worktrees {
        let branch = worktree.branch.clone().unwrap_or_else(|| "detached".to_string());
        println!("   {} {branch}", worktree.path);
    }
    let problems = problems(&worktrees, root);
    if !problems.is_empty() {
        return Err(problems.join("\n"));
    }
    println!("   ok");
    Ok(crate::Outcome::Ran)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_s_own_answer_is_read() {
        let text = "worktree /a/tree\nHEAD abc123\nbranch refs/heads/main\n\nworktree /a/other\nHEAD def456\nbranch refs/heads/exp/meta-v2\n\n";
        let worktrees = parse(text);
        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[0].path, "/a/tree");
        assert_eq!(worktrees[0].branch.as_deref(), Some("main"));
        assert_eq!(worktrees[1].branch.as_deref(), Some("exp/meta-v2"));
    }

    #[test]
    fn a_second_worktree_is_a_problem() {
        let worktrees =
            parse("worktree /a/tree\nbranch refs/heads/main\n\nworktree /a/other\nbranch refs/heads/x\n\n");
        let problems = problems(&worktrees, Path::new("/a/tree"));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("/a/other"), "{problems:?}");
    }

    #[test]
    fn one_worktree_is_no_problem() {
        let worktrees = parse("worktree /a/tree\nbranch refs/heads/main\n\n");
        assert!(problems(&worktrees, Path::new("/a/tree")).is_empty());
    }

    #[test]
    fn a_worktree_without_a_branch_is_a_worktree() {
        let worktrees = parse("worktree /a/tree\nHEAD abc123\n\n");
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].branch, None);
    }
}
