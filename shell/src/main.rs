mod utils;

use std::io::{self, BufWriter, Stdout, Write};
use std::path::PathBuf;
use std::process::Command;
use std::{env, process};

use utils::{sh_write, sh_writeln};

const BUILTINS: [&str; 5] = ["cd", "echo", "exit", "pwd", "type"];

pub struct Shell {
    cmd: String,
    args: Vec<String>,
    writer: BufWriter<Stdout>,
}

impl Shell {
    fn exit(&mut self) -> io::Result<()> {
        if let Some(code) = self.args.first() {
            if let Ok(code) = code.parse::<i32>() {
                process::exit(code)
            } else {
                sh_writeln!(self, "Invalid exit code: {}", code)?;
            }
        } else {
            process::exit(0);
        }
        Ok(())
    }

    fn echo(&mut self) -> io::Result<()> {
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                sh_write!(self, " ")?;
            }
            sh_write!(self, "{}", arg)?;
        }
        sh_write!(self, "\n")?;
        Ok(())
    }

    fn cmd_type(&mut self) -> io::Result<()> {
        if let Some(arg) = self.args.first() {
            let path = Self::find_exe_in_path(arg);

            if BUILTINS.contains(&arg.as_str()) {
                sh_writeln!(self, "{} is a shell builtin", arg)?;
            } else if let Some(path) = path {
                sh_writeln!(self, "{} is {}", arg, path.display())?;
            } else {
                sh_writeln!(self, "{}: not found", arg)?;
            }
        }
        Ok(())
    }

    fn execute(&mut self) -> io::Result<()> {
        if Command::new(&self.cmd).args(&self.args).status().is_err() {
            sh_writeln!(self, "{}: command not found", self.cmd)?
        }
        Ok(())
    }

    fn pwd(&mut self) -> io::Result<()> {
        sh_writeln!(self, "{}", env::current_dir()?.display())?;
        Ok(())
    }

    fn cd(&mut self) -> io::Result<()> {
        let home = env::var("HOME").expect("No $HOME variable set");
        let path = self
            .args
            .first()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&home));

        let path = if path.starts_with("~") {
            let home = PathBuf::from(home);
            home.join(path.strip_prefix("~").unwrap_or(&path))
        } else if path.is_absolute() {
            path
        } else {
            env::current_dir()?.join(&path)
        };

        if env::set_current_dir(&path).is_err() {
            sh_writeln!(self, "cd: {}: No such file or directory", path.display())?;
        }
        Ok(())
    }
}

fn main() -> Result<(), io::Error> {
    let stdin = io::stdin();

    let mut input = String::new();
    let mut shell = Shell::default();

    loop {
        shell.print_prompt()?;

        stdin.read_line(&mut input)?;
        shell.parse_input(&input);

        match shell.cmd.trim() {
            "exit" => shell.exit()?,
            "echo" => shell.echo()?,
            "type" => shell.cmd_type()?,
            "pwd" => shell.pwd()?,
            "cd" => shell.cd()?,
            _ => shell.execute()?,
        }
        shell.flush()?;
        input.clear();
    }
}
