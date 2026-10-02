use std::io::{BufRead, Write};

pub trait Prompter {
    fn confirm(&mut self, question: &str, default_yes: bool) -> bool;
    /// Returns the typed line, or `default` (or empty) if the user just pressed Enter.
    fn ask(&mut self, question: &str, default: Option<&str>) -> String;
}

pub struct StdioPrompter;

impl Prompter for StdioPrompter {
    fn confirm(&mut self, question: &str, default_yes: bool) -> bool {
        let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
        print!("▸ {} {} ", question, hint);
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        let _ = std::io::stdin().lock().read_line(&mut line);
        match line.trim().to_lowercase().as_str() {
            "" => default_yes,
            s => s.starts_with('y'),
        }
    }

    fn ask(&mut self, question: &str, default: Option<&str>) -> String {
        match default {
            Some(d) => print!("▸ {} [{}]: ", question, d),
            None => print!("▸ {}: ", question),
        }
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        let _ = std::io::stdin().lock().read_line(&mut line);
        let line = line.trim();
        if line.is_empty() { default.unwrap_or("").to_string() } else { line.to_string() }
    }
}

#[cfg(test)]
pub mod testing {
    use super::Prompter;
    use std::collections::VecDeque;

    pub struct ScriptedPrompter {
        answers: VecDeque<String>,
        pub questions: Vec<String>,
    }

    impl ScriptedPrompter {
        pub fn new(answers: &[&str]) -> Self {
            Self { answers: answers.iter().map(|s| s.to_string()).collect(), questions: Vec::new() }
        }
    }

    impl Prompter for ScriptedPrompter {
        fn confirm(&mut self, question: &str, default_yes: bool) -> bool {
            self.questions.push(question.to_string());
            match self.answers.pop_front().unwrap_or_default().as_str() {
                "" => default_yes,
                s => s.starts_with('y'),
            }
        }

        fn ask(&mut self, question: &str, default: Option<&str>) -> String {
            self.questions.push(question.to_string());
            let a = self.answers.pop_front().unwrap_or_default();
            if a.is_empty() { default.unwrap_or("").to_string() } else { a }
        }
    }
}
