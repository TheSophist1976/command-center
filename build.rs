fn main() {
    // rust-embed's derive fails to compile if the folder is missing.
    std::fs::create_dir_all("web/dist").expect("create web/dist");
    println!("cargo:rerun-if-changed=web/dist");
    println!("cargo:rerun-if-changed=AGENTS.md");
    println!("cargo:rerun-if-changed=skills");
}
