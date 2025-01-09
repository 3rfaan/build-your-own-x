use std::io::{self, BufWriter, Stdout, Write};
use std::path::PathBuf;
use std::process::Command;
use std::{env, process};

const BUILTINS: [&str; 3] = ["exit", "echo", "type"];

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
        let cmd = parts.next().unwrap_or(&input).to_string();
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
        if let Some(_) = self.find_exe(&self.cmd) {
            Command::new(&self.cmd).args(&self.args).status()?;
        } else {
            writeln!(self.writer, "{}: command not found", self.cmd)?;
        }
        self.flush()?;
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
            _ => shell.execute()?,
        }
        input.clear();
    }
}
