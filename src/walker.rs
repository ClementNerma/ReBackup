//! # The walker module
//!
//! This module contains the [walker](walk), which is the algorithm used to traverse filesystem items
//! in order to build the files list.

use crate::config::{WalkerConfig, WalkerRule, WalkerRuleResult};
use crate::WalkerItemType;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Walk through a directory (recursively) to build a list of files to backup
///
/// ## Path conversion
///
/// The provided directory will be canonicalized, which means all symbolic links will be resolved first.
///
/// ## Rules execution order
///
/// The provided rules are applied on each item in order.
///
/// ## Traversal order
///
/// Traversal is performed up-to-down, in the order provided by the result of [`std::fs::read_dir`].
///
/// ## Error handling
///
/// If an error occurs (I/O error or if a rule fails), the files list won't be built and a [`WalkerErr`] value will be returned instead.
pub fn walk(dir: &Path, config: &WalkerConfig) -> Result<Vec<PathBuf>, WalkerErr> {
    let dir = fs::canonicalize(dir).map_err(|err| WalkerErr::FailedToCanonicalize(dir.to_path_buf(), err))?;

    if !dir.is_dir() {
        err!("Input directory not found: {}", dir.display());
        return Err(WalkerErr::DirNotFound);
    }

    let mut history = HashSet::new();
    history.insert(dir.clone());

    walk_nested(&dir, config, &dir, &mut history)
}

/// (Internal) Walk through a directory (recursively) to build a list of files to backup
///
/// Provided directory path must be canonicalized and guaranteed to be a directory.
fn walk_nested(dir: &Path, config: &WalkerConfig, canonicalized_source: &Path, history: &mut HashSet<PathBuf>) -> Result<Vec<PathBuf>, WalkerErr> {
    debug!("Walking into directory: {}", dir.display());

    let mut items = vec![];
    let mut contains_items = false;

    // Iterate through all items inside the provided directory
    for item in fs::read_dir(dir).map_err(WalkerErr::FailedToWalkDir)? {
        let item = item.map_err(WalkerErr::FailedToReadDirEntry)?;
        walk_item(item.path(), config, canonicalized_source, history, &mut items)?;

        contains_items = true;
    }

    if !contains_items && !config.drop_empty_dirs {
        items.push(dir.to_path_buf());
    }

    Ok(items)
}

/// (Internal) Run the walker on a single item
fn walk_item(
    item_path: PathBuf,
    config: &WalkerConfig,
    canonicalized_source: &Path,
    history: &mut HashSet<PathBuf>,
    items: &mut Vec<PathBuf>,
) -> Result<(), WalkerErr> {
    // Get the item's metadata
    let item_metadata = item_path
        .symlink_metadata()
        .map_err(|err| WalkerErr::FailedToGetItemMetadata(item_path.clone(), err))?;

    // Determine the item's type
    let item_type = item_metadata.file_type();
    let item_type = if item_type.is_symlink() {
        WalkerItemType::Symlink
    } else if item_type.is_file() {
        WalkerItemType::File
    } else if item_type.is_dir() {
        WalkerItemType::Directory
    } else {
        unreachable!("Internal error: unknown file type at path: {}", item_path.display());
    };

    debug!("> Treating item: {}", item_path.display());

    // Ensure items are not treated twice
    if !history.insert(item_path.clone()) {
        err!("Item was already walked on, skippping it: {}", item_path.display());
        return Ok(());
    }

    // If asked to, ignore symbolic links
    if item_type == WalkerItemType::Symlink {
        if !config.follow_symlinks {
            debug!(">> Detected symlink, skipping based on configuration.");
            return Ok(());
        }

        let sym_target = fs::read_link(&item_path).map_err(|err| WalkerErr::FailedToReadSymlinkTarget(item_path.clone(), err))?;

        if history.contains(&sym_target) {
            err!("Symlink target was already walked on, skipping it: {}", item_path.display());
            return Ok(());
        }

        debug!(">> Detected symlink, following it based on configuration.");
    }

    // Canonicalize the path
    let canonicalized = fs::canonicalize(&item_path).map_err(|err| WalkerErr::FailedToCanonicalize(item_path.clone(), err))?;

    if item_path != canonicalized && !history.insert(canonicalized.clone()) {
        err!(
            "Symbolic link was already walked on, skippping it: {} => {}",
            item_path.display(),
            canonicalized.display()
        );
        return Ok(());
    }

    // Run all rules
    for rule in &config.rules {
        let applies_to_type = match rule.only_for {
            None => true,
            Some(only_type) => item_type == only_type,
        };

        // If applicable and matching, run the rule and check if it indicates to skip the current item
        if applies_to_type && (rule.matches)(&item_path, config, canonicalized_source) {
            match run_walker_rule(&item_path, item_type, config, canonicalized_source, rule)? {
                WalkerRuleDo::Nothing => {}
                WalkerRuleDo::SkipFollowingRules => break,
                WalkerRuleDo::SkipItem => return Ok(()),
                WalkerRuleDo::MapItem(mut mapped_items, absolute) => {
                    debug!(">>> Rule mapped to items (items = {}, absolute = {})", mapped_items.len(), absolute);

                    if absolute {
                        items.append(&mut mapped_items);
                    } else {
                        for item in mapped_items {
                            walk_item(item, config, canonicalized_source, history, items)?;
                        }
                    }

                    return Ok(());
                }
            }
        }
    }

    // Handle the item type
    if item_path.is_dir() {
        items.append(&mut walk_nested(&item_path, config, canonicalized_source, history)?);
    } else {
        items.push(item_path);
    }

    Ok(())
}

