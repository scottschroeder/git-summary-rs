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
extern crate prettytable;

use clap::{App, Arg};
use rayon::prelude::*;

mod fs_util;
mod git_util;
mod net_util;
mod cache;

const PROJECT_NAME: &str = "git-summary";
const THREAD_POOL_SIZE: usize = 50; // its all I/O, so bump it

type Result<T> = std::result::Result<T, failure::Error>;


fn run() -> Result<()> {
    let args = get_args();
    setup_logger(args.occurrences_of("verbosity"));
    trace!("Args: {:?}", args);
    let local_only = args.is_present("local_only");

    rayon::ThreadPoolBuilder::new()
        .num_threads(THREAD_POOL_SIZE)
        .build_global()?;

    let path = fs_util::get_working_dir(args.value_of("path"))?;
    debug!("Looking for git repos under {:?}", path);


    let git_repos = fs_util::get_all_repos(&path, args.is_present("deep_lookup"))
        .collect::<Vec<_>>();

    let netcache = cache::Cache::default();

    let repos = git_repos.par_iter()
        .map(|p| {
            git2::Repository::open(p)
                .map_err(|e| e.into())
                .and_then(|repo| {
                    git_util::branch_name(&repo)
                        .map(|branch_opt| branch_opt.map(|b| (p, repo, b)))
                })
        })
        .filter_map(|res| {
            match res {
                Ok(Some(x)) => Some(Ok(x)),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            }
        })
        .map(|res| {
            res.and_then(|(p, repo, branch)| {
                git_util::summarize_one_git_repo(&repo, !local_only, netcache.clone())
                    .map(|st| (p, branch, st))
            })
        })
        .collect::<Vec<_>>();

    let mut table = prettytable::Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(prettytable::Row::new(vec![
        prettytable::Cell::new("Repositories")
            .with_style(prettytable::Attr::Bold),
        prettytable::Cell::new("Branch")
            .with_style(prettytable::Attr::Bold),
        prettytable::Cell::new("State")
            .with_style(prettytable::Attr::Bold),
    ]));

    for res in repos {
        match res {
            Ok((p, branch, st)) => {
                if !args.is_present("skip_up_to_date") || !st.is_clean() {
                    let repo_name = fs_util::shorten(&path, &p).to_string_lossy();
                    let color = alert_color(&st);
                    table.add_row(prettytable::Row::new(vec![
                        prettytable::Cell::new(&repo_name)
                            .with_style(prettytable::Attr::ForegroundColor(color)),
                        prettytable::Cell::new(&branch)
                            .with_style(prettytable::Attr::ForegroundColor(color)),
                        prettytable::Cell::new(&format!("{}", st))
                            .with_style(prettytable::Attr::ForegroundColor(color)),
                    ]));
                }
            }
            Err(e) => error!("{}", e),
        }
    }
    table.printstd();


    Ok(())
}

fn alert_color(st: &git_util::RepoStatus) -> prettytable::color::Color {
    match st.severity() {
        git_util::RepoSeverity::Clean => prettytable::color::GREEN,
        git_util::RepoSeverity::NeedSync => prettytable::color::YELLOW,
        git_util::RepoSeverity::AheadBehind => prettytable::color::YELLOW,
        git_util::RepoSeverity::Dirty => prettytable::color::RED,
    }
}



fn main() {
    if let Err(e) = run() {
        error!("{} failed!", PROJECT_NAME);
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