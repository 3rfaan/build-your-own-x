use std::{
    env,
    io::{self, BufWriter, Write},
    iter::Peekable,
    mem,
    path::PathBuf,
};

use super::Shell;

macro_rules! sh_write {
    ($self:ident,$fmt:expr) => {
        write!($self.writer, $fmt)
    };
    ($self:ident,$fmt:expr,$($args:tt)*) => {
        write!($self.writer, $fmt, $($args)*)
    }
}

macro_rules! sh_writeln {
    ($self:ident,$fmt:expr) => {
        writeln!($self.writer, $fmt)
    };
    ($self:ident,$fmt:expr,$($args:tt)*) => {
        writeln!($self.writer, $fmt, $($args)*)
    }
}

pub(crate) use {sh_write, sh_writeln};

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
        let mut chars = input.trim().chars().peekable();

        self.cmd = self.parse_cmd(&mut chars);
        self.args = self.parse_args(&mut chars);
    }

    fn parse_cmd<I: Iterator<Item = char>>(&mut self, chars: &mut Peekable<I>) -> String {
        let mut cmd = String::new();

        let mut in_single_quotes = false;
        let mut in_double_quotes = false;

        while let Some(&c) = chars.peek() {
            match c {
                SINGLE_QUOTES if !in_double_quotes => Self::toggle_bool(&mut in_single_quotes),
                DOUBLE_QUOTES if !in_single_quotes => Self::toggle_bool(&mut in_double_quotes),
                SPACE if !in_single_quotes && !in_double_quotes => break,
                _ => cmd.push(c),
            }
            chars.next();
        }

        cmd
    }

    fn parse_args<I: Iterator<Item = char>>(&mut self, chars: &mut Peekable<I>) -> Vec<String> {
        let mut args = Vec::new();
        let mut curr_arg = String::new();

        let mut in_single_quotes = false;
        let mut in_double_quotes = false;
        let mut escape_next = false;

        const ESCAPABLE: [char; 4] = [BACKSLASH, PROMPT, DOUBLE_QUOTES, NEWLINE];

        while let Some(&c) = chars.peek() {
            if escape_next {
                if in_double_quotes && !ESCAPABLE.contains(&c) {
                    curr_arg.push(BACKSLASH);
                }

                curr_arg.push(c);
                escape_next = false;
                chars.next();
                continue;
            }

            match c {
                SINGLE_QUOTES if !in_double_quotes => Self::toggle_bool(&mut in_single_quotes),
                DOUBLE_QUOTES if !in_single_quotes => Self::toggle_bool(&mut in_double_quotes),
                BACKSLASH if !in_single_quotes => escape_next = true,
                BACKSLASH => curr_arg.push(c),
                SPACE if !in_single_quotes && !in_double_quotes => {
                    Self::save_arg(&mut curr_arg, &mut args)
                }
                _ => curr_arg.push(c),
            }
            chars.next();
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

    pub(super) fn find_exe_in_path(name: &str) -> Option<PathBuf> {
        env::var_os("PATH").map(|paths| {
            env::split_paths(&paths).find_map(|path| {
                let full_path = path.join(name);
                full_path.exists().then_some(full_path)
            })
        })?
    }

    pub(super) fn print_prompt(&mut self) -> io::Result<()> {
        sh_write!(self, "$ ")?;
        self.flush()?;
        Ok(())
    }

    pub(super) fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}
