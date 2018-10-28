extern crate clap;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate walkdir;
extern crate git2;
extern crate url;
extern crate rayon;

use clap::{App, Arg};
use rayon::prelude::*;

use std::cmp::max;
use std::result::Result as StdResult;
use std::path::Path;

mod fs_util;
mod git_util;
mod net_util;

use git_util::RepoStatus;

const PROJECT_NAME: &str = "git-summary";
const STATE_WIDTH: usize = 5; // max("State".len(), flags)

type Result<T> = std::result::Result<T, failure::Error>;


struct LinePrinter {
    max_repo_len: usize,
    max_branch_len: usize,
}

fn pad_equals(len: usize) -> String {
    unsafe {
        String::from_utf8_unchecked(vec![b'='; len])
    }
}

impl LinePrinter {
    fn max_repo_len(&self) -> usize {
        max("Repository".len(), self.max_repo_len)
    }
    fn max_branch_len(&self) -> usize {
        max("Branch".len(), self.max_branch_len)
    }
    pub fn print_header(&self) {
        println!("{:rwidth$} {:bwidth$} {:swidth$}",
                 "Repository",
                 "Branch",
                 "State",
                 rwidth = self.max_repo_len(),
                 bwidth = self.max_branch_len(),
                 swidth = STATE_WIDTH,
        );
        println!("{} {} {}",
                 pad_equals(self.max_repo_len()),
                 pad_equals(self.max_branch_len()),
                 pad_equals(STATE_WIDTH),
        )
    }
    pub fn print_repo(&self, name: &Path, branch: &str, status: &RepoStatus) {
        println!("{:rwidth$} {:bwidth$} {:swidth$}",
                 name.display(),
                 branch,
                 status,
                 rwidth = self.max_repo_len(),
                 bwidth = self.max_branch_len(),
                 swidth = STATE_WIDTH,
        );
    }
}


struct Repo<'a> {
    git_repo: &'a git2::Repository,
    short_path: &'a Path,
    branch_name: String,
}

//trait TestMe: Send {}
//
//impl<'a> TestMe for Repo<'a> {}


fn run() -> Result<()> {
    let args = get_args();
    setup_logger(args.occurrences_of("verbosity"));
    trace!("Args: {:?}", args);

    let path = fs_util::get_working_dir(args.value_of("path"))?;
    debug!("Looking for git repos under {:?}", path);


    let git_repos = fs_util::get_all_repos_iter(&path, args.is_present("deep_lookup"))
        .map(|p| {
            (git2::Repository::open(&p), p)
        })
        .collect::<Vec<_>>();


    let repo_data = git_repos.iter()
        .filter_map(|(r, p)| {
            match r {
                Ok(repo) => Some((repo, p.as_path())),
                Err(e) => {
                    error!("Failure to load repo from {}: {}", p.display(), e);
                    None
                }
            }
        })
        .filter_map(|(r, p)| {
            match r.head() {
                Ok(head) => Some((r, head, p)),
                Err(e) => {
                    error!("Unable to read HEAD from repo {}: {}", p.display(), e);
                    None
                }
            }
        })
        .filter_map(|(r, h, p)| {
            if h.is_branch() {
                h.shorthand()
                    .map(|b| {
                        Repo {
                            git_repo: r,
                            short_path: fs_util::shorten(&path, &p),
                            branch_name: b.to_owned(),
                        }
                    })
                    .or_else(|| {
                        error!("branch name was not valid UTF-8: {}", p.display());
                        None
                    })
            } else {
                warn!("Excluding detached HEAD: {}", p.display());
                None
            }
        })
        .collect::<Vec<_>>();

    let mut max_repo_len = 0usize;
    let mut max_branch_len = 0usize;

    for r in &repo_data {
        debug!("{} {}", r.short_path.display(), &r.branch_name);
        max_repo_len = max(max_repo_len, r.short_path.as_os_str().len());
        max_branch_len = max(max_branch_len, r.branch_name.len());
    }

    debug!("Max repo: {}", max_repo_len);
    debug!("Max branch: {}", max_branch_len);

    let pretty_printer = LinePrinter {
        max_repo_len,
        max_branch_len,
    };

    pretty_printer.print_header();

    let results = repo_data.into_iter()
        .map(|r| {
            let st = git_util::summarize_one_git_repo(
                r.git_repo,
                !args.is_present("local_only"),
            );
            (r, st)
        })
        .collect::<Vec<_>>();

    for (r, rst) in results {
        let st = rst?;
        if !args.is_present("skip_up_to_date") || !st.is_clean() {
            pretty_printer.print_repo(r.short_path, &r.branch_name, &st)
        }
    }


    Ok(())
}


fn main() {
    if let Err(e) = run() {
        error!("git-summary failed!");
        for cause in e.iter_chain() {
            error!("cause: {}", cause)
        }
    }
}

fn get_args() -> clap::ArgMatches<'static> {
    App::new(PROJECT_NAME)
        .setting(clap::AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::with_name("verbosity")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        ).arg(
        Arg::with_name("local_only")
            .short("l")
            .help("Local operation only. Without this the script runs \"git fetch\" in each repo before checking for unpushed/unpulled commits."),
    ).arg(
        Arg::with_name("deep_lookup")
            .short("d")
            .help("Deep lookup. Will search within the entire tree of the current folder."),
    ).arg(
        Arg::with_name("skip_up_to_date")
            .short("q")
            .help("Print nothing for repos that are up to date."),
    ).arg(
        Arg::with_name("path")
            .index(1)
            .help("Path to folder containing git repos; if omitted, the current working directory is used."),
    ).get_matches()
}


fn setup_logger(level: u64) {
    let mut builder = pretty_env_logger::formatted_builder().unwrap();
    let noisy_modules = &[
        "hyper",
        "mio",
        "tokio_core",
        "tokio_reactor",
        "tokio_threadpool",
        "want",
    ];
    let log_level = match level {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    if level > 1 && level < 4 {
        for module in noisy_modules {
            builder.filter_module(module, log::LevelFilter::Info);
        }
    }
    builder.filter_level(log_level);
    builder.init();
}