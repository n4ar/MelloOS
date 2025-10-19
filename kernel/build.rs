use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Path to the userspace init binary
    let init_binary_path = PathBuf::from("userspace/init/target/x86_64-unknown-none/release/init");
    
    // Create an empty placeholder if the init binary doesn't exist
    let init_binary_dest = out_dir.join("init_binary.bin");
    
    if init_binary_path.exists() {
        println!("cargo:warning=Found init binary at {:?}", init_binary_path);
        
        // Copy the init binary to the output directory
        fs::copy(&init_binary_path, &init_binary_dest)
            .expect("Failed to copy init binary");
        
        println!("cargo:warning=Copied init binary to {:?}", init_binary_dest);
    } else {
        println!("cargo:warning=Init binary not found at {:?}, creating empty placeholder", init_binary_path);
        
        // Create an empty file as placeholder
        fs::write(&init_binary_dest, &[])
            .expect("Failed to create empty init binary placeholder");
    }
    
    // Tell cargo to rerun this build script if the init binary changes
    println!("cargo:rerun-if-changed=userspace/init/target/x86_64-unknown-none/release/init");
    println!("cargo:rerun-if-changed=userspace/init/src/main.rs");
}
