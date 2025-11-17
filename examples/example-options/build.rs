use coral_rs::codegen::options::generate_option_structure;

fn main() {
    let file = "coral-agent.toml";
    println!("cargo:rerun-if-changed={file}");

    let mut out_file = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).to_path_buf();
    out_file.push("coral_options.rs");

    std::fs::write(out_file, generate_option_structure(file)).unwrap();
}
