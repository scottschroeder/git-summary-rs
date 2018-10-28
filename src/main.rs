extern crate clap;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate git2;
extern crate pretty_env_logger;
extern crate prettytable;
extern crate rayon;
extern crate url;
extern crate walkdir;

use clap::{App, Arg};
use rayon::prelude::*;

mod cache;
mod fs_util;
mod git_util;
mod net_util;
mod results_table;

const PROJECT_NAME: &str = "git-summary";
const THREAD_POOL_SIZE: usize = 50; // its all I/O, so bump it (and wish we were async)

type Result<T> = std::result::Result<T, failure::Error>;

fn run() -> Result<()> {
    let args = get_args();
    setup_logger(args.occurrences_of("verbosity"));
    trace!("Args: {:?}", args);
    let do_fetch = args.is_present("fetch");

    let pool_size = if let Some(p_str) = args.value_of("parallel") {
        p_str.parse::<usize>().map_err(|e| format_err!("Could not parse {:?} as integer: {}", p_str, e))?
    } else {
        THREAD_POOL_SIZE
    };

    rayon::ThreadPoolBuilder::new()
        .num_threads(pool_size)
        .build_global()?;

    let path = fs_util::get_working_dir(args.value_of("path"))?;
    debug!("Looking for git repos under {:?}", path);

    let git_repos = {
        let mut git_paths = fs_util::get_all_repos(
            &path,
            !args.is_present("shallow"),
            args.is_present("check_hidden"),
        );
        git_paths.sort();
        git_paths.dedup();
        git_paths
    };

    if args.is_present("list_repos") {
        for p in git_repos {
            println!("{}", p.display())
        }
        return Ok(());
    }

    let netcache = cache::Cache::default();

    let mut repos = git_repos
        .par_iter()
        .map(|p| {
            git2::Repository::open(p)
                .map_err(|e| e.into())
                .map(|repo| {
                    git_util::branch_name(&repo).map(|b| (p, repo, b))
                })
        })
        .filter_map(|res| match res {
            Ok(Some(x)) => Some(Ok(x)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        })
        .map(|res| {
            res.and_then(|(p, repo, branch)| {
                git_util::summarize_one_git_repo(&repo, do_fetch, netcache.clone())
                    .map(|st| (p, branch, st))
            })
        })
        .filter_map(|res| match res {
            Ok(x) => Some(x),
            Err(e) => {
                error!("{}", e);
                None
            }
        })
        .collect::<Vec<_>>();

    repos.sort_unstable_by_key(|d| d.0);

    let mut table = results_table::ResultsTable::new();
    info!("Checked {} git repositories.", repos.len());
    for (p, branch, st) in repos {
        if !args.is_present("skip_up_to_date") || !st.is_clean() {
            let repo_name = fs_util::shorten(&path, &p).to_string_lossy();
            table.add_repo(&repo_name, &branch, st);
        }
    }
    table.printstd();

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        error!("{} failed!", PROJECT_NAME);
        for cause in e.iter_chain() {
            error!("cause: {}", cause)
        }
    }
}

const LONG_ABOUT: &str = include_str!("DESCRIPTION");

fn get_args() -> clap::ArgMatches<'static> {
    App::new(PROJECT_NAME)
        .setting(clap::AppSettings::DeriveDisplayOrder)
        .about("Run 'git status' on entire directory tree")
        .long_about(LONG_ABOUT)
        .arg(
            Arg::with_name("verbosity")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity (-v warn, -vv info, -vvv debug, -vvvv trace)"),
        ).arg(
        Arg::with_name("list_repos")
            .short("l")
            .long("list")
            .help("Just print a list of all git repos"),
    ).arg(
        Arg::with_name("skip_up_to_date")
            .short("q")
            .long("quiet")
            .help("Print nothing for repos that are up to date."),
    ).arg(
        Arg::with_name("fetch")
            .short("f")
            .long("fetch")
            .help("Perform a 'git fetch' in each repo before checking for unpushed/unpulled commits."),
    ).arg(
        Arg::with_name("check_hidden")
            .long("hidden")
            .help("Check for git repos in hidden directories"),
    ).arg(
        Arg::with_name("shallow")
            .long("shallow")
            .help("Only search the directory provided, do NOT recurse."),
    ).arg(
        Arg::with_name("parallel")
            .long("parallel")
            .takes_value(true)
            .help("Max number of workers"),
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
