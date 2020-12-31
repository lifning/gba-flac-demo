fn main() {
    println!("cargo:rerun-if-changed=linker_script.ld");
    println!("cargo:rerun-if-changed=rt0.s");

    // Sort out our names
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let rt0_filename = "rt0.o";
    let out_rt0_pathbuf = std::path::Path::new(out_dir.as_str()).join(rt0_filename);
    let name = format!("{}", out_rt0_pathbuf.display());

    // Assemble the file into the OUT_DIR
    let as_output = std::process::Command::new("arm-none-eabi-as")
        .args(&["-o", name.as_str()])
        .arg("-mthumb-interwork")
        .arg("-mcpu=arm7tdmi")
        .arg("rt0.s")
        .output()
        .expect("failed to run arm-none-eabi-as");
    if !as_output.status.success() {
        panic!("{}", String::from_utf8_lossy(&as_output.stderr));
    }

    // Tell the linker to look in OUT_DIR.
    println!("cargo:rustc-link-search={}", out_dir);

    // Note: We do *not* tell the linker to link to anything,
    // the linker script will pull in our object file on its own.
}
