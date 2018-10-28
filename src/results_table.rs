use prettytable::{color, format, Attr, Cell, Row, Table};

use git_util::{RepoSeverity, RepoStatus};

pub struct ResultsTable {
    table: Table,
}

impl ResultsTable {
    pub fn new() -> ResultsTable {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(Row::new(vec![
            Cell::new("Repositories").with_style(Attr::Bold),
            Cell::new("Branch").with_style(Attr::Bold),
            Cell::new("State").with_style(Attr::Bold),
        ]));

        ResultsTable { table }
    }

    pub fn add_repo(&mut self, repo_name: &str, branch: &str, status: RepoStatus) {
        let color = alert_color(&status);
        self.table.add_row(Row::new(vec![
            Cell::new(repo_name).with_style(Attr::ForegroundColor(color)),
            Cell::new(branch).with_style(Attr::ForegroundColor(color)),
            Cell::new(&format!("{}", status)).with_style(Attr::ForegroundColor(color)),
        ]));
    }

    pub fn printstd(&self) -> usize {
        self.table.printstd()
    }
}

fn alert_color(st: &RepoStatus) -> color::Color {
    match st.severity() {
        RepoSeverity::Clean => color::GREEN,
        RepoSeverity::NeedSync => color::YELLOW,
        RepoSeverity::AheadBehind => color::YELLOW,
        RepoSeverity::Dirty => color::RED,
    }
}
