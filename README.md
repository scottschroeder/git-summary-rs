# git-summary
Scan a directory recursively for git repos, and print the status of each.

[![dependency status](https://deps.rs/repo/github/scottschroeder/git-summary-rs/status.svg)](https://deps.rs/repo/github/scottschroeder/git-summary-rs)

## History
- Inspired from https://github.com/albenik/git-summary
- Which essentially came from: https://github.com/MirkoLedda/git-summary
- Which was forked from https://gitlab.com/lordadamson/git-summary
- Which all started with this Gist https://gist.github.com/mzabriskie/6631607

## Why re-write?

I really liked this project, but I had quite a few repos in odd states (detached HEAD),
and the bash script error handling left something to be desired. I also wanted to make
tweaks to some of the behavior, and formatting.

## Usage

```
git-summary
Runs a "git status" like operation in an entire directory tree.
Status Legend:
 ? - Untracked files
 + - Uncommitted new files
 M - Modified files
 ^ - Your branch is ahead of upstream
 v - Your branch is behind of upstream
 X - Issue attempting to fetch from upstream
USAGE:
    git-summary [FLAGS] [OPTIONS] [path]
FLAGS:
    -v               Sets the level of verbosity (-v warn, -vv info, -vvv debug, -vvvv trace)
    -l, --list       Just print a list of all git repos
    -q, --quiet      Print nothing for repos that are up to date.
    -f, --fetch      Perform a 'git fetch' in each repo before checking for unpushed/unpulled commits.
        --hidden     Check for git repos in hidden directories
        --shallow    Only search the directory provided, do NOT recurse.
    -h, --help       Prints help information
    -V, --version    Prints version information
OPTIONS:
        --parallel <parallel>    Max number of workers
ARGS:
    <path>    Path to folder containing git repos; if omitted, the current working directory is used
```

