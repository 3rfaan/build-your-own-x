use std::{
    env,
    fs::{File, OpenOptions},
    io::{self, BufWriter, Write},
    mem,
    path::PathBuf,
};

use super::Shell;

const SINGLE_QUOTES: char = '\'';
const DOUBLE_QUOTES: char = '"';
const NEWLINE: char = '\n';
const BACKSLASH: char = '\\';
const SPACE: char = ' ';
const PROMPT: char = '$';

impl Default for Shell {
    fn default() -> Self {
        Self {
            cmd: String::new(),
            args: Vec::new(),
            writer: BufWriter::new(io::stdout()),
        }
    }
}

impl Shell {
    pub(super) fn parse_input(&mut self, input: &str) {
        let mut chars = input.trim().chars();

        self.cmd = Self::parse_cmd(&mut chars);
        self.args = Self::parse_args(&mut chars);
    }

    fn parse_cmd<I: Iterator<Item = char>>(chars: &mut I) -> String {
        let mut cmd = String::new();

        let mut in_single_quotes = false;
        let mut in_double_quotes = false;

        for c in chars {
            match c {
                SINGLE_QUOTES if !in_double_quotes => Self::toggle_bool(&mut in_single_quotes),
                DOUBLE_QUOTES if !in_single_quotes => Self::toggle_bool(&mut in_double_quotes),
                SPACE if !in_single_quotes && !in_double_quotes => break,
                _ => cmd.push(c),
            }
        }

        cmd
    }

    fn parse_args<I: Iterator<Item = char>>(chars: &mut I) -> Vec<String> {
        let mut args = Vec::new();
        let mut curr_arg = String::new();

        let mut in_single_quotes = false;
        let mut in_double_quotes = false;
        let mut escape_next = false;

        const ESCAPABLE: [char; 4] = [BACKSLASH, PROMPT, DOUBLE_QUOTES, NEWLINE];

        for c in chars {
            if escape_next {
                if in_double_quotes && !ESCAPABLE.contains(&c) {
                    curr_arg.push(BACKSLASH);
                }
                curr_arg.push(c);
                Self::toggle_bool(&mut escape_next);
                continue;
            }

            match c {
                SINGLE_QUOTES if !in_double_quotes => Self::toggle_bool(&mut in_single_quotes),
                DOUBLE_QUOTES if !in_single_quotes => Self::toggle_bool(&mut in_double_quotes),
                BACKSLASH if !in_single_quotes => Self::toggle_bool(&mut escape_next),
                BACKSLASH => curr_arg.push(c),
                SPACE if !in_single_quotes && !in_double_quotes => {
                    Self::save_arg(&mut curr_arg, &mut args)
                }
                _ => curr_arg.push(c),
            }
        }
        Self::save_arg(&mut curr_arg, &mut args);

        args
    }

    fn save_arg(arg: &mut String, args: &mut Vec<String>) {
        if !arg.is_empty() {
            args.push(mem::take(arg));
        }
    }

    fn toggle_bool(b: &mut bool) {
        *b = !*b;
    }

    pub(super) fn handle_redirect(&self) -> io::Result<(Vec<String>, Option<File>, Option<File>)> {
        let mut args = Vec::new();
        let mut stdout_file = None;
        let mut stderr_file = None;

        let mut iter = self.args.iter();
        while let Some(arg) = iter.next() {
            match arg.trim() {
                ">" | "1>" | ">>" | "1>>" => {
                    stdout_file = iter
                        .next()
                        .map(|path| OpenOptions::new().append(true).create(true).open(path))
                        .transpose()?
                }
                "2>" | "2>>" => {
                    stderr_file = iter
                        .next()
                        .map(|path| OpenOptions::new().append(true).create(true).open(path))
                        .transpose()?
                }
                _ => args.push(arg.to_owned()),
            }
        }

        Ok((args, stdout_file, stderr_file))
    }

    pub(super) fn find_exe_in_path(name: &str) -> Option<PathBuf> {
        env::var_os("PATH").map(|paths| {
            env::split_paths(&paths).find_map(|path| {
                let full_path = path.join(name);
                full_path.exists().then_some(full_path)
            })
        })?
    }

    pub(super) fn print_prompt(&mut self) -> io::Result<()> {
        write!(self.writer, "$ ")?;
        self.flush()?;
        Ok(())
    }

    pub(super) fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}
