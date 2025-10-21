//! Command parser for mello-sh

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Parsed command
#[derive(Debug, Clone)]
pub enum Command {
    /// Simple command with arguments
    Simple {
        args: Vec<String>,
        background: bool,
        redirects: Vec<Redirect>,
    },
    /// Pipeline of commands
    Pipeline {
        commands: Vec<Command>,
        background: bool,
    },
}

/// I/O redirection
#[derive(Debug, Clone)]
pub struct Redirect {
    pub kind: RedirectKind,
    pub target: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RedirectKind {
    Input,      // <
    Output,     // >
    Append,     // >>
}

/// Parse a command line
pub fn parse(line: &str) -> Result<Command, String> {
    let line = line.trim();
    
    // Check for pipeline
    if line.contains('|') {
        parse_pipeline(line)
    } else {
        parse_simple(line)
    }
}

/// Parse a pipeline
fn parse_pipeline(line: &str) -> Result<Command, String> {
    let mut commands = Vec::new();
    let mut background = false;

    // Split by pipe
    let parts: Vec<&str> = line.split('|').collect();
    
    for (i, part) in parts.iter().enumerate() {
        let part = part.trim();
        
        // Check for background on last command
        if i == parts.len() - 1 && part.ends_with('&') {
            background = true;
            let part = part[..part.len() - 1].trim();
            commands.push(parse_simple(part)?);
        } else {
            commands.push(parse_simple(part)?);
        }
    }

    if commands.is_empty() {
        return Err("empty pipeline".into());
    }

    Ok(Command::Pipeline { commands, background })
}

/// Parse a simple command
fn parse_simple(line: &str) -> Result<Command, String> {
    let line = line.trim();
    
    // Check for background
    let (line, background) = if line.ends_with('&') {
        (line[..line.len() - 1].trim(), true)
    } else {
        (line, false)
    };

    // Tokenize
    let tokens = tokenize(line)?;
    
    if tokens.is_empty() {
        return Err("empty command".into());
    }

    // Parse redirects and arguments
    let mut args = Vec::new();
    let mut redirects = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i].as_str() {
            "<" => {
                if i + 1 >= tokens.len() {
                    return Err("missing input file".into());
                }
                redirects.push(Redirect {
                    kind: RedirectKind::Input,
                    target: tokens[i + 1].clone(),
                });
                i += 2;
            }
            ">" => {
                if i + 1 >= tokens.len() {
                    return Err("missing output file".into());
                }
                redirects.push(Redirect {
                    kind: RedirectKind::Output,
                    target: tokens[i + 1].clone(),
                });
                i += 2;
            }
            ">>" => {
                if i + 1 >= tokens.len() {
                    return Err("missing output file".into());
                }
                redirects.push(Redirect {
                    kind: RedirectKind::Append,
                    target: tokens[i + 1].clone(),
                });
                i += 2;
            }
            _ => {
                args.push(tokens[i].clone());
                i += 1;
            }
        }
    }

    if args.is_empty() {
        return Err("empty command".into());
    }

    Ok(Command::Simple {
        args,
        background,
        redirects,
    })
}

/// Tokenize a command line
fn tokenize(line: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if escape {
            current.push(ch);
            escape = false;
            i += 1;
            continue;
        }

        match ch {
            '\\' => {
                escape = true;
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            '<' | '>' if !in_quotes => {
                // Handle redirect operators
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                
                // Check for >>
                if ch == '>' && i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(">>".into());
                    i += 1;
                } else {
                    tokens.push(ch.to_string());
                }
            }
            _ => {
                current.push(ch);
            }
        }

        i += 1;
    }

    if in_quotes {
        return Err("unclosed quote".into());
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}
