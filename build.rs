#[cfg(feature = "c")]
fn c_build_rs() {
    use std::env;

    use cbindgen::{Builder, Language};

    // TODO: the directory can be read-only, use OUT_DIR
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Builder::new()
    //   .with_crate(crate_dir)
    //   .with_language(Language::C)
    //   .with_parse_expand(&["hello-ffi"])
    //   .with_parse_expand_features(&["c"])
    //   .generate()
    //   .expect("Unable to generate bindings")
    //   .write_to_file("c/bindings.h");
}

fn main() {
    #[cfg(feature = "c")]
    c_build_rs();
}
