use std::{
    env, fmt,
    fmt::Display,
    io::{Read, Write},
    path::PathBuf,
    process::{self, Command},
};

pub trait Task: Display {
    /// Will be called when the task should be executed.
    fn execute(&self);
}

pub struct Job {
    pub name: String,
    children: Vec<Box<dyn Task>>,
}

impl Job {
    pub fn new(name: String) -> Self {
        Self {
            name,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: Box<dyn Task>) {
        self.children.push(child);
    }
}

impl Task for Job {
    fn execute(&self) {
        for child in self.children.iter() {
            child.execute();
        }
    }
}

impl Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = format!("Job: {{ name \"{}\" children {{ ", &self.name);
        for child in &self.children {
            text += &format!("{} ", child);
        }
        text += "} }";

        write!(f, "{}", text)
    }
}

pub trait ShellCommand {
    fn new(args: Vec<String>, work_dir: Option<String>) -> Self;
}

#[derive(Debug)]
pub struct Bash {
    args: Vec<String>,
    work_dir: Option<String>,
}

impl ShellCommand for Bash {
    fn new(args: Vec<String>, work_dir: Option<String>) -> Self {
        Bash { args, work_dir }
    }
}

impl Task for Bash {
    fn execute(&self) {
        let mut command = Command::new("bash");

        if let Some(work_dir) = &self.work_dir {
            command.current_dir(work_dir);
        }

        let output = command
            .arg("-c")
            .arg(&self.args.join(" "))
            .output()
            .unwrap();
        if !output.status.success() {
            println!(
                "Error: {:?} did not success and raised an error!\n{}",
                &self.args,
                String::from_utf8_lossy(&output.stderr)
            );
            process::exit(-1)
        }
    }
}

impl Display for Bash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}

#[derive(Debug)]
pub struct Cmd {
    args: Vec<String>,
    work_dir: Option<String>,
}

impl ShellCommand for Cmd {
    fn new(args: Vec<String>, work_dir: Option<String>) -> Self {
        Cmd { args, work_dir }
    }
}

impl Task for Cmd {
    fn execute(&self) {
        let mut command = Command::new("cmd");

        if let Some(work_dir) = &self.work_dir {
            command.current_dir(work_dir);
        }

        let output = command.arg("/c").args(&self.args).output().unwrap();
        if !output.status.success() {
            println!(
                "Error: {:?} did not success and raised an error!\n{}",
                &self.args,
                String::from_utf8_lossy(&output.stderr)
            );
            process::exit(-1)
        }
    }
}

impl Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}

#[derive(Debug)]
pub enum OS {
    Windows,
    Linux,
}

pub struct OSDependent {
    os: OS,
    children: Vec<Box<dyn Task>>,
}

impl OSDependent {
    pub fn new(os: OS) -> Self {
        Self {
            os,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: Box<dyn Task>) {
        self.children.push(child)
    }
}

impl Task for OSDependent {
    fn execute(&self) {
        match self.os {
            OS::Windows => {
                if env::consts::OS != "windows" {
                    return;
                }
            }
            OS::Linux => {
                if env::consts::OS != "linux" {
                    return;
                }
            }
        }

        for child in &self.children {
            child.execute();
        }
    }
}

impl Display for OSDependent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut text = format!("OSDependent: {{ os \"{:?}\" children {{ ", &self.os);
        for child in &self.children {
            text += &format!("{} ", child);
        }
        text += "} }";

        write!(f, "{}", text)
    }
}

#[derive(Debug)]
pub struct SshCommand {
    address: std::net::Ipv4Addr,
    user: String,
    password: String,
    commands: Vec<String>,
}

impl SshCommand {
    pub fn new(
        address: std::net::Ipv4Addr,
        user: String,
        password: String,
        commands: Vec<String>,
    ) -> Self {
        Self {
            address,
            user,
            password,
            commands,
        }
    }
}

/// Executes a command on the `Session`. Returns a Tuple with the Prompt and exit code.
fn execute_on_session(session: &ssh2::Session, command: &str) -> (String, i32) {
    let mut channel = session.channel_session().unwrap();

    channel.exec(command).unwrap();

    let mut stdout = String::new();
    channel.read_to_string(&mut stdout).unwrap();

    channel.wait_close().unwrap();

    (stdout, channel.exit_status().unwrap())
}

