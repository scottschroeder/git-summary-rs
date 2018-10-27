# git-summary-rs
Display an overview of many repos

## History
- Inspired from https://github.com/albenik/git-summary
- Which essentially came from: https://github.com/MirkoLedda/git-summary
- Which was origonally forked from https://github.com/lordadamson/git-summary
- Freely distributed under the MIT license. 2018@MirkoLedda

## Why re-write?

When scanning golang projects, the shell version would often print
errors from git (go vendoring tools stripped metadata). Even
after hiding those errors, the table formatting had too much free space.

Also I like rust, and wanted to try porting a shell script over.
