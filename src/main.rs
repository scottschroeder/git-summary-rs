extern crate clap;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate walkdir;
extern crate git2;

use clap::{App, Arg};

use std::cmp::max;

mod fs_util;
mod git_util;

const PROJECT_NAME: &str = "git-summary";

type Result<T> = std::result::Result<T, failure::Error>;


fn run() -> Result<()> {
    let args = get_args();
    setup_logger(args.occurrences_of("verbosity"));
    trace!("Args: {:?}", args);

    let path = fs_util::get_working_dir(args.value_of("path"))?;
    debug!("Looking for git repos under {:?}", path);

    let repos =fs_util::get_all_repos(path, args.is_present("deep_lookup"));

    if repos.is_empty() {
        info!("there were no repos found");
        return Ok(())
    }

    let repo_data = repos.into_iter()
        .filter_map(|r| {
            r.to_str().map(|repo_str| {
                git_util::open_repo(&r)
                    .map(|ob| {
                        ob.map(|b| (repo_str.to_owned(), b))
                    })
                    .unwrap_or(None)
            }).unwrap_or(None)
        })
        .collect::<Vec<_>>();

    let mut max_repo_len = 0;
    let mut max_branch_len = 0;

    for (repo, branch) in &repo_data {
        max_repo_len = max(max_repo_len, repo.len());
        max_branch_len = max(max_branch_len, branch.len());
    }

    debug!("Max repo: {}", max_repo_len);
    debug!("Max branch: {}", max_branch_len);
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
        //0 => log::Level::Error,
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
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