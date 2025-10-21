//! mkdir - make directories

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::vec::Vec;
use alloc::string::String;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "p")?;
    
    let create_parents = args.has_option('p');
    
    // Need at least one directory argument
    args.require_positional(1)?;
    
    // Create each directory
    for i in 0..args.positional_count() {
        let path = args.get_positional(i).unwrap();
        
        if create_parents {
            create_directory_recursive(path)?;
        } else {
            create_directory(path)?;
        }
    }
    
    Ok(0)
}

fn create_directory(path: &str) -> Result<()> {
    let mut path_bytes = Vec::new();
    path_bytes.extend_from_slice(path.as_bytes());
    path_bytes.push(0);
    
    let result = syscalls::mkdir(&path_bytes, syscalls::S_IRWXU);
    
    if result < 0 {
        Err(Error::from_errno(result))
    } else {
        Ok(())
    }
}

fn create_directory_recursive(path: &str) -> Result<()> {
    // Split path into components
    let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    
    if components.is_empty() {
        return Ok(());
    }
    
    // Build path incrementally
    let mut current_path = String::new();
    let starts_with_slash = path.starts_with('/');
    
    for component in components {
        if starts_with_slash || !current_path.is_empty() {
            current_path.push('/');
        }
        current_path.push_str(component);
        
        // Try to create this directory
        let mut path_bytes = Vec::new();
        path_bytes.extend_from_slice(current_path.as_bytes());
        path_bytes.push(0);
        
        let result = syscalls::mkdir(&path_bytes, syscalls::S_IRWXU);
        
        // Ignore EEXIST error (directory already exists)
        if result < 0 {
            let errno = -result;
            if errno != 17 { // EEXIST
                return Err(Error::from_errno(result));
            }
        }
    }
    
    Ok(())
}
