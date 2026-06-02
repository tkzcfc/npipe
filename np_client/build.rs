fn main() {
    // 让 cargo 追踪 BIN_VERSION 环境变量，值变了就重编译
    println!("cargo:rerun-if-env-changed=BIN_VERSION");
}
