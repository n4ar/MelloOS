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
        fs::copy(&init_binary_path, &init_binary_dest).expect("Failed to copy init binary");

        println!("cargo:warning=Copied init binary to {:?}", init_binary_dest);
    } else {
        println!(
            "cargo:warning=Init binary not found at {:?}, creating empty placeholder",
            init_binary_path
        );

        // Create an empty file as placeholder
        fs::write(&init_binary_dest, &[]).expect("Failed to create empty init binary placeholder");
    }

    // Tell cargo to rerun this build script if the init binary changes
    println!("cargo:rerun-if-changed=userspace/init/target/x86_64-unknown-none/release/init");
    println!("cargo:rerun-if-changed=userspace/init/src/main.rs");

    // Compile AP trampoline assembly
    let trampoline_src = PathBuf::from("src/arch/x86_64/smp/boot_ap.S");
    let trampoline_obj = out_dir.join("boot_ap.o");
    let trampoline_bin = out_dir.join("boot_ap.bin");

    if trampoline_src.exists() {
        println!("cargo:warning=Compiling AP trampoline assembly with GAS");

        // Compile assembly to object file using assembler
        // On macOS, use clang; on Linux, use as
        let output = if cfg!(target_os = "macos") {
            Command::new("clang")
                .args(&[
                    "-target",
                    "x86_64-unknown-none",
                    "-c",
                    "-o",
                    trampoline_obj.to_str().unwrap(),
                    trampoline_src.to_str().unwrap(),
                ])
                .output()
                .expect("Failed to execute clang")
        } else {
            Command::new("as")
                .args(&[
                    "--64",
                    "-o",
                    trampoline_obj.to_str().unwrap(),
                    trampoline_src.to_str().unwrap(),
                ])
                .output()
                .expect("Failed to execute as")
        };

        if !output.status.success() {
            eprintln!(
                "assembler stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            eprintln!(
                "assembler stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            panic!("Failed to compile AP trampoline assembly");
        }

        // Extract .text section to flat binary using objcopy
        let output = Command::new("objcopy")
            .args(&[
                "-O",
                "binary",
                "--only-section=.text",
                trampoline_obj.to_str().unwrap(),
                trampoline_bin.to_str().unwrap(),
            ])
            .output()
            .expect("Failed to execute objcopy");

        if !output.status.success() {
            eprintln!(
                "objcopy stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            eprintln!(
                "objcopy stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            panic!("Failed to extract binary from object file");
        }

        println!(
            "cargo:warning=AP trampoline compiled to {:?}",
            trampoline_bin
        );
    } else {
        // Create empty placeholder if source doesn't exist
        fs::write(&trampoline_bin, &[]).expect("Failed to create empty trampoline binary");
    }

    println!("cargo:rerun-if-changed=src/arch/x86_64/smp/boot_ap.S");

    // Compile user entry trampoline assembly
    let user_entry_src = PathBuf::from("src/arch/x86_64/user_entry.S");
    let user_entry_obj = out_dir.join("user_entry.o");

    if user_entry_src.exists() {
        println!("cargo:warning=Compiling user entry assembly with GAS");

        // Compile assembly to object file using assembler
        let output = if cfg!(target_os = "macos") {
            Command::new("clang")
                .args(&[
                    "-target",
                    "x86_64-unknown-none",
                    "-c",
                    "-o",
                    user_entry_obj.to_str().unwrap(),
                    user_entry_src.to_str().unwrap(),
                ])
                .output()
                .expect("Failed to execute clang")
        } else {
            Command::new("as")
                .args(&[
                    "--64",
                    "-o",
                    user_entry_obj.to_str().unwrap(),
                    user_entry_src.to_str().unwrap(),
                ])
                .output()
                .expect("Failed to execute as")
        };

        if !output.status.success() {
            eprintln!(
                "assembler stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            eprintln!(
                "assembler stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            panic!("Failed to compile user entry assembly");
        }

        // Link the object file with the kernel
        println!("cargo:rustc-link-arg={}", user_entry_obj.to_str().unwrap());

        println!(
            "cargo:warning=User entry assembly compiled to {:?}",
            user_entry_obj
        );
    }

    println!("cargo:rerun-if-changed=src/arch/x86_64/user_entry.S");

    // Compile syscall entry assembly
    let syscall_entry_src = PathBuf::from("src/arch/x86_64/syscall/entry.S");
    let syscall_entry_obj = out_dir.join("syscall_entry.o");

    if syscall_entry_src.exists() {
        println!("cargo:warning=Compiling syscall entry assembly with GAS");

        // Compile assembly to object file using assembler
        let output = if cfg!(target_os = "macos") {
            Command::new("clang")
                .args(&[
                    "-target",
                    "x86_64-unknown-none",
                    "-c",
                    "-o",
                    syscall_entry_obj.to_str().unwrap(),
                    syscall_entry_src.to_str().unwrap(),
                ])
                .output()
                .expect("Failed to execute clang")
        } else {
            Command::new("as")
                .args(&[
                    "--64",
                    "-o",
                    syscall_entry_obj.to_str().unwrap(),
                    syscall_entry_src.to_str().unwrap(),
                ])
                .output()
                .expect("Failed to execute as")
        };

        if !output.status.success() {
            eprintln!(
                "assembler stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            eprintln!(
                "assembler stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            panic!("Failed to compile syscall entry assembly");
        }

        // Link the object file with the kernel
        println!("cargo:rustc-link-arg={}", syscall_entry_obj.to_str().unwrap());

        println!(
            "cargo:warning=Syscall entry assembly compiled to {:?}",
            syscall_entry_obj
        );
    }

    println!("cargo:rerun-if-changed=src/arch/x86_64/syscall/entry.S");
}
