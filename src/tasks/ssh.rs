use super::Task;
use std::{
    fmt,
    fmt::Display,
    io::{Read, Write},
    path::{Path, PathBuf},
};

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

pub trait RemoteTransfer {
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

impl RemoteTransfer for ScpFileDownload {
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

impl RemoteTransfer for ScpFileUpload {
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
pub struct SftpDownload {
    address: std::net::Ipv4Addr,
    user: String,
    password: String,
    remote_path: PathBuf,
    local_path: PathBuf,
}

impl RemoteTransfer for SftpDownload {
    fn new(
        address: std::net::Ipv4Addr,
        user: String,
        password: String,
        remote_path: PathBuf,
        local_path: PathBuf,
    ) -> Self {
        Self {
            address,
            user,
            password,
            remote_path,
            local_path,
        }
    }
}

impl Task for SftpDownload {
    fn execute(&self) {
        // create connection
        let tcp = std::net::TcpStream::connect(self.address.to_string() + ":22").unwrap();
        let mut session = ssh2::Session::new().unwrap();
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();
        session
            .userauth_password(&self.user, &self.password)
            .unwrap();

        let sftp = session.sftp().unwrap();

        let stat = match sftp.stat(&self.remote_path) {
            Ok(stat) => stat,
            Err(error) => panic!(
                "Error while getting stats of remote_path({}): {}",
                &self.remote_path.to_str().unwrap(),
                error
            ),
        };

        if stat.is_file() {
            if self.local_path.is_file() {
                panic!("File {} already exists", &self.local_path.to_str().unwrap())
            } else if self.local_path.is_dir() {
                // use file name on remote as local file
                download_sftp_file(
                    &sftp,
                    &self.local_path.join(self.remote_path.file_name().unwrap()),
                    &self.remote_path,
                );
            } else {
                download_sftp_file(&sftp, &self.local_path, &self.remote_path);
            }
        } else if stat.is_dir() {
            if self
                .local_path
                .join(self.remote_path.file_name().unwrap())
                .is_dir()
            {
                panic!("Directory already exists");
            }
            if !self.local_path.is_dir() {
                panic!("Path {} does not exist", {self.local_path.to_str().unwrap()});
            }
            std::fs::create_dir(self.local_path.join(self.remote_path.file_name().unwrap()))
                .unwrap();
            download_sftp_dir(
                &sftp,
                &self.local_path.join(self.remote_path.file_name().unwrap()),
                &self.remote_path,
            );
        } else {
            panic!(
                "Remote path {} does not exist",
                self.remote_path.to_str().unwrap()
            );
        }
    }
}

impl Display for SftpDownload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            format!("{:?}", self)
                .replace(&self.password, "***Not displayed for security reasons***")
        )
    }
}

fn download_sftp_dir(sftp: &ssh2::Sftp, local_path: &Path, remote_path: &Path) {
    for (path, file_stat) in sftp.readdir(remote_path).unwrap() {
        if file_stat.is_file() {
            download_sftp_file(sftp, &local_path.join(path.file_name().unwrap()), &path);
        } else {
            std::fs::create_dir(local_path.join(path.file_name().unwrap())).unwrap();
            download_sftp_dir(sftp, &local_path.join(path.file_name().unwrap()), &path);
        }
    }
}

// will download a file via sftp -> assumes that the paths are valid
fn download_sftp_file(sftp: &ssh2::Sftp, local_path: &Path, remote_path: &Path) {
    let mut remote_file = sftp.open(remote_path).unwrap();

    let mut contents = Vec::new();
    remote_file.read_to_end(&mut contents).unwrap();

    let mut local_file = std::fs::File::create(local_path).unwrap();
    local_file.write_all(&contents).unwrap();
}
