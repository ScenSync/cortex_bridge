use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let package_name = env::var("CARGO_PKG_NAME").unwrap();

    let output_file = PathBuf::from(&crate_dir)
        .join("include")
        .join(format!("{}.h", package_name.replace("-", "_")));

    std::fs::create_dir_all(output_file.parent().unwrap()).unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&output_file);

    println!("cargo:rerun-if-changed=src/");
    println!("Generated header file: {:?}", output_file);
}
