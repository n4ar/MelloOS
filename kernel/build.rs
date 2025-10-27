use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Helper function to copy or create placeholder for userspace binaries
    fn handle_userspace_binary(
        binary_path: PathBuf,
        dest_name: &str,
        out_dir: &PathBuf,
    ) -> PathBuf {
        let dest = out_dir.join(dest_name);

        if binary_path.exists() {
            println!("cargo:warning=Found {} at {:?}", dest_name, binary_path);
            fs::copy(&binary_path, &dest)
                .unwrap_or_else(|_| panic!("Failed to copy {}", dest_name));
            println!("cargo:warning=Copied {} to {:?}", dest_name, dest);
        } else {
            println!(
                "cargo:warning={} not found at {:?}, creating empty placeholder",
                dest_name, binary_path
            );
            fs::write(&dest, &[])
                .unwrap_or_else(|_| panic!("Failed to create empty {} placeholder", dest_name));
        }

        dest
    }

    // Handle all userspace binaries
    let init_binary_path = PathBuf::from("userspace/init/target/x86_64-unknown-none/release/init");
    let mello_term_path =
        PathBuf::from("userspace/mello-term/target/x86_64-unknown-none/release/mello-term");
    let mello_sh_path =
        PathBuf::from("userspace/mello-sh/target/x86_64-unknown-none/release/mello-sh");
    let mellobox_path =
        PathBuf::from("userspace/mellobox/target/x86_64-unknown-none/release/mellobox");

    let _init_dest = handle_userspace_binary(init_binary_path, "init_binary.bin", &out_dir);
    let _mello_term_dest =
        handle_userspace_binary(mello_term_path, "mello_term_binary.bin", &out_dir);
    let _mello_sh_dest = handle_userspace_binary(mello_sh_path, "mello_sh_binary.bin", &out_dir);
    let _mellobox_dest = handle_userspace_binary(mellobox_path, "mellobox_binary.bin", &out_dir);

    // Tell cargo to rerun this build script if any userspace binary changes
    println!("cargo:rerun-if-changed=userspace/init/target/x86_64-unknown-none/release/init");
    println!("cargo:rerun-if-changed=userspace/init/src/main.rs");
    println!(
        "cargo:rerun-if-changed=userspace/mello-term/target/x86_64-unknown-none/release/mello-term"
    );
    println!("cargo:rerun-if-changed=userspace/mello-term/src/main.rs");
    println!(
        "cargo:rerun-if-changed=userspace/mello-sh/target/x86_64-unknown-none/release/mello-sh"
    );
    println!("cargo:rerun-if-changed=userspace/mello-sh/src/main.rs");
    println!(
        "cargo:rerun-if-changed=userspace/mellobox/target/x86_64-unknown-none/release/mellobox"
    );
    println!("cargo:rerun-if-changed=userspace/mellobox/src/main.rs");

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
        println!(
            "cargo:rustc-link-arg={}",
            syscall_entry_obj.to_str().unwrap()
        );

        println!(
            "cargo:warning=Syscall entry assembly compiled to {:?}",
            syscall_entry_obj
        );
    }

    println!("cargo:rerun-if-changed=src/arch/x86_64/syscall/entry.S");
}
