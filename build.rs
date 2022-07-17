extern crate cc;
use cc::Build;
use std::env;
#[cfg(unix)]
fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-lib=static=os_linux");
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rerun-if-changed=src/net_ext/os_linux.c");
    println!("cargo:rerun-if-changed=build.rs");

    println!("Out Dir={}", out_dir);
    Build::new()
        .file("src/net_ext/os_linux.c")
        .out_dir(out_dir)
        .compile("libos_linux.a");
}

#[cfg(windows)]
fn main() {}
