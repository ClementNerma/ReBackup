use clap::Clap;
use glob::Pattern;
use rebackup::{fail, WalkerRule, WalkerRuleResult};

#[derive(Clap)]
pub struct GlobPatternsOpts {
    #[clap(long, about = "Ignore all following rules when matching")]
    pub include_absolute: Vec<String>,

    #[clap(long, about = "Only include items with a glob pattern")]
    pub include_only: Vec<String>,

    #[clap(short, long, about = "Exclude items with a glob pattern")]
    pub exclude: Vec<String>,
}

pub fn make_pattern_filters(opts: &GlobPatternsOpts, out: &mut Vec<WalkerRule>) {
    fn make_pattern_filter(rule_name: &'static str, action: WalkerRuleResult, pattern: &str, out: &mut Vec<WalkerRule>) {
        let pattern = Pattern::new(pattern).unwrap_or_else(|err| fail!(exit 10, "Invalid pattern provided: {}", err));

        out.push(WalkerRule {
            name: rule_name,
            description: Some(format!("Pattern: {}", pattern)),
            only_for: None,
            matches: Box::new(move |path, _, source| pattern.matches_path(path.strip_prefix(source).unwrap())),
            action: Box::new(move |_, _, _| Ok(action.clone())),
        });
    }

    for pattern in &opts.include_absolute {
        make_pattern_filter("include-pattern-absolute", WalkerRuleResult::IncludeItemAbsolute, pattern, out);
    }

    for pattern in &opts.include_only {
        make_pattern_filter("include-pattern", WalkerRuleResult::IncludeItem, pattern, out);
    }

    for pattern in &opts.exclude {
        make_pattern_filter("exclude-pattern", WalkerRuleResult::ExcludeItem, pattern, out);
    }
}
