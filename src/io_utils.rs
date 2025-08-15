use std::io::{self, Write};

/// Prompt the user with the given prompt and return true if they respond with "y"
pub fn confirm(prompt: &str, default: bool) -> bool {
    if default { return true;}
    loop {
        print!("{prompt} (y/N): ");
        // flush stdout so the prompt shows up immediately
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "y" => return true,
            _ => return false,
        }
    }
}