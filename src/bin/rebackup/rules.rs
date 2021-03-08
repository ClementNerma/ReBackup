mod glob_patterns;
mod shell_filters;

use clap::Clap;
use rebackup::WalkerRule;

#[derive(Clap)]
pub struct RulesOpts {
    #[clap(flatten)]
    shell_cmd_filters: shell_filters::ShellCmdFiltersOpts,

    #[clap(flatten)]
    glob_patterns: glob_patterns::GlobPatternsOpts,
}

pub fn make_rules(opts: &RulesOpts) -> Vec<WalkerRule> {
    let mut rules = vec![];

    shell_filters::make_shell_cmd_filters(&opts.shell_cmd_filters, &mut rules);
    glob_patterns::make_pattern_filters(&opts.glob_patterns, &mut rules);

    rules
}
