use std::{env};
use std::path::Path;
use std::io::{Write, BufWriter};
use std::fs;
fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let src_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let formats_in_out_dir = Path::new(&out_dir).join("compact_formats.rs");
    let src_code = Path::new(&src_dir).join("src/proto/compact_formats.rs");
    // Build proto files
    tower_grpc_build::Config::new()
        .enable_server(false)
        .enable_client(true)
        .build(
            &["proto/service.proto", "proto/compact_formats.proto"],
            &["proto"],
        )
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
    println!("cargo:rerun-if-changed=proto/service.proto");
    fs::copy(formats_in_out_dir, src_code);
}
