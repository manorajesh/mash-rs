use std::{path::PathBuf, ffi::CString};

use nix::{unistd::{ForkResult, fork, execvp, chdir}, sys::wait::wait};
use rustyline::{DefaultEditor, KeyEvent, Cmd};

struct Shell {
    prompt: String,
    path: PathBuf,
    current_command: Option<Command>,
    home: PathBuf,
}

impl Default for Shell {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or(String::from("/"));
        Self {
            prompt: format!("{} % ", home),
            path: PathBuf::from(&home),
            current_command: None,
            home: PathBuf::from(home),
        }
    }
}

impl Shell {
    fn execute(&mut self) -> Result<(), String> {
        if let Some(command) = &self.current_command {
            match command.name.as_str() {
                "cd" => {
                    if let Some(path) = command.args.get(0) {
                        let path = PathBuf::from(path);
                        if path.is_relative() {
                            self.path.push(path);
                        } else {
                            self.path = path;
                        }
                        self.prompt = format!("{} % ", self.path.to_str().ok_or("Unable to convert path to str")?);
                    } else {
                        self.path = self.home.clone();
                    }
                    self.prompt = format!("{} % ", self.path.canonicalize().map_err(|e| e.to_string())?.display());
                    chdir(self.path.as_os_str()).map_err(|e| e.to_string())?;
                },
                _ => {
                    command.execute_external(&self.path)?;
                }
            }
        }
        Ok(())
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

    fn execute_external(&self, workdir: &PathBuf) -> Result<(), String> {
        match unsafe { fork() } {
            Ok(ForkResult::Parent { .. }) => {
                // parent process
                // wait for child process to finish
                wait().map_err(|e| e.to_string())?;
            }

            Ok(ForkResult::Child) => {
                chdir(workdir.as_os_str()).map_err(|e| e.to_string())?;
                let cmd = CString::new(self.name.clone()).map_err(|e| e.to_string())?;
                let mut args = self.args.iter().map(|arg| CString::new(arg.clone()).log_expect("Failed to create CString for args")).collect::<Vec<_>>();
                args.insert(0, cmd.clone());
                execvp(&cmd, &args).map_err(|e| e.to_string())?;
            }

            Err(_) => {
                return Err(String::from("Failed to fork process"));
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
    if rl.load_history(".mash_history").is_err() {
        std::fs::File::create(".mash_history").log_expect("Failed to create history file");
    }
    let mut shell = Shell::default();
    rl.bind_sequence(KeyEvent::ctrl('r'), Cmd::HistorySearchBackward);
    // tab completion
    rl.bind_sequence(KeyEvent::ctrl('i'), Cmd::Complete);

    loop {
        let readline = rl.readline(shell.prompt.as_str());
        match readline {
            Ok(line) => {
                if line.is_empty() {
                    continue;
                }

                if line.trim() == "exit" {
                    break;
                }

                shell.current_command = Some(Command::parse(&line));

                if let Err(e) = shell.execute() {
                    log::error!("{}", e);
                } else {
                    rl.add_history_entry(line.as_str()).log_expect("Failed to add history entry");
                    rl.save_history(".mash_history").log_expect("Failed to save history file");
                }
            },
            Err(e) => {
                log::error!("{}", e);
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