impl Task for SshCommand {
    fn execute(&self) {
        // create connection with handshake etc.
        let tcp = std::net::TcpStream::connect(self.address.to_string() + ":22").unwrap();
        let mut sess = ssh2::Session::new().unwrap();
        sess.set_tcp_stream(tcp);
        sess.handshake().unwrap();

        // authenticate
        sess.userauth_password(&self.user, &self.password).unwrap();

        // execute command
        for command in &self.commands {
            let (_stdout, exit_code) = execute_on_session(&sess, command);
            if exit_code != 0 {
                panic!(
                    "Something went wrong while executing an command (`{}`). Exit code {}.",
                    command, exit_code
                )
            }
        }
    }
}

impl Display for SshCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            format!("{:?}", self)
                .replace(&self.password, "***Not displayed for security reasons***")
        )
    }
}

pub trait Scp {
    fn new(
        address: std::net::Ipv4Addr,
        user: String,
        password: String,
        remote_path: PathBuf,
        local_path: PathBuf,
    ) -> Self;
}

#[derive(Debug)]
pub struct ScpFileDownload {
    address: std::net::Ipv4Addr,
    user: String,
    password: String,
    remote_path: PathBuf,
    local_path: PathBuf,
}

impl Scp for ScpFileDownload {
    fn new(
        address: std::net::Ipv4Addr,
        user: String,
        password: String,
        remote_path: PathBuf,
        local_path: PathBuf,
    ) -> Self {
        ScpFileDownload {
            address,
            user,
            password,
            remote_path,
            local_path,
        }
    }
}

impl Task for ScpFileDownload {
    fn execute(&self) {
        // create connection
        let tcp = std::net::TcpStream::connect(self.address.to_string() + ":22").unwrap();
        let mut session = ssh2::Session::new().unwrap();
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();
        session
            .userauth_password(&self.user, &self.password)
            .unwrap();

        // receive file
        let (mut remote_file, _stat) = session.scp_recv(&self.remote_path).unwrap();
        let mut contents = Vec::new();
        remote_file.read_to_end(&mut contents).unwrap();

        // close channel and wait for the content to be transferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();

        // write content to local file
        let mut file = std::fs::File::create(&self.local_path).unwrap();
        file.write_all(&contents).unwrap();
    }
}

impl Display for ScpFileDownload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            format!("{:?}", self)
                .replace(&self.password, "***Not displayed for security reasons***")
        )
    }
}

#[derive(Debug)]
pub struct ScpFileUpload {
    address: std::net::Ipv4Addr,
    user: String,
    password: String,
    remote_path: PathBuf,
    local_path: PathBuf,
}

impl Scp for ScpFileUpload {
    fn new(
        address: std::net::Ipv4Addr,
        user: String,
        password: String,
        remote_path: PathBuf,
        local_path: PathBuf,
    ) -> Self {
        ScpFileUpload {
            address,
            user,
            password,
            remote_path,
            local_path,
        }
    }
}

impl Task for ScpFileUpload {
    fn execute(&self) {
        // create connection
        let tcp = std::net::TcpStream::connect(self.address.to_string() + ":22").unwrap();
        let mut session = ssh2::Session::new().unwrap();
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();
        session
            .userauth_password(&self.user, &self.password)
            .unwrap();

        // read file
        let mut file = std::fs::File::open(&self.local_path).unwrap();
        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();

        // upload file
        let mut remote_file = session
            .scp_send(&self.remote_path, 0o644, content.len() as u64, None)
            .unwrap();
        remote_file.write_all(&content).unwrap();

        // close channel and wait for the content to be transferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();
    }
}

impl Display for ScpFileUpload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            format!("{:?}", self)
                .replace(&self.password, "***Not displayed for security reasons***")
        )
    }
}

#[derive(Debug)]
pub struct PrintTask {
    prompt: String,
}

impl PrintTask {
    pub fn new(prompt: String) -> Self {
        Self { prompt }
    }
}

impl Task for PrintTask {
    fn execute(&self) {
        println!("{}", self.prompt);
    }
}

impl Display for PrintTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
