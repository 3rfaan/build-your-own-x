use std::io::{self, BufWriter, Stdout, Write};
use std::path::PathBuf;
use std::process::Command;
use std::{env, process};

const BUILTINS: [&str; 5] = ["cd", "echo", "exit", "pwd", "type"];

struct Shell {
    cmd: String,
    args: Vec<String>,
    writer: BufWriter<Stdout>,
}

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
    fn parse(&mut self, input: &str) {
        let mut parts = input.trim().splitn(2, ' ');
        let cmd = parts.next().unwrap_or(input).to_string();
        let args = parts
            .next()
            .unwrap_or("")
            .split_whitespace()
            .map(str::to_string)
            .collect();

        self.cmd = cmd;
        self.args = args;
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        Ok(())
    }

    fn print_prompt(&mut self) -> io::Result<()> {
        write!(self.writer, "$ ")?;
        self.flush()?;
        Ok(())
    }

    fn find_exe(&self, name: &str) -> Option<PathBuf> {
        env::var_os("PATH").map(|paths| {
            env::split_paths(&paths).find_map(|path| {
                let full_path = path.join(name);
                full_path.exists().then_some(full_path)
            })
        })?
    }

    fn exit(&mut self) -> io::Result<()> {
        if let Some(code) = self.args.first() {
            if let Ok(code) = code.parse::<i32>() {
                process::exit(code)
            } else {
                writeln!(self.writer, "Invalid exit code: {}", code)?;
            }
        } else {
            process::exit(0);
        }
        self.flush()?;
        Ok(())
    }

    fn echo(&mut self) -> io::Result<()> {
        writeln!(self.writer, "{}", self.args.join(" "))?;
        self.flush()?;
        Ok(())
    }

    fn cmd_type(&mut self) -> io::Result<()> {
        if let Some(arg) = self.args.first() {
            let path = self.find_exe(arg);

            if BUILTINS.contains(&arg.as_str()) {
                writeln!(self.writer, "{} is a shell builtin", arg)?;
            } else if let Some(path) = path {
                writeln!(self.writer, "{} is {}", arg, path.display())?;
            } else {
                writeln!(self.writer, "{}: not found", arg)?;
            }
        }
        self.flush()?;
        Ok(())
    }

    fn execute(&mut self) -> io::Result<()> {
        if self.find_exe(&self.cmd).is_some() {
            Command::new(&self.cmd).args(&self.args).status()?;
        } else {
            writeln!(self.writer, "{}: command not found", self.cmd)?;
        }
        self.flush()?;
        Ok(())
    }

    fn pwd(&mut self) -> io::Result<()> {
        writeln!(self.writer, "{}", env::current_dir()?.display())?;
        Ok(())
    }

    fn cd(&mut self) -> io::Result<()> {
        let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());
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

    let mut input = String::new();
    let mut shell = Shell::default();

    loop {
        shell.print_prompt()?;

        stdin.read_line(&mut input)?;
        shell.parse(&input);

        match shell.cmd.trim() {
            "exit" => shell.exit()?,
            "echo" => shell.echo()?,
            "type" => shell.cmd_type()?,
            "pwd" => shell.pwd()?,
            "cd" => shell.cd()?,
            _ => shell.execute()?,
        }
        input.clear();
    }
}
