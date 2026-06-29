use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    // workspace root is two levels up: crates/ark-resolve -> crates -> root
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("cannot find workspace root from CARGO_MANIFEST_DIR");

    let prelude_path = workspace_root.join("std").join("prelude.ark");
    if !prelude_path.exists() {
        panic!(
            "std/prelude.ark not found at {:?}. Ensure the file exists in the workspace root.",
            prelude_path
        );
    }

    println!(
        "cargo:rustc-env=ARK_PRELUDE_PATH={}",
        prelude_path.display()
    );
    // Re-run this script if prelude.ark changes
    println!("cargo:rerun-if-changed={}", prelude_path.display());
}
