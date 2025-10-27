//! Simple getopt-style argument parser for mellobox utilities

#![allow(dead_code)]

use crate::error::{Error, Result};
extern crate alloc;
use alloc::vec::Vec;

/// Parsed command-line arguments
pub struct Args {
    /// Program name (argv[0])
    pub program: &'static str,
    /// Parsed options
    options: Vec<(char, Option<&'static str>)>,
    /// Positional arguments
    positional: Vec<&'static str>,
    /// Current position for iteration
    current: usize,
}

impl Args {
    /// Parse command-line arguments
    ///
    /// # Arguments
    /// * `argv` - Array of argument strings (including program name)
    /// * `optstring` - Option specification string (e.g., "abc:d::" for -a, -b, -c arg, -d [arg])
    ///   - Single letter: flag option (no argument)
    ///   - Letter followed by ':': option requires argument
    ///   - Letter followed by '::': option has optional argument
    pub fn parse(argv: &'static [&'static str], optstring: &str) -> Result<Self> {
        if argv.is_empty() {
            return Err(Error::InvalidArgument);
        }

        let program = argv[0];
        let mut options = Vec::new();
        let mut positional = Vec::new();
        let mut i = 1;

        while i < argv.len() {
            let arg = argv[i];

            // Check if this is an option
            if arg.starts_with('-') && arg.len() > 1 && arg != "--" {
                // Handle long option terminator
                if arg == "--" {
                    i += 1;
                    break;
                }

                // Parse short options
                let chars: Vec<char> = arg.chars().skip(1).collect();
                let mut j = 0;

                while j < chars.len() {
                    let opt = chars[j];

                    // Find option in optstring
                    if let Some(pos) = optstring.find(opt) {
                        let requires_arg = optstring.as_bytes().get(pos + 1) == Some(&b':');
                        let optional_arg = optstring.as_bytes().get(pos + 1) == Some(&b':')
                            && optstring.as_bytes().get(pos + 2) == Some(&b':');

                        if requires_arg && !optional_arg {
                            // Option requires argument
                            let opt_arg = if j + 1 < chars.len() {
                                // Argument is rest of this string
                                let rest: alloc::string::String = chars[j + 1..].iter().collect();
                                // This is a workaround - we need to leak the string
                                // In a real implementation, we'd use a proper string pool
                                Some(alloc::string::String::leak(rest) as &'static str)
                            } else if i + 1 < argv.len() {
                                // Argument is next argv element
                                i += 1;
                                Some(argv[i])
                            } else {
                                return Err(Error::MissingArgument);
                            };

                            options.push((opt, opt_arg));
                            break; // Done with this argv element
                        } else if optional_arg {
                            // Option has optional argument
                            let opt_arg = if j + 1 < chars.len() {
                                // Argument is rest of this string
                                let rest: alloc::string::String = chars[j + 1..].iter().collect();
                                Some(alloc::string::String::leak(rest) as &'static str)
                            } else {
                                None
                            };

                            options.push((opt, opt_arg));
                            break; // Done with this argv element
                        } else {
                            // Flag option (no argument)
                            options.push((opt, None));
                        }
                    } else {
                        return Err(Error::UnknownOption);
                    }

                    j += 1;
                }
            } else {
                // Not an option, it's a positional argument
                positional.push(arg);
            }

            i += 1;
        }

        // Add remaining arguments as positional
        while i < argv.len() {
            positional.push(argv[i]);
            i += 1;
        }

        Ok(Args {
            program,
            options,
            positional,
            current: 0,
        })
    }

    /// Check if an option is present
    pub fn has_option(&self, opt: char) -> bool {
        self.options.iter().any(|(o, _)| *o == opt)
    }

    /// Get option argument (returns None if option not present or has no argument)
    pub fn get_option(&self, opt: char) -> Option<&'static str> {
        self.options
            .iter()
            .find(|(o, _)| *o == opt)
            .and_then(|(_, arg)| *arg)
    }

    /// Get all positional arguments
    pub fn positional(&self) -> &[&'static str] {
        &self.positional
    }

    /// Get number of positional arguments
    pub fn positional_count(&self) -> usize {
        self.positional.len()
    }

    /// Get positional argument at index
    pub fn get_positional(&self, index: usize) -> Option<&'static str> {
        self.positional.get(index).copied()
    }

    /// Require at least n positional arguments
    pub fn require_positional(&self, n: usize) -> Result<()> {
        if self.positional.len() < n {
            Err(Error::MissingArgument)
        } else {
            Ok(())
        }
    }

    /// Require at most n positional arguments
    pub fn require_at_most(&self, n: usize) -> Result<()> {
        if self.positional.len() > n {
            Err(Error::TooManyArguments)
        } else {
            Ok(())
        }
    }

    /// Require exactly n positional arguments
    pub fn require_exactly(&self, n: usize) -> Result<()> {
        if self.positional.len() != n {
            if self.positional.len() < n {
                Err(Error::MissingArgument)
            } else {
                Err(Error::TooManyArguments)
            }
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_flags() {
        let argv = &["ls", "-l", "-a"];
        let args = Args::parse(argv, "la").unwrap();

        assert!(args.has_option('l'));
        assert!(args.has_option('a'));
        assert!(!args.has_option('h'));
        assert_eq!(args.positional_count(), 0);
    }

    #[test]
    fn test_parse_combined_flags() {
        let argv = &["ls", "-la"];
        let args = Args::parse(argv, "la").unwrap();

        assert!(args.has_option('l'));
        assert!(args.has_option('a'));
        assert_eq!(args.positional_count(), 0);
    }

    #[test]
    fn test_parse_option_with_arg() {
        let argv = &["grep", "-n", "pattern", "file"];
        let args = Args::parse(argv, "n:").unwrap();

        assert!(args.has_option('n'));
        assert_eq!(args.get_option('n'), Some("pattern"));
        assert_eq!(args.positional_count(), 1);
        assert_eq!(args.get_positional(0), Some("file"));
    }

    #[test]
    fn test_parse_positional() {
        let argv = &["cat", "file1", "file2"];
        let args = Args::parse(argv, "").unwrap();

        assert_eq!(args.positional_count(), 2);
        assert_eq!(args.get_positional(0), Some("file1"));
        assert_eq!(args.get_positional(1), Some("file2"));
    }
}
