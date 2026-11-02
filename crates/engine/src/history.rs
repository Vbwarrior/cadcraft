//! WoodCraft Phase 9: local git history for `.wood` project files.
//! `plans/Wood_Workflow_Scope.md` "History (git)": commands set a dirty flag (already
//! [`crate::DocState::is_dirty`]); a background timer (`gitTimerMinutes`, a WoodCraft setting)
//! checks it, and if the drawing is dirty and no command is running, writes the file and commits
//! it to a local git repository rooted at the project file's folder — never pushed. Adding a new
//! object commits at once instead of waiting for the timer. WC also commits on exit
//! ([`Historian::shutdown`]).
//!
//! Follows the shape of [`crate::autosave`]: [`Historian::plan`] decides what to do with no I/O
//! (so it is testable with a fake clock), [`Historian::tick`] carries it out. Unlike autosave,
//! this only ever applies to a `.wood` project (a drawing with no path, or a path not ending in
//! `.wood`, is never committed — ordinary CADCraft drawings are untouched by WoodCraft history).

use std::collections::HashMap;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{DocState, Session};

/// The WoodCraft project file extension this module acts on.
const WOOD_EXT: &str = "wood";

/// `gitTimerMinutes`'s default (matches `cadcraft_wood::settings_data::WoodSettings::default`).
#[cfg(not(target_arch = "wasm32"))]
pub const DEFAULT_GIT_TIMER_MINUTES: f64 = 1.0;

#[cfg(not(target_arch = "wasm32"))]
static GIT_TIMER_MINUTES_BITS: AtomicU64 = AtomicU64::new(DEFAULT_GIT_TIMER_MINUTES.to_bits());

/// `gitTimerMinutes`: minutes between background git commits of a dirty `.wood` project; 0 turns
/// the timer off (a new object still commits at once). Set from the WoodCraft settings file once
/// the host has loaded it ([`set_git_timer_minutes`]).
#[cfg(not(target_arch = "wasm32"))]
pub fn git_timer_minutes() -> f64 {
    f64::from_bits(GIT_TIMER_MINUTES_BITS.load(Ordering::Relaxed))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_git_timer_minutes(minutes: f64) {
    GIT_TIMER_MINUTES_BITS.store(minutes.max(0.0).to_bits(), Ordering::Relaxed);
}

/// Whether `path` is a WoodCraft project file (ends in `.wood`, case-insensitively).
fn is_wood_path(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|e| e.eq_ignore_ascii_case(WOOD_EXT))
}

#[derive(Debug, Default)]
struct Tracked {
    /// Entities in the drawing as of the last commit (or since this drawing was first seen).
    last_entity_count: usize,
    /// When a change not yet committed was first seen (seconds).
    since: Option<f64>,
    /// Commands run since the last commit, for the commit message.
    commands: Vec<String>,
}

/// What to do: write the drawing to its own path and commit it.
#[derive(Clone, Debug)]
pub struct CommitJob {
    pub uid: u64,
    pub path: String,
    /// The commands run since the last commit, oldest first.
    pub commands: Vec<String>,
}

/// Git history state for one running app.
pub struct Historian {
    docs: HashMap<u64, Tracked>,
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
}

impl Default for Historian {
    fn default() -> Self {
        Historian {
            docs: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
        }
    }
}

impl Historian {
    pub fn new() -> Self {
        Historian::default()
    }

    /// A command just changed drawing `uid`, saved (or not yet saved) at `path`: if `path` is a
    /// `.wood` project, record the command id so the next commit's message lists it. A drawing
    /// with no path, or a path not ending in `.wood`, is never tracked.
    pub fn record_command(&mut self, uid: u64, path: Option<&str>, command: &str) {
        if !path.is_some_and(is_wood_path) {
            return;
        }
        self.docs.entry(uid).or_default().commands.push(command.to_string());
    }

    /// What to commit at `now` (seconds, any fixed origin) with the timer set to `minutes`:
    /// drawings whose `.wood` file is dirty and changed `minutes` ago or earlier, or that grew a
    /// new entity since the last commit (committed at once, not timed).
    pub fn plan(&mut self, s: &Session, now: f64, minutes: f64) -> Vec<CommitJob> {
        self.plan_with(s, now, minutes, false)
    }

