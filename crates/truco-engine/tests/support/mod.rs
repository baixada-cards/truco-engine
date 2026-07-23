use std::path::PathBuf;

pub fn spec_root() -> PathBuf {
    if let Some(path) = std::env::var_os("TRUCO_SPEC_DIR") {
        return PathBuf::from(path);
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("engine crate should live under crates/")
        .parent()
        .expect("crates/ should live under the workspace root")
        .join(".cache")
        .join("truco-spec")
}
