use git2;

use cache::Cache;
use net_util::{tcp_check, SocketData};
use std::fmt;
use url;
use Result;

#[derive(Debug, Default, PartialEq)]
pub struct RepoStatus {
    pub uncommited_changes: bool,
    pub untracked_files: bool,
    pub new_files: bool,
    pub local_ahead: bool,
    pub local_behind: bool,
    pub err_check: bool,
}

impl fmt::Display for RepoStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.untracked_files {
            write!(f, "?")?
        } else {
            write!(f, "_")?
        }
        if self.new_files {
            write!(f, "+")?
        } else {
            write!(f, "_")?
        }
        if self.uncommited_changes {
            write!(f, "M")?
        } else {
            write!(f, "_")?
        }
        if self.local_ahead {
            write!(f, "^")?
        } else if self.local_behind {
            write!(f, "v")?
        } else {
            write!(f, "_")?
        }
        if self.err_check {
            write!(f, "X")?
        } else {
            write!(f, "_")?
        }
        Ok(())
    }
}

pub enum RepoSeverity {
    Clean,
    NeedSync,
    AheadBehind,
    Dirty,
}

impl RepoStatus {
    pub fn is_clean(&self) -> bool {
        *self == RepoStatus::default()
    }
    pub fn severity(&self) -> RepoSeverity {
        if self.uncommited_changes || self.untracked_files || self.new_files {
            RepoSeverity::Dirty
        } else if self.local_behind || self.local_ahead {
            RepoSeverity::AheadBehind
        } else if self.err_check {
            RepoSeverity::NeedSync
        } else {
            RepoSeverity::Clean
        }
    }
}

fn uncommitted_changes() -> git2::Status {
    use git2::Status;
    Status::INDEX_MODIFIED
        | Status::INDEX_DELETED
        | Status::INDEX_RENAMED
        | Status::INDEX_TYPECHANGE
        | Status::WT_MODIFIED
        | Status::WT_DELETED
        | Status::WT_TYPECHANGE
        | Status::WT_RENAMED
        | Status::CONFLICTED
}

pub fn summarize_one_git_repo(
    repo: &git2::Repository,
    fetch: bool,
    netcache: Cache<SocketData, bool>,
) -> Result<RepoStatus> {
    let head = repo.head().unwrap();
    let head_oid = head
        .resolve()?
        .target()
        .ok_or_else(|| format_err!("Unable to resolve OID for head"))?;

    let mut status = RepoStatus::default();

    if let Ok((mut upstream_oid, upstream_ref)) = repo.revparse_ext("@{u}") {
        if fetch {
            if let Some(gitref) = upstream_ref {
                match do_fetch(repo, gitref, netcache) {
                    Ok(()) => {
                        let (new_upstream_oid, _) = repo.revparse_ext("@{u}")?;
                        upstream_oid = new_upstream_oid;
                    },
                    Err(e) => {
                        error!(
                            "Could not fetch {}: {}",
                            repo.workdir().unwrap().display(),
                            e
                        );
                        status.err_check = true;
                    }
                }
            }
        }
        status.local_ahead = repo.graph_descendant_of(head_oid, upstream_oid.id())?;
        status.local_behind = repo.graph_descendant_of(upstream_oid.id(), head_oid)?;
    }

    let mut aggregate_status = git2::Status::empty();
    for file in repo.statuses(None)?.iter() {
        //debug!("file: {:?} status: {:?}", file.path(), file.status());
        aggregate_status |= file.status();
    }
    //debug!("aggregate status: {:?}", aggregate_status);

    if git2::Status::WT_NEW.intersects(aggregate_status) {
        status.untracked_files = true;
    }
    if git2::Status::INDEX_NEW.intersects(aggregate_status) {
        status.new_files = true;
    }
    if uncommitted_changes().intersects(aggregate_status) {
        status.uncommited_changes = true;
    }

    Ok(status)
}

fn do_fetch(
    repo: &git2::Repository,
    upstream_ref: git2::Reference,
    netcache: Cache<SocketData, bool>,
) -> Result<()> {
    let (mut remote, remote_branch) =
        parse_remote_from_ref(upstream_ref).and_then(|(remote_name, remote_branch)| {
            repo.find_remote(&remote_name)
                .map_err(|e| {
                    format_err!(
                        "Unable to get remote {} for repo {}: {}",
                        remote_name,
                        repo.workdir().unwrap().display(),
                        e
                    )
                })
                .map(|remote| (remote, remote_branch))
        })?;
    if let Some(url_string) = remote.url() {
        trace!("Check to see if we can reach remote");

        match get_url_host(url_string) {
            Ok(socket_data) => {
                let reachable = netcache.get(&socket_data, tcp_check);
                if !reachable {
                    bail!("I can't reach the host: {:?}", &socket_data);
                }
            }
            Err(e) => {
                debug!(
                    "Can't parse url {:?} ({}), assuming git knows what to do...",
                    url_string, e
                );
            }
        }
    }
    trace!("Actually Do Fetch");
    let fetch_result = remote.fetch(&[&remote_branch], None, None);
    Ok(fetch_result?)
}

fn get_url_host(url_string: &str) -> Result<SocketData> {
    let git_url = url::Url::parse(url_string)?;

    if let url::Origin::Tuple(_, host, port) = git_url.origin() {
        return Ok(SocketData { host, port });
    }

    Err(format_err!("can not understand url: {:?}", git_url))
}

fn parse_remote_from_ref(gitref: git2::Reference) -> Result<(String, String)> {
    if gitref.is_remote() {
        gitref
            .name()
            .ok_or_else(|| format_err!("gitref can not be coreced into a string to parse"))
            .and_then(|refspec| {
                let segments = refspec.split('/').collect::<Vec<&str>>();
                if segments.len() >= 4 && segments[0] == "refs" && segments[1] == "remotes" {
                    let x = (segments[2].to_owned(), segments[3].to_owned());
                    trace!("Using remote: {:?}", x);
                    Ok(x)
                } else {
                    bail!("Can not parse refspec: {:?}", refspec);
                }
            })
    } else {
        bail!("git reference is not a remote object");
    }
}


pub fn branch_name(repo: &git2::Repository) -> Option<String> {
    let path = repo.workdir().unwrap();
    let branch = repo.head().ok().and_then(|h| {
        if h.is_branch() {
            let branch = String::from_utf8_lossy(h.shorthand_bytes());
            Some(branch.into())
        } else {
            None
        }
    });

    if branch.is_none() {
        warn!("Excluding detached HEAD: {}", path.display());
    }
    branch
}
