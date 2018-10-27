
use git2;
use std::path::Path;

use ::Result;

pub fn open_repo<P: AsRef<Path>>(path: P) -> Result<Option<String>> {
    let repo = git2::Repository::open(path)?;
    let gitref = repo.head()?;
    Ok(if gitref.is_branch() {
        gitref.shorthand().map(|s| s.to_owned())
    } else {
        None
    })
}