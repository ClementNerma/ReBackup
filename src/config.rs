//! # The configuration module
//!
//! The walker can be configured through [`WalkerConfig`].
//! Rules can be defined using [`WalkerRule`].

use std::path::{Path, PathBuf};

/// Configuration for ReBackup's walker
pub struct WalkerConfig {
    /// List of rules to apply on items
    pub rules: Vec<WalkerRule>,

    /// Should the walker follow symbolic links?
    pub follow_symlinks: bool,

    /// Drop empty directoryes
    pub drop_empty_dirs: bool,
}

/// Create a default configuration from rules
impl WalkerConfig {
    pub fn new(rules: Vec<WalkerRule>) -> Self {
        Self {
            rules,
            follow_symlinks: false,
            drop_empty_dirs: false,
        }
    }
}

/// Walker rule (run on individual items)
///
/// ```
/// use rebackup::config::*;
///
/// let rule = WalkerRule {
///     // Name of the rule
///     name: "nomedia",
///
///     // Optional description of the rule
///     description: None,
///
///     // The type of items the rule applies to (`None` for all)
///     only_for: Some(WalkerItemType::Directory),
///
///     // Check if the rule would match a specific item
///     matches: Box::new(|path, _, _| path.join(".nomedia").is_file()),
///
///     // Apply the rule to determine what to do
///     action: Box::new(|_, _, _| Ok(WalkerRuleResult::ExcludeItem)),
/// };
pub struct WalkerRule {
    /// Rule's name
    pub name: &'static str,

    /// Rule's optional description
    pub description: Option<String>,

    /// Indicate if the rule should only be applied on a specific type of filesystem items
    pub only_for: Option<WalkerItemType>,

    /// Predicate to indicate if the rule should be run on a specific item.
    /// The checking should be as fast as possible, the goal of this callback being to not having as much overhad as `action`.
    ///
    /// Arguments are the item's absolute path, the walker's configuration, as well as the source directory (absolute, canonicalized)
    pub matches: Box<dyn Fn(&Path, &WalkerConfig, &Path) -> bool>,

    /// Action to perform when the rule is applies on a specific item
    ///
    /// Arguments are the item's absolute path, the walker's configuration, as well as the source directory (absolute, canonicalized)
    pub action: Box<dyn Fn(&Path, &WalkerConfig, &Path) -> Result<WalkerRuleResult, std::io::Error>>,
}

/// Walker's item type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkerItemType {
    Directory,
    File,
    Symlink,
}

/// Walker rule's result
#[derive(Debug, Clone)]
pub enum WalkerRuleResult {
    /// Fail with the provided error message
    StrError(String),

    /// Skip this rule - used for edge case where the rule only realizes it shouldn't be run
    /// after starting to perform its action. In general cases the [`WalkerRule::matches`] callback
    /// should be used instead.
    SkipRule,

    /// Include the item the rule was ran on (no effect)
    IncludeItem,

    /// Include the item the rule was ran on and ignore all following rules
    IncludeItemAbsolute,

    /// Exclude the item the rule was ran on
    ExcludeItem,

    /// Don't traverse the item the rule was ran on and instead replace it with a list of provided paths
    /// Paths may either be absolute or relative to the item itself, but they must always be children items
    /// of the base path.
    ///
    /// The second operand indicates if the mapping is absolute, wich means if all following rules should be skipped.
    ///
    /// **NOTE:** This return value is only valid on directories and symbolic links, if will generate an error if used on files.
    ///
    /// **NOTE:** If the return value includes a path that has already been visited, an error will be emitted but the process won't fail.
    ///           It will simply skip the said path and go on to the next item to treat.
    MapAsList(Vec<PathBuf>, bool),
}
