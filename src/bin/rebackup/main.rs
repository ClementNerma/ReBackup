//! ReBackup's entrypoint binary

#![forbid(unsafe_code)]
#![forbid(unused_must_use)]

mod rules;

use atomic::Ordering;
use clap::{crate_authors, crate_description, crate_name, crate_version, Clap};
use rebackup::*;
use rules::{make_rules, RulesOpts};
use std::fs;
use std::path::PathBuf;

#[derive(Clap)]
#[clap(name = crate_name!(), version = crate_version!(), about = crate_description!(), author = crate_authors!())]
pub struct Opts {
    #[clap(about = "Source directory")]
    pub source: PathBuf,

    #[clap(short, long, about = "Output file (will print to STDOUT if empty)")]
    pub output: Option<PathBuf>,

    #[clap(short, long, about = "Output absolute paths (default is relative)")]
    pub absolute: bool,

    #[clap(short, long, about = "Prefix all output lines with a specific string")]
    pub prefix: Option<String>,

    #[clap(long, about = "Don't sort the items by path")]
    pub no_sort: bool,

    #[clap(
        long,
        about = "Convert invalid UTF-8 filenames to lossy filenames (this may cause problems with custom commands)"
    )]
    pub allow_non_utf8_filenames: bool,

    #[clap(short, long, about = "Don't backup items with invalid UTF-8 filenames")]
    pub ignore_non_utf8_filenames: bool,

    #[clap(short = 's', long, about = "Follow symbolic links")]
    pub follow_symlinks: bool,

    #[clap(long, about = "Drop empty directories")]
    pub drop_empty_dirs: bool,

    #[clap(short, long, about = "Display debug informations")]
    pub verbose: bool,

    #[clap(flatten)]
    pub rules: RulesOpts,

    #[clap(long, about = "Simulate the listing without priting / writing the actual files list (useful for debugging)")]
    pub dry_run: bool,
}

fn main() {
    let opts = Opts::parse();

    if opts.verbose {
        LOGGER_LEVEL.store(LoggerLevel::Debug, Ordering::SeqCst);
    } else if opts.output.is_none() {
        // Prevent STDOUT from being polluated with messages when the files list is output to it
        LOGGER_LEVEL.store(LoggerLevel::Error, Ordering::SeqCst);
    }

    if !opts.source.is_dir() {
        fail!(exit 2, "Source directory was not found at path: {}", opts.source.display());
    }

    info!("Building files list...");

    let source = fs::canonicalize(&opts.source)
        .unwrap_or_else(|err| fail!(exit 2, "Failed to canonicalize source directory: {} (from path {})", err, opts.source.display()));

    let items = walk(
        &source,
        &WalkerConfig {
            rules: make_rules(&opts.rules),
            follow_symlinks: opts.follow_symlinks,
            drop_empty_dirs: opts.drop_empty_dirs,
        },
    )
    .unwrap_or_else(|err| fail!(exit 3, "Failed to build files list: {}", err));

    debug!("Converting filenames...");

    // Convert the files list to filenames
    let mut out = vec![];

    for mut path in items {
        if !opts.absolute {
            path = path
                .strip_prefix(&source)
                .unwrap_or_else(
                    |err| fail!(exit 3, "Internal: cannot strip prefix from item '{}' with source '{}': {}", path.display(), source.display(), err),
                )
                .to_path_buf();
        }

        let mut path_str = match path.to_str() {
            Some(str) => str.to_string(),
            None => {
                let lossy_path = path.display().to_string();

                if opts.allow_non_utf8_filenames {
                    debug!("> Converting invalid UTF-8 item to lossy item name: {}", lossy_path);
                    lossy_path
                } else if opts.ignore_non_utf8_filenames {
                    err!("> Found invalid UTF-8 name: {}", lossy_path);
                    continue;
                } else {
                    fail!(exit 4, "> Found invalid UTF-8 name: {}", lossy_path);
                }
            }
        };

        if let Some(prefix) = &opts.prefix {
            path_str = format!("{}{}", prefix, path_str);
        }

        out.push(path_str);
    }

    if !opts.no_sort {
        out.sort();
    }

    let out = out.join("\n");

    // Output the result
    if !opts.dry_run {
        match &opts.output {
            Some(dest) => fs::write(dest, out).unwrap_or_else(|err| fail!(exit 5, "Failed to write output file: {}", err)),
            None => println!("{}", out),
        }
    }

    debug!("Done!");
}
