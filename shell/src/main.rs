mod utils;

use std::io::{self, BufWriter, Stdout, Write};
use std::path::PathBuf;
use std::process::Command;
use std::{env, process};

const BUILTINS: [&str; 5] = ["cd", "echo", "exit", "pwd", "type"];

pub struct Shell {
    cmd: String,
    args: Vec<String>,
    writer: BufWriter<Stdout>,
}

impl Shell {
    fn new(stdout: Stdout) -> Self {
        Self {
            cmd: String::new(),
            args: Vec::new(),
            writer: BufWriter::new(stdout),
        }
    }

    fn handle_cmd(&mut self) -> io::Result<()> {
        match self.cmd.trim() {
            "exit" => self.exit()?,
            "echo" => self.echo()?,
            "type" => self.type_()?,
            "pwd" => self.pwd()?,
            "cd" => self.cd()?,
            _ => self.execute()?,
        }
        Ok(())
    }
}

impl Shell {
    fn exit(&mut self) -> io::Result<()> {
        // Get first argument
        if let Some(code) = self.args.first() {
            // Parse exit code to i32
            match code.parse::<i32>() {
                Ok(code) => process::exit(code),
                Err(_) => writeln!(self.writer, "Invalid exit code: {}", code)?,
            }
        } else {
            // Exit with exit code 0 when user types `exit` in terminal without
            // argument
            process::exit(0);
        }
        Ok(())
    }

    fn echo(&mut self) -> io::Result<()> {
        let (cmd_args, stdout_file, _) = self.handle_redirect()?;
        let output = cmd_args.join(" ");

        if let Some(mut stdout) = stdout_file {
            writeln!(stdout, "{}", output)?;
        } else {
            writeln!(self.writer, "{}", output)?;
        }

        Ok(())
    }

    fn type_(&mut self) -> io::Result<()> {
        // Get first argument
        if let Some(arg) = self.args.first() {
            // Check if command is shell builtin
            if BUILTINS.contains(&arg.as_str()) {
                writeln!(self.writer, "{} is a shell builtin", arg)?;
            }
            // Check if command is in `$PATH`
            else if let Some(path) = Self::find_exe_in_path(arg) {
                writeln!(self.writer, "{} is {}", arg, path.display())?;
            } else {
                writeln!(self.writer, "{}: not found", arg)?;
            }
        }
        Ok(())
    }

    fn execute(&mut self) -> io::Result<()> {
        // If redirect with either `>`, `1>` or `2>` then get arguments until symbol,
        // handle to file of either stdout or stderr
        let (cmd_args, stdout_file, stderr_file) = self.handle_redirect()?;

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
            writeln!(self.writer, "{}: command not found", self.cmd)?;
        }

        Ok(())
    }

    fn pwd(&mut self) -> io::Result<()> {
        // Print working directory
        writeln!(self.writer, "{}", env::current_dir()?.display())?;
        Ok(())
    }

    fn cd(&mut self) -> io::Result<()> {
        // Get `$HOME` path
        let home = env::var("HOME").expect("No $HOME variable set");
        // Get first argument and try to create PathBuf from it, otherwise PathBuf from
        // home path
        let path = self
            .args
            .first()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&home));

        // If path starts with `~` then strip the symbol and replace it with `$HOME` path
        // (usually /home/{$USER})
        let path = if path.starts_with("~") {
            let home = PathBuf::from(home);
            home.join(path.strip_prefix("~").unwrap_or(&path))
        }
        // If absolute path return path
        else if path.is_absolute() {
            path
        }
        // Otherwise return relative path
        else {
            env::current_dir()?.join(&path)
        };

        // Set environment current working directory to `path`
        if env::set_current_dir(&path).is_err() {
            writeln!(
                self.writer,
                "cd: {}: No such file or directory",
                path.display()
            )?;
        }
        Ok(())
    }
}

fn main() -> Result<(), io::Error> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    let mut input = String::new();
    let mut shell = Shell::new(stdout);

    loop {
        shell.print_prompt()?;

        stdin.read_line(&mut input)?;

        shell.parse_input(&input);
        shell.handle_cmd()?;
        shell.flush()?;

        input.clear();
    }
}
