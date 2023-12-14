extern crate prost_build;
use protoc_prebuilt::init;
use std::env::set_var;
use prost_build::Config;


// https://docs.rs/prost-build/latest/prost_build/
fn main() {
    let (protoc_bin, _) = init("22.0").unwrap();
    println!("protoc_bin: {}", protoc_bin.to_str().unwrap());
    set_var("PROTOC", protoc_bin);

    Config::new()
        .out_dir("src")
        .compile_protos(&["src/pb/protos.proto"], &["src/pb"])
        .unwrap();
}