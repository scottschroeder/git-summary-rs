use std::path;
use std::env;
use std::fs;
use walkdir::WalkDir;
use std::ffi::OsStr;

use ::Result;

pub fn get_working_dir(user_path: Option<&str>) -> Result<path::PathBuf> {
    if let Some(s) = user_path {
        let p = fs::canonicalize(s)?;
        trace!("Using real path: {:?}", p);
        let meta = p.metadata()?;
        if !meta.is_dir() {
            bail!("the path {:?} is not a directory", p);
        }
        Ok(p)
    } else {
        Ok(env::current_dir()?)
    }
}

pub fn get_all_repos<P: AsRef<path::Path>>(src_path: P, deep: bool) -> Vec<path::PathBuf> {
    let git_dir = OsStr::new(".git");


    WalkDir::new(src_path.as_ref())
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| !deep_filter(deep, e))
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if entry.path().file_name() == Some(git_dir) {
                    let git_repo = entry.path().parent().unwrap();
                    return Some(git_repo.to_owned())
                }
            }
            None
        })
        .collect()
}

// TODO refactor
fn deep_filter(deep: bool, entry: &walkdir::DirEntry) -> bool {
    if is_hidden(entry) {
        trace!("Filtering {:?} (hidden)", entry.path().display());
        return true
    }
    if deep {
        return false;
    } else if entry.depth() > 2 {
        trace!("Filtering {:?} d={}", entry.path().display(), entry.depth());
        true
    } else {
        trace!("Keeping {:?} d={}", entry.path().display(), entry.depth());
        false
    }
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| {
            s.starts_with(".") && !(s == ".git")
        })
        .unwrap_or(false)
}