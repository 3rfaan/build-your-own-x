mod error;
mod utils;

use std::io::{self, BufWriter, Stderr, Stdout, Write};
use std::path::PathBuf;
use std::process::Command;
use std::{env, process};

use self::error::ShellError;

pub type Result<T> = std::result::Result<T, ShellError>;

const BUILTINS: [&str; 5] = ["cd", "echo", "exit", "pwd", "type"];

pub struct Shell {
    cmd: String,
    args: Option<Vec<String>>,
    stdout: BufWriter<Stdout>,
    stderr: BufWriter<Stderr>,
}

impl Shell {
    fn new(stdout: Stdout, stderr: Stderr) -> Self {
        Self {
            cmd: String::new(),
            args: None,
            stdout: BufWriter::new(stdout),
            stderr: BufWriter::new(stderr),
        }
    }

    fn handle_cmd(&mut self) -> Result<()> {
        match self.cmd.trim() {
            "exit" => {
                if let Err(error) = self.exit() {
                    writeln!(self.stderr, "{}", error)?;
                }
            }
            "echo" => {
                if let Err(error) = self.echo() {
                    writeln!(self.stderr, "{}", error)?;
                }
            }
            "type" => {
                if let Err(error) = self.type_() {
                    writeln!(self.stderr, "{}", error)?;
                }
            }
            "pwd" => self.pwd()?,
            "cd" => {
                if let Err(error) = self.cd() {
                    writeln!(self.stderr, "{}", error)?;
                }
            }
            _ => {
                if let Err(error) = self.execute() {
                    writeln!(self.stderr, "{}", error)?;
                }
            }
        }
        Ok(())
    }
}

impl Shell {
    fn exit(&mut self) -> Result<()> {
        let args = match self.args {
            Some(ref args) => args,
            None => {
                // Exit with exit code `0` as default when user doesn't provide argument
                process::exit(0);
            }
        };

        if let Some(code) = args.first() {
            match code.parse::<i32>() {
                Ok(code) => process::exit(code),
                Err(error) => return Err(ShellError::InvalidExitCode(error)),
            }
        }
        Ok(())
    }

    fn echo(&mut self) -> Result<()> {
        match self.handle_redirect() {
            Ok((cmd_args, stdout_file, _)) => {
                let output = cmd_args.join(" ");

                if let Some(mut stdout) = stdout_file {
                    writeln!(stdout, "{}", output)?;
                } else {
                    writeln!(self.stdout, "{}", output)?;
                }
            }
            Err(error) => return Err(ShellError::RedirectionError(error.to_string())),
        }
        Ok(())
    }

    fn type_(&mut self) -> Result<()> {
        let args = match self.args {
            Some(ref args) => args,
            None => return Ok(()),
        };

        // Get first argument
        match args.first() {
            Some(arg) => {
                // Check if command is shell builtin
                if BUILTINS.contains(&arg.as_str()) {
                    writeln!(self.stdout, "{} is a shell builtin", arg)?;
                }
                // Check if command is in `$PATH`
                else if let Some(path) = Self::find_exe_in_path(arg) {
                    writeln!(self.stdout, "{} is {}", arg, path.display())?;
                } else {
                    return Err(ShellError::CommandNotFound(arg.to_owned()));
                }
            }
            None => {
                return Err(ShellError::TooFewArguments {
                    required: 1,
                    received: args.len(),
                })
            }
        }
        Ok(())
    }

    fn execute(&mut self) -> Result<()> {
        // If redirect with either `>`, `1>` or `2>` then get arguments until symbol,
        // handle to file of either stdout or stderr

        match self.handle_redirect() {
            Ok((cmd_args, stdout_file, stderr_file)) => {
                // Pass command and arguments
                let mut cmd = Command::new(&self.cmd);
                cmd.args(cmd_args);

                // If `>` or `1>` or `1>>` we should get back file for stdout
                if let Some(stdout) = stdout_file {
                    cmd.stdout(stdout);
                }

                // If `2>` or `2>>` we should get back file for stderr
                if let Some(stderr) = stderr_file {
                    cmd.stderr(stderr);
                }

                // Execute command with provided arguments, if error then command is invalid
                if cmd.status().is_err() {
                    return Err(ShellError::CommandNotFound(self.cmd.clone()));
                }
            }
            Err(error) => return Err(ShellError::RedirectionError(error.to_string())),
        }

        Ok(())
    }

    fn pwd(&mut self) -> io::Result<()> {
        // Print working directory
        writeln!(self.stdout, "{}", env::current_dir()?.display())?;
        Ok(())
    }

    fn cd(&mut self) -> Result<()> {
        // Get `$HOME` path
        let home = env::var("HOME").map_err(|_| ShellError::EnvVarNotFound("HOME".to_owned()))?;

        // Get first argument and try to create PathBuf from it, otherwise PathBuf from
        // home path
        let path = self
            .args
            .as_ref()
            .and_then(|args| args.first().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from(&home));

        // Replace `~` with `$HOME`
        let path = if path.starts_with("~") {
            let home = PathBuf::from(home);
            home.join(path.strip_prefix("~").unwrap_or(&path))
        }
        // Use absolute path as-is
        else if path.is_absolute() {
            path
        }
        // Resolve relative paths against the current working directory
        else {
            env::current_dir()?.join(&path)
        };

        // Attempt to change the current working directory
        if env::set_current_dir(&path).is_err() {
            return Err(ShellError::FileOrDirNotFound(path));
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();

    let mut input = String::new();
    let mut shell = Shell::new(stdout, stderr);

    loop {
        shell.print_prompt()?;

        stdin.read_line(&mut input)?;

        shell.parse_input(&input);
        shell.handle_cmd()?;
        shell.flush()?;

        input.clear();
    }
}
