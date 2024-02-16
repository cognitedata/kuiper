use rustyline::{
    completion::Completer, highlight::Highlighter, Context, Helper, Hinter, Validator,
};

use crate::builtins::BUILT_INS;

#[derive(Hinter, Validator, Helper)]
pub struct KuiperHelper {}

impl KuiperHelper {
    pub fn new() -> Self {
        KuiperHelper {}
    }
}

fn is_separator(c: Option<char>) -> bool {
    match c {
        None | Some(',') | Some(' ') | Some(':') | Some('\n') | Some(')') | Some('(')
        | Some('"') => true,
        Some(_) => false,
    }
}

impl Completer for KuiperHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let (mut low, mut high) = (pos.saturating_sub(1), pos);

        while !is_separator(line.chars().nth(low)) && low > 0 {
            low -= 1;
        }
        if low != 0 {
            low += 1;
        }
        while !is_separator(line.chars().nth(high)) {
            high += 1;
        }

        let word: String = line.chars().skip(low).take(high - low).collect();
        let candidates = BUILT_INS
            .into_iter()
            .filter(|s| s.starts_with(&word))
            .map(String::from)
            .collect();

        Ok((low, candidates))
    }
}

impl Highlighter for KuiperHelper {}
