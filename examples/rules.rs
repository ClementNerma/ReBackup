//! # Examples
//!
//! This file contains examples on how to write simple to complex rules for ReBackup's [walker](rebackup::walk).

use rebackup::{WalkerItemType, WalkerRule, WalkerRuleResult};
use std::env;
use std::ffi::OsString;
use std::process::Command;

/// Exclude the 'target' directory in Cargo projects
pub fn rust_cargo_build() -> WalkerRule {
    WalkerRule {
        name: "rust_cargo_build",
        description: None,
        only_for: Some(WalkerItemType::Directory),
        matches: Box::new(|path, _, _| path.file_name() == Some(OsString::from("target").as_os_str()) && path.join("..").join("Cargo.toml").is_file()),
        action: Box::new(|_, _, _| Ok(WalkerRuleResult::ExcludeItem)),
    }
}

/// Exclude directories containing a '.nomedia' file
pub fn nomedia() -> WalkerRule {
    WalkerRule {
        name: "nomedia",
        description: None,
        only_for: Some(WalkerItemType::Directory),
        matches: Box::new(|path, _, _| path.join(".nomedia").is_file()),
        action: Box::new(|_, _, _| Ok(WalkerRuleResult::ExcludeItem)),
    }
}

/// Exclude the '.git' directories
pub fn dotgit() -> WalkerRule {
    WalkerRule {
        name: "dotgit",
        description: None,
        only_for: Some(WalkerItemType::Directory),
        matches: Box::new(|path, _, _| path.file_name() == Some(OsString::from(".git").as_os_str())),
        action: Box::new(|_, _, _| Ok(WalkerRuleResult::ExcludeItem)),
    }
}

/// Exclude the 'node_modules' directory
pub fn node_modules() -> WalkerRule {
    WalkerRule {
        name: "node_modules",
        description: None,
        only_for: Some(WalkerItemType::Directory),
        matches: Box::new(|path, _, _| path.file_name() == Some(OsString::from("node_modules").as_os_str())),
        action: Box::new(|_, _, _| Ok(WalkerRuleResult::ExcludeItem)),
    }
}

/// Exclude files based on the '.gitignore' file in Git repositories
pub fn gitignore() -> WalkerRule {
    WalkerRule {
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

            let is_excluded = Command::new("git").arg("check-ignore").arg(dir.to_string_lossy().to_string()).output();

            // Restore the current directory before returning eventual error from the command
            env::set_current_dir(cwd)?;

            if is_excluded?.status.success() {
                Ok(WalkerRuleResult::ExcludeItem)
            } else {
                Ok(WalkerRuleResult::IncludeItem)
            }
        }),
    }
}

fn main() {
    println!("This example is not runnable by itself.");
    println!("Its purpose is to show how to make simple or advanced rules for the walker.");
    println!("Please check the example's source code for more informations");
}
