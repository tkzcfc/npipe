fn main() {
    if let Ok(version) = std::env::var("BIN_VERSION") {
        println!("cargo:rustc-env=BIN_VERSION={version}");
    }
}
