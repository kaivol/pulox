use std::env;
use std::path::PathBuf;

fn main() {
    let matlab_root = r#"C:\Program Files\MATLAB\R2022a\"#;
    let matlab_lib = format!("{}{}", matlab_root, "extern\\lib\\win64\\microsoft");
    let matlab_include = format!("{}{}", matlab_root, "extern\\include");

    println!("cargo:rustc-link-search={matlab_lib}");
    println!("cargo:rustc-link-lib=libmex");
    println!("cargo:rustc-link-lib=libmx");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .clang_arg(format!(r#"-I{}"#, matlab_include))
        .size_t_is_usize(true)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
