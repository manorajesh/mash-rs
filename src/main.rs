use std::{path::PathBuf, ffi::CString};

use nix::{unistd::{ForkResult, fork, execvp}, sys::wait::wait};
use rustyline::DefaultEditor;

struct Shell {
    prompt: String,
    path: PathBuf,
    current_command: Option<Command>,
}

impl Default for Shell {
    fn default() -> Self {
        let home = std::env::var("HOME").log_expect("Failed to get $HOME");
        Self {
            prompt: format!("{} % ", home),
            path: PathBuf::from(home),
            current_command: None,
        }
    }
}

struct Command {
    name: String,
    args: Vec<String>,
}

impl Command {
    fn new(name: String, args: Vec<String>) -> Self {
        Self {
            name,
            args,
        }
    }

    fn parse(line: &str) -> Self {
        let mut parts = line.split_whitespace();
        let name = parts.next().unwrap_or("").to_string();
        let args = parts.map(|s| s.to_string()).collect();
        Self::new(name, args)
    }

    fn execute(&self) -> Result<(), &str> {
        match unsafe { fork() } {
            Ok(ForkResult::Parent { .. }) => {
                // parent process
                // wait for child process to finish
                wait().log_expect("Failed to wait for child process");
            }

            Ok(ForkResult::Child) => {
                let cmd = CString::new(self.name.clone()).log_expect("Failed to create CString");
                let mut args = self.args.iter().map(|arg| CString::new(arg.clone()).log_expect("Failed to create CString for args")).collect::<Vec<_>>();
                args.insert(0, cmd.clone());
                execvp(&cmd, &args).log_expect("Failed to execute command");
            }

            Err(_) => {
                return Err("Failed to fork process")
            }
        }
        Ok(())
    }
}

fn main() -> Result<(), ()> {
    // start main loop
    // print prompt and read input
    // call fork and exec system calls
    // wait for child process to finish
    // repeat

    env_logger::init();

    let mut rl = DefaultEditor::new().log_expect("Failed to create editor");
    let mut shell = Shell::default();

    loop {
        let readline = rl.readline(shell.prompt.as_str());
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                shell.current_command = Some(Command::parse(&line));

                shell.current_command
                    .log_expect("")
                    .execute()
                    .log_expect("");
            },
            Err(_) => {
                println!("EOF");
                break;
            }
        }
    }

    Ok(())
}

pub trait LogExpect<T> {
    fn log_expect(self, msg: &str) -> T;
}

impl<T> LogExpect<T> for Option<T> {
    fn log_expect(self, msg: &str) -> T {
        match self {
            Some(val) => val,
            None => {
                log::error!("{}", msg);
                std::process::exit(1);
            }
        }
    }
}

impl<T, E> LogExpect<T> for Result<T, E> 
where E: std::fmt::Display
{
    fn log_expect(self, msg: &str) -> T {
        match self {
            Ok(val) => val,
            Err(e) => {
                if msg.is_empty() {
                    log::error!("{}", e);
                } else {
                    log::error!("{}", msg);
                    log::error!("{}", e);
                }
                
                std::process::exit(1);
            }
        }
    }
}