use std::process::{Command, Stdio, Child, ChildStdin};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{self, Write, Read};

pub struct Subcommand {
    child: Arc<Mutex<Child>>,
    child_stdin: Arc<Mutex<ChildStdin>>,
    child_pid: u32,
}

impl Subcommand {
    pub fn spawn(program: &str, args: &[String]) -> Result<Self, String> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to start {}: {}", program, e))?;

        let child_pid = child.id();
        let child_stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let mut child_stdout = child.stdout.take().ok_or("Failed to open stdout")?;

        let child_shared = Arc::new(Mutex::new(child));

        let prompt = Arc::new(Mutex::new(String::new()));
        let prompt_clone = Arc::clone(&prompt);

        // Thread to read from child's stdout
        thread::spawn(move || {
            let mut buffer = [0u8; 1];
            let mut current_line = String::new();

            loop {
                match child_stdout.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let ch = buffer[0] as char;
                        print!("{}", ch);
                        io::stdout().flush().unwrap();

                        if ch == '\n' {
                            current_line.clear();
                        } else {
                            current_line.push(ch);
                            // Check if this looks like a prompt (ends with >, :, or $)
                            if (ch == '>' || ch == ':' || ch == '$') && !current_line.is_empty() {
                                let mut prompt = prompt_clone.lock().unwrap();
                                *prompt = current_line.clone();
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let child_stdin = Arc::new(Mutex::new(child_stdin));

        Ok(Subcommand {
            child: child_shared,
            child_stdin,
            child_pid,
        })
    }

    pub fn get_stdin(&self) -> Arc<Mutex<ChildStdin>> {
        Arc::clone(&self.child_stdin)
    }

    pub fn wait_for_exit(&self) -> i32 {
        let mut child = self.child.lock().unwrap();
        let status = child.wait().expect("Failed to wait on child");
        status.code().unwrap_or(1)
    }

    pub fn start_stdin_forwarder(&self) {
        let child_stdin = Arc::clone(&self.child_stdin);

        thread::spawn(move || {
            let stdin = io::stdin();
            let mut input = String::new();

            loop {
                input.clear();
                match stdin.read_line(&mut input) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let mut child_stdin_lock = child_stdin.lock().unwrap();
                        if writeln!(*child_stdin_lock, "{}", input.trim()).is_err() {
                            break;
                        }
                        if child_stdin_lock.flush().is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    pub fn terminate_gracefully(&self) {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        // Use the stored PID directly - no need to acquire the lock
        let pid_for_kill = self.child_pid as i32;

        let pid = Pid::from_raw(pid_for_kill);
        let _ = kill(pid, Signal::SIGTERM);

        // Give it a moment to terminate gracefully
        thread::sleep(std::time::Duration::from_millis(500));

        // Try to force kill if still running
        let pid = Pid::from_raw(pid_for_kill);
        let _ = kill(pid, Signal::SIGKILL);
    }
}
