//! **The build stamp a chronicle carries in its header**
//! (`lab::ui::chronicle_text`, the `BUILD` line).
//!
//! The 10-03 playtest's chronicle (`chronicle-herb_ant-s1.txt`) named its
//! seed, its bed and its dials and not the code that ran them, so which build
//! produced it was found *by elimination* against the merge history. A file
//! the owner uploads and an agent reads days later has to say which commit it
//! came from, or every number in it is a number about some tree.
//!
//! Emits two compile-time variables, read with `option_env!` so a build from
//! a tarball with no `.git` still compiles:
//!
//! - `PIXEL_PHYSICS_GIT_SHA` -- `git rev-parse --short HEAD`, or `unknown`.
//! - `PIXEL_PHYSICS_GIT_DATE` -- that commit's committer date (`%cs`,
//!   `YYYY-MM-DD`), or `unknown`.
//!
//! **A commit date, not a "built at" time, and deliberately.** A wall-clock
//! stamp taken here is only refreshed when this script reruns, which (below)
//! is when `HEAD` moves -- so after an uncommitted edit and a rebuild it would
//! name the *previous* build's moment and read as this one's. The commit date
//! is true of the sha beside it for as long as that sha is. Uncommitted edits
//! are not flagged at all: `git status` per build is a scan of a large tree,
//! and the sha already says which commit the edit sits on.
//!
//! **Reruns only when `HEAD` or the branch it points at moves**, not on every
//! source edit (cargo's default with no `rerun-if-changed` lines is to rerun
//! on any change in the package, which would re-fork `git` on every build).
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn main() {
    let sha = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let date = git(&["log", "-1", "--format=%cs"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=PIXEL_PHYSICS_GIT_SHA={sha}");
    println!("cargo:rustc-env=PIXEL_PHYSICS_GIT_DATE={date}");
    // `--git-path` rather than a literal `.git/HEAD`: in a worktree (this repo
    // works in `.claude/worktrees/*` constantly) `.git` is a file, and HEAD
    // lives under the main checkout's `.git/worktrees/<name>/`.
    //
    // Only paths that exist: cargo treats a missing `rerun-if-changed` path as
    // always stale, and a branch whose ref lives only in `packed-refs` has no
    // loose ref file -- that would fork `git` on every build after all.
    let mut watch = vec![
        git(&["rev-parse", "--git-path", "HEAD"]),
        git(&["rev-parse", "--git-path", "packed-refs"]),
    ];
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
        watch.push(git(&["rev-parse", "--git-path", &branch]));
    }
    for path in watch.into_iter().flatten() {
        if std::path::Path::new(&path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