    /// [`Historian::plan`], with `urgent` committing every dirty `.wood` project now (the app is
    /// closing) instead of waiting for the timer or a new object.
    fn plan_with(&mut self, s: &Session, now: f64, minutes: f64, urgent: bool) -> Vec<CommitJob> {
        let mut jobs = Vec::new();
        let live: std::collections::HashSet<u64> = s.docs.iter().map(|d| d.uid).collect();
        self.docs.retain(|uid, _| live.contains(uid));
        for st in &s.docs {
            let Some(path) = st.path.as_deref().filter(|p| is_wood_path(p)) else { continue };
            let count = st.doc.entity_count();
            // A drawing already dirty the first time it is seen (opened mid-edit) starts from 0,
            // so entities it already holds count as new; a clean drawing starts from its own
            // count, so nothing "grows" until something is actually added.
            let baseline = if st.is_dirty() { 0 } else { count };
            let t = self.docs.entry(st.uid).or_insert_with(|| Tracked { last_entity_count: baseline, since: None, commands: Vec::new() });
            if !st.is_dirty() {
                t.since = None;
                t.commands.clear();
                t.last_entity_count = count;
                continue;
            }
            let grew = count > t.last_entity_count;
            let since = *t.since.get_or_insert(now);
            if !urgent && !grew && minutes > 0.0 && now - since < minutes * 60.0 {
                continue;
            }
            if minutes <= 0.0 && !grew && !urgent {
                // The timer is off: only a new object forces a commit.
                continue;
            }
            jobs.push(CommitJob { uid: st.uid, path: path.to_string(), commands: std::mem::take(&mut t.commands) });
            t.since = None;
            t.last_entity_count = count;
        }
        jobs
    }

