//! Links the Windows release's icon and version information into the executable: `scripts/release/build`
//! compiles them with llvm-mingw's `windres` into an object and names it in `SPELUNKY_WINDOWS_RESOURCES`.
//! Every other build leaves it unset and gets nothing. Linking the object from here, rather than with a
//! `-C link-arg`, lets cargo see it change: it relinks the app when the version does.

fn main() {
    println!("cargo:rerun-if-env-changed=SPELUNKY_WINDOWS_RESOURCES");
    let Some(object) = std::env::var_os("SPELUNKY_WINDOWS_RESOURCES") else {
        return;
    };
    let object = std::path::PathBuf::from(object);
    println!("cargo:rerun-if-changed={}", object.display());
    println!("cargo:rustc-link-arg-bins={}", object.display());
}
