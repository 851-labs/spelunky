// Empty, but it has to be there for Cargo to resolve the package's build-dependencies. It names its one input,
// so Cargo compares that file's mtime with the last run's, as it does a crate's sources, rather than fingerprint
// the newest of the package's files exactly: scripts/seed-target's copies of the main checkout's mtimes are a
// microsecond on, which would run it (and rebuild every member's tests) in each new worktree.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
