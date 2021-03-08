use clap::Clap;
use rebackup::{WalkerRule, WalkerRuleResult};
use std::process::{Command, Stdio};

#[derive(Clap)]
pub struct ShellCmdFiltersOpts {
    #[clap(short, long, about = "Exclude items when provided commands fail (use REBACKUP_ITEM variable)")]
    pub filter_with: Vec<String>,

    #[clap(long, about = "The binary shell to use for filtering")]
    pub shell: Option<String>,

    #[clap(long, about = "Shell arguments provided before commands", requires = "shell")]
    pub shell_head_args: Vec<String>,

    #[clap(long, about = "Shell arguments provided after commands", requires = "shell")]
    pub shell_tail_args: Vec<String>,

    #[clap(long, about = "Print commands' STDOUT and STDERR")]
    pub display_shell_output: bool,
}

pub fn make_shell_cmd_filters(opts: &ShellCmdFiltersOpts, out: &mut Vec<WalkerRule>) {
    let (shell_path, shell_head_args, shell_tail_args) = if let Some(shell_path) = &opts.shell {
        (shell_path.clone(), opts.shell_head_args.clone(), opts.shell_tail_args.clone())
    } else if cfg!(windows) {
        ("cmd.exe".to_string(), vec!["-C".to_string()], vec![])
    } else {
        ("sh".to_string(), vec!["-c".to_string()], vec![])
    };

    let display_shell_output = opts.display_shell_output;

    for filter in &opts.filter_with {
        let (shell_path, shell_head_args, shell_tail_args) = (shell_path.clone(), shell_head_args.clone(), shell_tail_args.clone());
        let filter = filter.clone();

        out.push(WalkerRule {
            name: "shell-filter",
            description: Some(format!("Command: {}", filter)),
            only_for: None,
            matches: Box::new(|_, _, _| true),
            action: Box::new(move |path, _, _| {
                let output = Command::new(shell_path.clone())
                    .args(&shell_head_args)
                    .arg(&filter)
                    .args(&shell_tail_args)
                    .env("REBACKUP_ITEM", path)
                    .stdout(if display_shell_output { Stdio::inherit() } else { Stdio::null() })
                    .stderr(if display_shell_output { Stdio::inherit() } else { Stdio::null() })
                    .output()?;

                Ok(if output.status.success() {
                    WalkerRuleResult::IncludeItem
                } else {
                    WalkerRuleResult::ExcludeItem
                })
            }),
        });
    }
}
