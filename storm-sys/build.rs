use std::env;

use std::path::Path;
use std::path::PathBuf;
use std::fs;

fn find_source_files<P: AsRef<Path>>(path: P) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().unwrap().is_file())
        .filter(|entry| {
            if let Some(ext) = entry.path().extension() {
                ext == "c" || ext == "cpp"
            } else {
                false
            }
        })
        .map(|entry| entry.path())
}

fn make_bindings() {
    let target_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let bindings = bindgen::Builder::default()
        .header("StormLib.h")
        .clang_arg("-x")
        .clang_arg("c++")
        .layout_tests(true)
        .whitelist_function("SFile.*")
        .whitelist_function("SList.*")
        .whitelist_function("GetLastError")
        .whitelist_type("T.*")
        .whitelist_var("ERROR.*")
        .whitelist_var("MPQ.*")
        .whitelist_var("SFILE.*")
        .default_enum_style(bindgen::EnumVariation::Rust)
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(target_dir.join("src/storm.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    let target_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_triple = env::var("TARGET").unwrap();

    let mut build = cc::Build::new();

    build
        .extra_warnings(false)
        .warnings(false)
        .define("_7ZIP_ST", None)
        .flag("-w")
        .files(find_source_files("StormLib/src"))
        .files(find_source_files("StormLib/src/libtomcrypt/src/hashes"))
        .files(find_source_files("StormLib/src/libtomcrypt/src/math"))
        .files(find_source_files("StormLib/src/libtomcrypt/src/misc"))
        .files(find_source_files("StormLib/src/libtomcrypt/src/pk/asn1"))
        .files(find_source_files("StormLib/src/libtomcrypt/src/pk/ecc"))
        .files(find_source_files("StormLib/src/libtomcrypt/src/pk/pkcs1"))
        .files(find_source_files("StormLib/src/libtomcrypt/src/pk/rsa"))
        .files(find_source_files("StormLib/src/libtommath"))
        .files(find_source_files("StormLib/src/bzip2"))
        .files(find_source_files("StormLib/src/huffman"))
        .files(find_source_files("StormLib/src/pklib"))
        .files(find_source_files("StormLib/src/jenkins"))
        .file("StormLib/src/zlib/crc32.c")
        .file("StormLib/src/zlib/trees.c")
        .file("StormLib/src/zlib/compress.c")
        .file("StormLib/src/zlib/adler32.c")
        .file("StormLib/src/zlib/inftrees.c")
        .file("StormLib/src/zlib/inffast.c")
        .file("StormLib/src/zlib/deflate.c")
        .file("StormLib/src/zlib/inflate.c")
        .file("StormLib/src/zlib/zutil.c")
        .file("StormLib/src/lzma/C/LzFind.c")
        .file("StormLib/src/lzma/C/LzmaEnc.c")
        .file("StormLib/src/lzma/C/LzmaDec.c")
        .file("StormLib/src/adpcm/adpcm.cpp")
        .file("StormLib/src/sparse/sparse.cpp");

    build.static_flag(true);

    build.compile("storm");

    if target_triple.contains("windows-gnu") && env::var("CERES_NOBUNDLE_CPP").is_ok() {
        println!("cargo:rustc-link-lib=static-nobundle=stdc++");
        println!("cargo:rustc-link-lib=static-nobundle=gcc");
    } else if target_triple.contains("darwin") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}
