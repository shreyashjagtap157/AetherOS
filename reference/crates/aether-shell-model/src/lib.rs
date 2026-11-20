//! Shell model (host reference): minimal command-line shell.
//!
//! This is a bounded executable host model of the AetherOS shell.
//! It parses and executes a small set of built-in commands against
//! a simple virtual file system. It is deliberately NOT a production
//! shell (no pipes, redirection, scripting, or job control).
//!
//! Scope:
//! - Line-based input parsing (space-separated tokens).
//! - Built-in commands: help, echo, ls, cat, mkdir, rm, pwd, cd, exit.
//! - Virtual working directory.
//! - Command history (fixed size).
//!
//! Out of scope:
//! - Pipes, redirection, background jobs.
//! - Scripting, variables, conditionals.
//! - Tab completion, line editing.
//! - Environment variables.

#![allow(missing_docs)]

use std::collections::VecDeque;

/// Maximum command history size.
const MAX_HISTORY: usize = 16;

/// Maximum number of arguments per command.
const MAX_ARGS: usize = 8;

/// Shell error types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellError {
    /// Unknown command.
    UnknownCommand,
    /// Missing argument.
    MissingArg,
    /// Too many arguments.
    TooManyArgs,
    /// File not found.
    FileNotFound,
    /// Command failed.
    Failed,
    /// Exit requested.
    Exit,
}

/// A parsed command.
#[derive(Clone, Debug)]
pub struct Command {
    pub name: String,
    pub args: Vec<String>,
}

/// Virtual file system entry.
#[derive(Clone, Debug)]
pub enum FsEntry {
    File { content: String },
    Dir { children: Vec<String> },
}

/// Minimal virtual file system.
#[derive(Debug)]
pub struct VirtualFs {
    entries: std::collections::BTreeMap<String, FsEntry>,
    cwd: String,
}

impl VirtualFs {
    pub fn new() -> Self {
        let mut entries = std::collections::BTreeMap::new();
        entries.insert("/".to_string(), FsEntry::Dir { children: vec![] });
        Self {
            entries,
            cwd: "/".to_string(),
        }
    }

    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    pub fn ls(&self, path: Option<&str>) -> Result<Vec<String>, ShellError> {
        let dir = path.unwrap_or(&self.cwd);
        let dir = if dir == "." { self.cwd.as_str() } else { dir };
        match self.entries.get(dir) {
            Some(FsEntry::Dir { children }) => Ok(children.clone()),
            _ => Err(ShellError::FileNotFound),
        }
    }

    pub fn mkdir(&mut self, name: &str) -> Result<(), ShellError> {
        let path = if self.cwd == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", self.cwd, name)
        };
        if self.entries.contains_key(&path) {
            return Err(ShellError::Failed);
        }
        self.entries.insert(path.clone(), FsEntry::Dir { children: vec![] });
        // Add to parent's children.
        if let Some(FsEntry::Dir { children }) = self.entries.get_mut(&self.cwd) {
            children.push(name.to_string());
        }
        Ok(())
    }

    pub fn touch(&mut self, name: &str) -> Result<(), ShellError> {
        let path = if self.cwd == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", self.cwd, name)
        };
        if self.entries.contains_key(&path) {
            return Err(ShellError::Failed);
        }
        self.entries.insert(path.clone(), FsEntry::File { content: String::new() });
        if let Some(FsEntry::Dir { children }) = self.entries.get_mut(&self.cwd) {
            children.push(name.to_string());
        }
        Ok(())
    }

    pub fn cat(&self, name: &str) -> Result<String, ShellError> {
        let path = if self.cwd == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", self.cwd, name)
        };
        match self.entries.get(&path) {
            Some(FsEntry::File { content }) => Ok(content.clone()),
            _ => Err(ShellError::FileNotFound),
        }
    }

    pub fn write_file(&mut self, name: &str, content: &str) -> Result<(), ShellError> {
        let path = if self.cwd == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", self.cwd, name)
        };
        match self.entries.get_mut(&path) {
            Some(FsEntry::File { content: c }) => {
                *c = content.to_string();
                Ok(())
            }
            _ => Err(ShellError::FileNotFound),
        }
    }

    pub fn rm(&mut self, name: &str) -> Result<(), ShellError> {
        let path = if self.cwd == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", self.cwd, name)
        };
        if self.entries.remove(&path).is_none() {
            return Err(ShellError::FileNotFound);
        }
        if let Some(FsEntry::Dir { children }) = self.entries.get_mut(&self.cwd) {
            children.retain(|c| c != name);
        }
        Ok(())
    }

    pub fn cd(&mut self, path: &str) -> Result<(), ShellError> {
        if path == "/" {
            self.cwd = "/".to_string();
            return Ok(());
        }
        if path == ".." {
            if self.cwd != "/" {
                if let Some(pos) = self.cwd.rfind('/') {
                    if pos == 0 {
                        self.cwd = "/".to_string();
                    } else {
                        self.cwd = self.cwd[..pos].to_string();
                    }
                }
            }
            return Ok(());
        }
        let new_path = if self.cwd == "/" {
            format!("/{}", path)
        } else {
            format!("{}/{}", self.cwd, path)
        };
        if self.entries.contains_key(&new_path) {
            self.cwd = new_path;
            Ok(())
        } else {
            Err(ShellError::FileNotFound)
        }
    }
}

