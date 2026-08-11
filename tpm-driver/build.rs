use std::env;
use std::path::PathBuf;
use std::process::Command;


fn main() {
    let source_dir = env::var("TPM_DRIVER_SOURCE_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}", source_dir);
    println!("cargo:rustc-link-lib=tpm_driver");
    let bindings = bindgen::Builder::default()
        .header("./header/tpm_lib_header.h")
        .clang_arg("--target=riscv64-unknown-kernel")
        //.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .use_core()
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("BIND_DIR").unwrap());
    bindings.write_to_file(out_path.join("bindings.rs")).expect("Couldn't write bindings!");

    let mut mv_binding_to_interface_cmd = Command::new("cp");
    mv_binding_to_interface_cmd.args([out_path.join("bindings.rs"), "src/tpm_interface.rs".into()]);
    if !mv_binding_to_interface_cmd.status().unwrap().success() {
        panic!("Move of generated bindings to tpm_interface mod failed");
    }

    let mut patch_interface_cmd = Command::new("patch");
    patch_interface_cmd.args(["src/tpm_interface.rs","../patches/tpm_interface.patch"]);
    if !patch_interface_cmd.status().unwrap().success() {
        panic!("Patching tpm_interface.rs failed");
    }
}

