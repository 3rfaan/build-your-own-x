use std::{
    env,
    fs::{File, OpenOptions},
    io::{self, Write},
    mem,
    path::PathBuf,
};

use super::Shell;

pub const BUILTINS: [&str; 5] = ["cd", "echo", "exit", "pwd", "type"];

const SINGLE_QUOTES: char = '\'';
const DOUBLE_QUOTES: char = '"';
const NEWLINE: char = '\n';
const BACKSLASH: char = '\\';
const SPACE: char = ' ';
const PROMPT: char = '$';

impl Shell {
    pub(super) fn parse_input(&mut self, input: &str) {
        // Iterator over characters of input string
        let mut chars = input.trim().chars();

        self.cmd = Self::parse_cmd(&mut chars); // Parse command as string
        self.args = Self::parse_args(&mut chars); // Parse arguments as vector of strings
    }

    fn parse_cmd<I: Iterator<Item = char>>(chars: &mut I) -> String {
        let mut cmd = String::new();

        let mut in_single_quotes = false;
        let mut in_double_quotes = false;

        for c in chars {
            match c {
                SINGLE_QUOTES if !in_double_quotes => Self::toggle_bool(&mut in_single_quotes),
                DOUBLE_QUOTES if !in_single_quotes => Self::toggle_bool(&mut in_double_quotes),
                // If not inside single quotes or double quotes then we reached the end of
                // command and can start parsing the arguments
                SPACE if !in_single_quotes && !in_double_quotes => break,
                // Store any other character inside `cmd`
                _ => cmd.push(c),
            }
        }
        cmd
    }

    fn parse_args<I: Iterator<Item = char>>(chars: &mut I) -> Option<Vec<String>> {
        // Characters which should be escaped by `\`
        const ESCAPABLE: [char; 4] = [BACKSLASH, PROMPT, DOUBLE_QUOTES, NEWLINE];

        let mut args = Vec::new();
        let mut curr_arg = String::new();

        let mut in_single_quotes = false;
        let mut in_double_quotes = false;
        let mut escape_next = false;

        for c in chars {
            // If `escape_next` is truthy then escape current character
            if escape_next {
                // If inside double quotes or character is not an escapable character then
                // also save `\`...
                if in_double_quotes && !ESCAPABLE.contains(&c) {
                    curr_arg.push(BACKSLASH);
                }
                // ...then save current character
                curr_arg.push(c);
                // `escape_next` is now false as we have escaped current character
                Self::toggle_bool(&mut escape_next);
                // Proceed with next character
                continue;
            }

            match c {
                SINGLE_QUOTES if !in_double_quotes => Self::toggle_bool(&mut in_single_quotes),
                DOUBLE_QUOTES if !in_single_quotes => Self::toggle_bool(&mut in_double_quotes),
                BACKSLASH if !in_single_quotes => Self::toggle_bool(&mut escape_next),
                BACKSLASH => curr_arg.push(c),
                // When encountering a space and not inside quotes then we parsed a
                // complete argument, so push it to the vector and clear the string and
                // proceed with next argument
                SPACE if !in_single_quotes && !in_double_quotes => {
                    Self::save_arg(&mut curr_arg, &mut args)
                }
                _ => curr_arg.push(c),
            }
        }
        // Push last argument to the vector of arguments
        Self::save_arg(&mut curr_arg, &mut args);

        if args.is_empty() {
            None
        } else {
            Some(args)
        }
    }

    fn save_arg(arg: &mut String, args: &mut Vec<String>) {
        // Using `mem::take()` here avoids allocating `String`s on the heap
        if !arg.is_empty() {
            args.push(mem::take(arg));
        }
    }

    fn toggle_bool(b: &mut bool) {
        *b = !*b;
    }

    pub(super) fn handle_redirect(&self) -> io::Result<(Vec<String>, Option<File>, Option<File>)> {
        // Arguments up to redirection symbols (`>`, `1>`, `1>>`, `2>`, `2>>`)
        let mut cmd_args = Vec::new();

        let mut stdout_file = None; // File for stdout
        let mut stderr_file = None; // File for stderr

        let args = match self.args {
            Some(ref args) => args,
            None => {
                return Ok((cmd_args, stdout_file, stderr_file));
            }
        };

        // Iterator over arguments (String)
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                // Create file of path from next argument after redirection symbol for stdout
                ">" | "1>" | ">>" | "1>>" => stdout_file = Self::create_output_file(iter.next())?,
                // Create file of path from next argument after redirection symbol for stderr
                "2>" | "2>>" => stderr_file = Self::create_output_file(iter.next())?,
                // Any other argument we pass to `args`
                _ => cmd_args.push(arg.to_owned()),
            }
        }

        Ok((cmd_args, stdout_file, stderr_file))
    }

    fn create_output_file(arg: Option<&String>) -> io::Result<Option<File>> {
        // Create file which if doesn't exists will be created and can be appended to
        arg.map(|path| OpenOptions::new().append(true).create(true).open(path))
            .transpose() // Option<Result<T,E> -> Result<Option<T>, E>
    }

    pub(super) fn find_exe_in_path(name: &str) -> Option<PathBuf> {
        // Get `$PATH` and split on `:` to get all environment paths, then check if command is in
        // one of these paths
        env::var_os("PATH").and_then(|paths| {
            env::split_paths(&paths).find_map(|path| {
                let full_path = path.join(name);
                full_path.exists().then_some(full_path)
            })
        })
    }

    pub(super) fn print_prompt(&mut self) -> io::Result<()> {
        // Print prompt `$ ` and then flush to force direct output
        write!(self.stdout, "$ ")?;
        self.flush()?;
        Ok(())
    }

    pub(super) fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()?;
        self.stderr.flush()?;
        Ok(())
    }
}
