mod error;
mod utils;

use std::io::{self, BufWriter, Stderr, Stdout, Write};
use std::path::PathBuf;
use std::process::Command;
use std::{env, process};

use self::error::ShellError;
use self::utils::BUILTINS;

pub type Result<T> = std::result::Result<T, ShellError>;

pub struct Shell {
    cmd: String,
    args: Option<Vec<String>>,
    stdout: BufWriter<Stdout>,
    stderr: BufWriter<Stderr>,
}

impl Shell {
    pub fn new(stdout: Stdout, stderr: Stderr) -> Self {
        Self {
            cmd: String::new(),
            args: None,
            stdout: BufWriter::new(stdout),
            stderr: BufWriter::new(stderr),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut input = String::new();

        loop {
            self.print_prompt()?;
            stdin.read_line(&mut input)?;

            self.parse_input(&input);

            if let Err(error) = self.handle_cmd() {
                writeln!(self.stderr, "{}", error)?;
            }

            self.flush()?;
            input.clear();
        }
    }

    fn handle_cmd(&mut self) -> Result<()> {
        if self.cmd.is_empty() {
            return Ok(());
        }

        match self.cmd.as_str() {
            "exit" => self.exit(),
            "echo" => self.echo(),
            "type" => self.type_(),
            "pwd" => self.pwd(),
            "cd" => self.cd(),
            _ => self.execute(),
        }
    }
}

impl Shell {
    fn exit(&mut self) -> Result<()> {
        let code = self.args.as_ref().and_then(|args| args.first());

        match code {
            Some(code) => code.parse::<i32>().map_or_else(
                |error| Err(ShellError::ExitCodeParseError(error)),
                |code| {
                    process::exit(code);
                },
            ),
            None => process::exit(0),
        }
    }

    fn echo(&mut self) -> Result<()> {
        let (cmd_args, stdout_file, _) = self.handle_redirect()?;
        let output = cmd_args.join(" ");

        if let Some(mut file) = stdout_file {
            writeln!(file, "{}", output)?;
        } else {
            writeln!(self.stdout, "{}", output)?;
        }

        Ok(())
    }

    fn type_(&mut self) -> Result<()> {
        let args = self.args.as_ref().ok_or(ShellError::NoArguments)?;
        let arg = args.first().ok_or(ShellError::NoArguments)?;

        if BUILTINS.contains(&arg.as_str()) {
            // Check if command is shell builtin
            writeln!(self.stdout, "{} is a shell builtin", arg)?;
        } else if let Some(path) = Self::find_exe_in_path(arg) {
            // Check if command is in `$PATH`
            writeln!(self.stdout, "{} is {}", arg, path.display())?;
        } else {
            return Err(ShellError::CommandNotFound(arg.to_owned()));
        }

        Ok(())
    }

    fn pwd(&mut self) -> Result<()> {
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
            let path = path
                .strip_prefix("~")
                .map_err(ShellError::HomeDirPathError)?;

            PathBuf::from(home).join(path)
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
        env::set_current_dir(&path).map_err(|_| ShellError::FileOrDirNotFound(path))?;

        Ok(())
    }
    fn execute(&mut self) -> Result<()> {
        // If redirect with either `>`, `1>` or `2>` then get arguments until symbol,
        // handle to file of either stdout or stderr
        let (cmd_args, stdout_file, stderr_file) = self.handle_redirect()?;
        let mut cmd = Command::new(&self.cmd);

        cmd.args(cmd_args);

        if let Some(file) = stdout_file {
            cmd.stdout(file);
        }

        if let Some(file) = stderr_file {
            cmd.stderr(file);
        }

        cmd.status()
            .map_err(|_| ShellError::CommandNotFound(self.cmd.clone()))?;

        Ok(())
    }
}

fn main() -> Result<()> {
    let stdout = io::stdout();
    let stderr = io::stderr();

    let mut shell = Shell::new(stdout, stderr);

    shell.run()
}
