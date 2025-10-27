//! echo - display a line of text

use crate::args::Args;
use crate::error::Result;
use crate::syscalls;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "ne")?;

    let no_newline = args.has_option('n');
    let interpret_escapes = args.has_option('e');

    // Print each argument separated by spaces
    for i in 0..args.positional_count() {
        if i > 0 {
            syscalls::write(1, b" ");
        }

        let arg = args.get_positional(i).unwrap();

        if interpret_escapes {
            print_with_escapes(arg);
        } else {
            syscalls::write(1, arg.as_bytes());
        }
    }

    // Print newline unless -n is specified
    if !no_newline {
        syscalls::write(1, b"\n");
    }

    Ok(0)
}

fn print_with_escapes(s: &str) {
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            // Escape sequence
            match bytes[i + 1] {
                b'n' => {
                    syscalls::write(1, b"\n");
                }
                b't' => {
                    syscalls::write(1, b"\t");
                }
                b'r' => {
                    syscalls::write(1, b"\r");
                }
                b'\\' => {
                    syscalls::write(1, b"\\");
                }
                _ => {
                    // Unknown escape - print as-is
                    syscalls::write(1, &bytes[i..i + 2]);
                }
            }
            i += 2;
        } else {
            syscalls::write(1, &bytes[i..i + 1]);
            i += 1;
        }
    }
}