/// (Internal) Run a walker rule on an item
fn run_walker_rule(
    item_path: &Path,
    item_type: WalkerItemType,
    config: &WalkerConfig,
    canonicalized_source: &Path,
    rule: &WalkerRule,
) -> Result<WalkerRuleDo, WalkerErr> {
    // Get the rule's plain description
    let rule_description = || rule.description.clone().unwrap_or_else(|| "<no rule description>".to_string());

    debug!(
        ">> Running walker rule '{}' ({}) on item path: {}",
        rule.name,
        rule_description(),
        item_path.display()
    );

    // Create an error value from a rule's failure
    let rule_failed = |err: WalkerRuleErr| WalkerErr::RuleFailedToRun {
        rule_name: rule.name,
        rule_description: rule_description(),
        item_path: item_path.to_path_buf(),
        err,
    };

    // Run the rule and get its result
    let rule_result = (rule.action)(&item_path, config, canonicalized_source)
        .map_err(WalkerRuleErr::Io)
        .map_err(rule_failed)?;

    debug!(">> Rule returned response: {:?}", rule_result);

    match rule_result {
        // Rule failed with an error message
        WalkerRuleResult::StrError(err) => Err(rule_failed(WalkerRuleErr::Str(err))),

        // Rule indicated it should be skipped
        WalkerRuleResult::SkipRule => Ok(WalkerRuleDo::Nothing),

        // Rule indicated to include the item it was applied on
        WalkerRuleResult::IncludeItem => Ok(WalkerRuleDo::Nothing),

        // Rule indicated to include the item it was applied on and to ignore all following rules
        WalkerRuleResult::IncludeItemAbsolute => Ok(WalkerRuleDo::SkipFollowingRules),

        // Rule indicated to exclude the item it was applied on
        WalkerRuleResult::ExcludeItem => Ok(WalkerRuleDo::SkipItem),

        // Rule indicated to map the item it was applied on to a specific list of items
        WalkerRuleResult::MapAsList(paths, absolute) => {
            if item_type == WalkerItemType::File {
                return Err(WalkerErr::RuleMappedFileAsDir {
                    rule_name: rule.name,
                    rule_description: rule_description(),
                    item_path: item_path.to_path_buf(),
                });
            }

            let mut mapped_items = Vec::with_capacity(paths.len());

            for mut mapped_item_path in paths {
                if !mapped_item_path.is_absolute() {
                    mapped_item_path = item_path.join(mapped_item_path)
                }

                if !mapped_item_path.ancestors().any(|ancestor| ancestor == item_path) {
                    return Err(WalkerErr::RuleMappingContainsExternalItem {
                        rule_name: rule.name,
                        rule_description: rule_description(),
                        item_path: item_path.to_path_buf(),
                        mapped_item_path,
                    });
                }

                if !mapped_item_path.exists() {
                    return Err(WalkerErr::RuleMappingContainsNonExistingItem {
                        rule_name: rule.name,
                        rule_description: rule_description(),
                        item_path: item_path.to_path_buf(),
                        mapped_item_path,
                    });
                }

                mapped_items.push(mapped_item_path);
            }

            Ok(WalkerRuleDo::MapItem(mapped_items, absolute))
        }
    }
}

