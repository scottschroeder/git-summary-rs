use std::{env, fs, path};
use walkdir::WalkDir;

use crate::Result;

const GIT_DIR: &str = ".git";

pub fn get_working_dir(user_path: Option<&str>) -> Result<path::PathBuf> {
    if let Some(s) = user_path {
        let p = fs::canonicalize(s)?;
        let meta = p.metadata()?;
        if !meta.is_dir() {
            anyhow::bail!("the path {:?} is not a directory", p);
        }
        Ok(p)
    } else {
        Ok(env::current_dir()?)
    }
}

pub fn shorten<PB>(base: PB, full: &path::Path) -> &path::Path
where
    PB: AsRef<path::Path>,
{
    full.strip_prefix(base.as_ref().parent().unwrap_or(base.as_ref()))
        .unwrap_or(full)
}

pub fn get_all_repos<P: AsRef<path::Path>>(
    src_path: P,
    deep: bool,
    do_hidden: bool,
) -> Vec<path::PathBuf> {
    WalkDir::new(src_path.as_ref())
        .follow_links(true)
        .into_iter()
        .filter_entry(move |e| !deep_filter(deep, !do_hidden, e))
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if is_git_dir(&entry) {
                    let git_repo = fs::canonicalize(entry.path().parent().unwrap()).unwrap();

                    return Some(git_repo);
                }
            }
            None
        })
        .collect()
}

// TODO refactor
fn deep_filter(deep: bool, skip_hidden: bool, entry: &walkdir::DirEntry) -> bool {
    if skip_hidden && is_hidden(entry) {
        //trace!("Filtering {:?} (hidden)", entry.path().display());
        return true;
    }
    if deep {
        false
    } else {
        entry.depth() > 2
    }
}

fn is_git_dir(entry: &walkdir::DirEntry) -> bool {
    check_entry_filename(entry, |s| s == GIT_DIR)
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    check_entry_filename(entry, |s| s.starts_with('.') && s != GIT_DIR)
}

fn check_entry_filename<F>(entry: &walkdir::DirEntry, predicate: F) -> bool
where
    F: FnOnce(&str) -> bool,
{
    entry
        .file_name()
        .to_str()
        .map(predicate)
        .unwrap_or_else(|| {
            log::error!("unable to parse {:?} as str", entry.path().display());
            false
        })
}
