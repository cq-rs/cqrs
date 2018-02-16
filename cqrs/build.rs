extern crate rustc_version;

use rustc_version::Channel;

fn main() {
    let version = rustc_version::version_meta().unwrap();
    println!("cargo:rerun-if-changed-build=build.rs");
    if version.channel == Channel::Nightly {
        println!("cargo:rustc-cfg=nightly");
    }
}