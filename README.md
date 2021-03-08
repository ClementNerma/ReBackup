# ReBackup program

ReBackup is a simple backup program that doesn't actually create backups but instead creates a list of files to backup from a source directory.

It uses a walker to traverse filesystem items, which can be customized through rules (see [`WalkerRule`]).

Its main features are:

* Fast recursive directory traversing
* Powerful rules system to include, exclude or remap items
* Handling of symbolic links (requires to enable an option for the walker)
* Detection of already visited paths
* Command-line interface

ReBackup can be used either:

* As a library (see [`walk`](src/walker.rs))
* As a standalone binary with the `cli` feature

## Library usage

ReBackup only exports one single function which is the Walker: [`walk`](src/walker.rs).

It can be used like this:

```rust
use std::path::PathBuf;
use rebackup::{fail, walk, WalkerConfig};

let source = std::env::args().nth(1)
    .unwrap_or_else(|| fail!(exit 1, "Please provide a source directory"));

// NOTE: This can be shortened to `WalkerConfig::new(vec![])`
//       (expanded here for explanations purpose)
let config = WalkerConfig {
    rules: vec![],
    follow_symlinks: false,
    drop_empty_dirs: false,
};

let files_list = walk(&PathBuf::from(source), &config)
    .unwrap_or_else(|err| fail!(exit 2, "Failed to build the files list: {}", err));

let files_list_str: Vec<_> = files_list
    .iter()
    .map(|item| item.to_string_lossy())
    .collect();

println!("{}", files_list_str.join("\n"));
```

### Rules

You can use powerful rules to configure how the walker behaves.

A rule is defined using [`WalkerRule`], and uses two callbacks:

* One to determine if the rule applies on a specific item
* One to run the rule itself

Here is a basic rule excluding all directories containing `.nomedia` files:

```rust
use rebackup::config::*;

let rule = WalkerRule {
    // Name of the rule
    name: "nomedia",

    // Optional description of the rule
    description: None,

    // The type of items the rule applies to (`None` for all)
    only_for: Some(WalkerItemType::Directory),

    // Check if the rule would match a specific item
    matches: Box::new(|path, _, _| path.join(".nomedia").is_file()),

    // Apply the rule to determine what to do
    action: Box::new(|_, _, _| Ok(WalkerRuleResult::ExcludeItem)),
};
```

You can also build more powerful rules, like excluding files ignored by Git:

```rust
use std::env;
use std::process::Command;
use rebackup::config::*;

let rule = WalkerRule {
    name: "gitignore",
    description: None,
    only_for: None,
    matches: Box::new(|path, _, _| path.ancestors().any(|path| path.join(".git").is_dir())),
    action: Box::new(|dir, _, _| {
        let cwd = env::current_dir()?;

        if dir.is_dir() {
            env::set_current_dir(dir)?;
        } else if let Some(parent) = dir.parent() {
            env::set_current_dir(parent)?;
        }

        let is_excluded = Command::new("git")
            .arg("check-ignore")
            .arg(dir.to_string_lossy().to_string())
            .output();

        // Restore the current directory before returning eventual error from the command
        env::set_current_dir(cwd)?;

        if is_excluded?.status.success() {
            Ok(WalkerRuleResult::ExcludeItem)
        } else {
            Ok(WalkerRuleResult::IncludeItem)
        }
    }),
};
```

You can check more examples of rules in [`examples/rules.rs`](examples/rules.rs).

## Command-line usage

```shell
# Build the list of files to backup, and pipe it to 'tar'
# to create a compressed archive
# Be aware of not creating the archive inside the directory to backup, or the archive
# will be listed as well (you can still exclude it from the results afterwards)
rebackup path_to_backup/ | tar -czf output.tgz -T -

# If you are in another directory, ask for absolute paths instead
# Please note that the archive's content will have absolute paths as well
rebackup path_to_backup/ -a | tar -czf output.tgz -T -

# Using filters to exclude items based on patterns
# Here we're excluding all items ignored by the '.gitignore' file in Git repositories
rebackup path_to_backup/ -f '! git check-ignore "$REBACKUP_ITEM"'

# To also exclude the ".git" folder (using glob pattern):
rebackup path_to_backup/ -f '! git check-ignore "$REBACKUP_ITEM"' -e '**/.git'

# Use an alternate shell:
rebackup path_to_backup/ -f '! git check-ignore "$REBACKUP_ITEM"' --shell zsh --shell-head-args=-c

# To list all available arguments:
rebackup --help
```

## License

This project is released under the [Apache-2.0](LICENSE.md) license terms.