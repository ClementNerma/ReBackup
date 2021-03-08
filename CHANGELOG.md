# CHANGELOG

## Version 1.0.2 (08/03/2021)

* :bug: **Fix (Minor):** Symbolic links were resolved even when not followed, leading to error messages

## Version 1.0.1 (08/03/2021)

* :rocket: Switch from [`std::collections::BTreeSet`](https://doc.rust-lang.org/std/collections/struct.BTreeSet.html) to [`std::collections::HashSet`](https://doc.rust-lang.org/std/collections/struct.HashSet.html) for better performances

## Version 1.0.0 (08/03/2021)

* :gear: Recursive traversal of directories
* :gear: Output is sorted by default
* :gear: Changeable logging level
* :gear: Rules handling
* :gear: Rules can map directories to custom items
* :gear: Handling of symbolic links
* :gear: Detection of already visited items
* :gear: Command-line interface