/// Shell state.
#[derive(Debug)]
pub struct Shell {
    pub fs: VirtualFs,
    pub history: VecDeque<String>,
    pub running: bool,
    pub output: VecDeque<String>,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            fs: VirtualFs::new(),
            history: VecDeque::new(),
            running: true,
            output: VecDeque::new(),
        }
    }

    /// Parses a command line into a Command.
    pub fn parse(line: &str) -> Result<Command, ShellError> {
        let line = line.trim();
        if line.is_empty() {
            return Err(ShellError::MissingArg);
        }
        let mut parts = line.split_whitespace();
        let name = parts.next().unwrap().to_string();
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();
        if args.len() > MAX_ARGS {
            return Err(ShellError::TooManyArgs);
        }
        Ok(Command { name, args })
    }

    /// Executes a command, returning output lines.
    pub fn execute(&mut self, cmd: &Command) -> Result<Vec<String>, ShellError> {
        let mut out = Vec::new();
        match cmd.name.as_str() {
            "help" => {
                out.push("Commands: help, echo, ls, cat, mkdir, touch, rm, pwd, cd, exit".to_string());
            }
            "echo" => {
                out.push(cmd.args.join(" "));
            }
            "ls" => {
                let path = cmd.args.first().map(|s| s.as_str());
                let entries = self.fs.ls(path)?;
                out.push(entries.join("  "));
            }
            "cat" => {
                let name = cmd.args.first().ok_or(ShellError::MissingArg)?;
                let content = self.fs.cat(name)?;
                out.push(content);
            }
            "mkdir" => {
                let name = cmd.args.first().ok_or(ShellError::MissingArg)?;
                self.fs.mkdir(name)?;
                out.push(format!("created dir '{}'", name));
            }
            "touch" => {
                let name = cmd.args.first().ok_or(ShellError::MissingArg)?;
                self.fs.touch(name)?;
                out.push(format!("created file '{}'", name));
            }
            "rm" => {
                let name = cmd.args.first().ok_or(ShellError::MissingArg)?;
                self.fs.rm(name)?;
                out.push(format!("removed '{}'", name));
            }
            "pwd" => {
                out.push(self.fs.cwd().to_string());
            }
            "cd" => {
                let path = cmd.args.first().ok_or(ShellError::MissingArg)?;
                self.fs.cd(path)?;
            }
            "exit" => {
                self.running = false;
                return Err(ShellError::Exit);
            }
            _ => return Err(ShellError::UnknownCommand),
        }
        Ok(out)
    }

    /// Runs a single input line: parse + execute + record history.
    pub fn run_line(&mut self, line: &str) -> Result<Vec<String>, ShellError> {
        if line.trim().is_empty() {
            return Ok(vec![]);
        }
        if self.history.len() >= MAX_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(line.to_string());
        let cmd = Self::parse(line)?;
        self.execute(&cmd)
    }

    /// Returns whether the shell is still running.
    pub fn is_running(&self) -> bool {
        self.running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let cmd = Shell::parse("echo hello world").unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["hello", "world"]);
    }

    #[test]
    fn parse_empty_fails() {
        assert!(Shell::parse("").is_err());
    }

    #[test]
    fn echo_command() {
        let mut shell = Shell::new();
        let out = shell.run_line("echo hello").unwrap();
        assert_eq!(out, vec!["hello"]);
    }

    #[test]
    fn pwd_default() {
        let mut shell = Shell::new();
        let out = shell.run_line("pwd").unwrap();
        assert_eq!(out, vec!["/"]);
    }

    #[test]
    fn mkdir_and_ls() {
        let mut shell = Shell::new();
        shell.run_line("mkdir foo").unwrap();
        let out = shell.run_line("ls").unwrap();
        assert!(out[0].contains("foo"));
    }

    #[test]
    fn touch_cat_write() {
        let mut shell = Shell::new();
        shell.run_line("touch file.txt").unwrap();
        shell.fs.write_file("file.txt", "hello").unwrap();
        let out = shell.run_line("cat file.txt").unwrap();
        assert_eq!(out, vec!["hello"]);
    }

    #[test]
    fn rm_file() {
        let mut shell = Shell::new();
        shell.run_line("touch tmp.txt").unwrap();
        shell.run_line("rm tmp.txt").unwrap();
        assert!(shell.run_line("cat tmp.txt").is_err());
    }

    #[test]
    fn cd_and_pwd() {
        let mut shell = Shell::new();
        shell.run_line("mkdir dir").unwrap();
        shell.run_line("cd dir").unwrap();
        let out = shell.run_line("pwd").unwrap();
        assert_eq!(out, vec!["/dir"]);
        shell.run_line("cd ..").unwrap();
        let out = shell.run_line("pwd").unwrap();
        assert_eq!(out, vec!["/"]);
    }

    #[test]
    fn unknown_command() {
        let mut shell = Shell::new();
        assert_eq!(shell.run_line("foobar"), Err(ShellError::UnknownCommand));
    }

    #[test]
    fn exit_sets_running_false() {
        let mut shell = Shell::new();
        assert!(shell.is_running());
        let _ = shell.run_line("exit");
        assert!(!shell.is_running());
    }

    #[test]
    fn history_tracking() {
        let mut shell = Shell::new();
        shell.run_line("echo a").unwrap();
        shell.run_line("echo b").unwrap();
        assert_eq!(shell.history.len(), 2);
    }
}