    /// Seconds from `now` until the next commit is due, if one is waiting on the timer.
    pub fn next_due(&self, now: f64, minutes: f64) -> Option<f64> {
        if minutes <= 0.0 {
            return None;
        }
        self.docs.values().filter_map(|t| t.since).map(|since| (since + minutes * 60.0 - now).max(0.0)).reduce(f64::min)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::time::Duration;

    use super::*;

    /// Write `doc`'s drawing to `path` (reusing the same `.wood` write path as `woodsave`) and
    /// commit it to a git repository in the file's folder, initializing one if none exists yet.
    /// Logged, never returned: a failed background commit must not disturb editing.
    fn write_and_commit(st: &DocState, path: &str, commands: &[String]) {
        let Some(hooks) = crate::cmd::file::io() else {
            log::warn!("wood history: file formats are not available in this build");
            return;
        };
        let bytes = match (hooks.write)(&st.doc, "woodcraft-project.dxf") {
            Ok(b) => b,
            Err(e) => {
                log::warn!("wood history: {path}: {e}");
                return;
            }
        };
        let tmp = format!("{path}.cadcraft-tmp");
        if let Err(e) = std::fs::write(&tmp, &bytes).and_then(|()| std::fs::rename(&tmp, path)) {
            log::warn!("wood history: {path}: {e}");
            return;
        }
        if let Err(e) = commit(path, commands) {
            log::warn!("wood history: commit {path}: {e}");
        }
    }

    /// Open (initializing if needed) the git repository rooted at `path`'s folder, stage `path`
    /// and commit it. The message lists the commands run since the last commit.
    fn commit(path: &str, commands: &[String]) -> std::result::Result<(), git2::Error> {
        let dir = Path::new(path).parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        let repo = git2::Repository::open(dir).or_else(|_| git2::Repository::init(dir))?;
        let file_name = Path::new(path).file_name().ok_or_else(|| git2::Error::from_str("project file has no name"))?;
        let mut index = repo.index()?;
        index.add_path(Path::new(file_name))?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let sig = repo.signature().or_else(|_| git2::Signature::now("WoodCraft", "woodcraft@localhost"))?;
        let message = if commands.is_empty() { "WoodCraft autosave".to_string() } else { commands.join(", ") };
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        match parent {
            Some(parent) => repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent])?,
            None => repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[])?,
        };
        Ok(())
    }

    impl Historian {
        /// Commit what is due now (writing and committing on the calling thread: these are small
        /// text files, not worth a background thread). Returns how long until the next commit is
        /// due, so the host can wake up for it.
        pub fn tick(&mut self, s: &Session) -> Option<Duration> {
            let now = self.start.elapsed().as_secs_f64();
            let minutes = git_timer_minutes();
            let jobs = self.plan(s, now, minutes);
            self.perform(s, jobs);
            self.next_due(now, minutes).map(Duration::from_secs_f64)
        }

        /// The app is closing: commit every dirty `.wood` project now, regardless of the timer.
        pub fn shutdown(&mut self, s: &Session) {
            let jobs = self.plan_with(s, 0.0, 0.0, true);
            self.perform(s, jobs);
        }

        fn perform(&mut self, s: &Session, jobs: Vec<CommitJob>) {
            for job in jobs {
                let Some(st) = s.docs.iter().find(|d| d.uid == job.uid) else { continue };
                write_and_commit(st, &job.path, &job.commands);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn wood_session() -> Session {
        let mut s = Session::new();
        cadcraft_wood::store::put(std::sync::Arc::make_mut(&mut s.state_mut().unwrap().doc), &cadcraft_wood::WoodData::default()).unwrap();
        let st = s.state_mut().unwrap();
        st.path = Some("/project/box.wood".to_string());
        st.saved = st.doc.clone();
        s
    }

    fn draw(s: &mut Session) {
        s.execute("circle", &json!({"center": [0, 0], "radius": 1})).unwrap();
    }

    /// Move the drawing's one entity: a change that does not add a new object, so it waits for
    /// the timer instead of committing at once.
    fn edit(s: &mut Session) {
        let h = s.doc().unwrap().model.handles()[0];
        s.execute("move", &json!({"handles": [h.hex()], "delta": [1, 0]})).unwrap();
    }

    /// [`Historian::plan`] takes `&Session`, so it cannot run on `s.history` itself (that would
    /// borrow `s` both ways at once): pull it out, plan, then put it back, exactly as a host
    /// would call it from outside `Session`.
    fn plan(s: &mut Session, now: f64, minutes: f64) -> Vec<CommitJob> {
        let mut h = std::mem::take(&mut s.history);
        let jobs = h.plan(s, now, minutes);
        s.history = h;
        jobs
    }

    #[test]
    fn a_dirty_wood_project_commits_timer_minutes_after_the_change() {
        // Uses `s.history` (not a separate `Historian`), since commands land there as `execute`
        // runs them ([`Session::execute`]'s WoodCraft Phase 9 hook): this is what a real host sees.
        let mut s = wood_session();
        draw(&mut s);
        assert_eq!(plan(&mut s, 0.0, 1.0).len(), 1, "the first entity commits at once");

        edit(&mut s);
        assert!(plan(&mut s, 10.0, 1.0).is_empty(), "an ordinary change starts the clock instead");
        assert_eq!(s.history.next_due(10.0, 1.0), Some(60.0));
        assert!(plan(&mut s, 69.0, 1.0).is_empty());
        let jobs = plan(&mut s, 70.0, 1.0);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].path, "/project/box.wood");
        assert_eq!(jobs[0].commands, vec!["move".to_string()]);

        // Nothing new since the commit: no more jobs.
        assert!(plan(&mut s, 1000.0, 1.0).is_empty());
    }

    #[test]
    fn adding_an_entity_commits_at_once_and_restarts_the_timer() {
        let mut s = wood_session();
        let mut h = Historian::new();
        draw(&mut s);
        let jobs = h.plan(&s, 0.0, 1.0);
        assert_eq!(jobs.len(), 1, "a new object commits immediately, not after the timer");

        // A second entity before the next commit also commits at once.
        draw(&mut s);
        let jobs = h.plan(&s, 0.1, 1.0);
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn only_wood_projects_are_tracked() {
        let mut s = Session::new();
        s.state_mut().unwrap().path = Some("/plans/ordinary.dxf".to_string());
        draw(&mut s);
        let mut h = Historian::new();
        assert!(h.plan(&s, 1000.0, 1.0).is_empty(), "a non-.wood drawing is never committed by WoodCraft history");
    }

    #[test]
    fn a_path_wood_project_with_no_path_is_never_committed() {
        let mut s = Session::new();
        draw(&mut s);
        let mut h = Historian::new();
        assert!(h.plan(&s, 1000.0, 1.0).is_empty(), "an unsaved drawing has no file to commit to");
    }

    #[test]
    fn saving_clears_the_pending_commit() {
        let mut s = wood_session();
        let mut h = Historian::new();
        draw(&mut s);
        h.plan(&s, 0.0, 1.0);
        let st = s.state_mut().unwrap();
        st.saved = st.doc.clone();
        assert!(h.plan(&s, 60.0, 1.0).is_empty(), "no unsaved changes left, nothing to commit");
    }

    #[test]
    fn timer_off_still_commits_new_objects() {
        let mut s = wood_session();
        let mut h = Historian::new();
        draw(&mut s);
        assert_eq!(h.plan(&s, 0.0, 0.0).len(), 1, "minutes=0 still commits at once for a new object");
    }

    #[test]
    fn git_timer_minutes_defaults_to_one_and_round_trips() {
        assert_eq!(git_timer_minutes(), DEFAULT_GIT_TIMER_MINUTES);
        set_git_timer_minutes(5.0);
        assert_eq!(git_timer_minutes(), 5.0);
        // A negative setting is clamped to 0 (timer off), never stored negative.
        set_git_timer_minutes(-3.0);
        assert_eq!(git_timer_minutes(), 0.0);
        set_git_timer_minutes(DEFAULT_GIT_TIMER_MINUTES);
    }

    #[test]
    fn closed_drawings_are_forgotten() {
        let mut s = wood_session();
        let uid = s.state().unwrap().uid;
        draw(&mut s);
        let mut h = Historian::new();
        h.plan(&s, 0.0, 1.0);
        s.execute("close", &json!({})).unwrap();
        h.plan(&s, 1.0, 1.0);
        assert!(!h.docs.contains_key(&uid));
    }
}
