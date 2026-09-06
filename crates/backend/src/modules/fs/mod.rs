// Transitional re-exports keep older backend-owned call sites compiling while
// feature crates migrate to the standalone filesystem contract.
pub use labonair_filesystem::{file, grep, mutate, paths, search, tree};
pub mod watcher;