/// (Internal) Action to perform after a specific rule ended
enum WalkerRuleDo {
    /// Do nothing
    Nothing,

    /// Skip all following rules
    SkipFollowingRules,

    /// Skip this item
    SkipItem,

    /// Map this item as a list of paths, also indicating if the mapping is absolute
    MapItem(Vec<PathBuf>, bool),
}

/// Error occured while the [walker](walk) was running
#[derive(Error, Debug)]
pub enum WalkerErr {
    /// Path could not be canonicalized
    #[error("Failed to canonicalize path: {0} ({1})")]
    FailedToCanonicalize(PathBuf, std::io::Error),

    /// (Internal error) Directory provided to the walker was not found
    #[error("Internal: directory provided to walker was not found")]
    DirNotFound,

    /// Failed to walk through a directory ([`std::fs::read_dir`] I/O error)
    #[error("Failed to walk directory: {0}")]
    FailedToWalkDir(std::io::Error),

    /// Failed to read a directory entry ([`std::fs::DirEntry`] I/O error)
    #[error("Failed to read directory entry: {0}")]
    FailedToReadDirEntry(std::io::Error),

    /// Failed to read the target of a symbolic link ([`std::fs::read_link`] I/O error)
    #[error("Failed to read the target of the symbolic link at path: {0} ({1})")]
    FailedToReadSymlinkTarget(PathBuf, std::io::Error),

    /// Failed to get an [item's metadata](std::fs::Metadata)
    #[error("Failed to get metadata from an item at path: {0} ({1})")]
    FailedToGetItemMetadata(PathBuf, std::io::Error),

    /// A [rule](WalkerRule) failed to run
    #[error("Rule '{rule_name}' ({rule_description}) failed to execute: {err} (on item: {item_path})")]
    RuleFailedToRun {
        rule_name: &'static str,
        rule_description: String,
        item_path: PathBuf,
        err: WalkerRuleErr,
    },

    /// A rule mapped a file as a directory (see [`WalkerRuleResult::MapAsList`]
    #[error("Rule '{rule_name}' ({rule_description}) mapped a non-directory item as a directory (path is: {item_path})")]
    RuleMappedFileAsDir {
        rule_name: &'static str,
        rule_description: String,
        item_path: PathBuf,
    },

    /// One of the mapped items returned by a rule is not a sub-item of the base directory
    #[error("Rule '{rule_name}' ({rule_description}) mapped directory '{item_path}' as a list containing external item: {mapped_item_path}")]
    RuleMappingContainsExternalItem {
        rule_name: &'static str,
        rule_description: String,
        item_path: PathBuf,
        mapped_item_path: PathBuf,
    },

    /// A rule mapped a file as a directory (see [`WalkerRuleResult::MapAsList`]
    #[error("Rule '{rule_name}' ({rule_description}) mapped directory '{item_path}' as a list containing inexisting item: {mapped_item_path}")]
    RuleMappingContainsNonExistingItem {
        rule_name: &'static str,
        rule_description: String,
        item_path: PathBuf,
        mapped_item_path: PathBuf,
    },
}

/// Error caused by a walker rule (see [`WalkerRule`])
#[derive(Debug)]
pub enum WalkerRuleErr {
    Io(std::io::Error),
    Str(String),
}

impl fmt::Display for WalkerRuleErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{}", err),
            Self::Str(err) => write!(f, "{}", err),
        }
    }
}
