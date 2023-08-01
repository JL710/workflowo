use super::{Task, TaskError};
use std::{
    fmt,
    fmt::Display,
    io::{Read, Write},
    path::{Path, PathBuf},
};

fn connect_ssh(addr: &str, username: &str, password: &str) -> Result<ssh2::Session, TaskError> {
    // create connection with handshake etc.
    let tcp = match std::net::TcpStream::connect(addr.to_string() + ":22") {
        Ok(tcp) => tcp,
        Err(error) => {
            return Err(TaskError::from_error(
                "Connecting failed".to_string(),
                Box::new(error),
            ))
        }
    };
    let mut session = ssh2::Session::new().unwrap();
    session.set_tcp_stream(tcp);
    session.handshake().unwrap();

    // authenticate
    if let Err(error) = session.userauth_password(username, password) {
        return Err(TaskError::from_error(
            "Authentication failed".to_string(),
            Box::new(error),
        ));
    }
    Ok(session)
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
    fn execute(&self) -> Result<(), TaskError> {
        let sess = connect_ssh(&self.address.to_string(), &self.user, &self.password)?;

        // execute command
        for command in &self.commands {
            let (_stdout, exit_code) = execute_on_session(&sess, command);
            if exit_code != 0 {
                return Err(TaskError::from_message(format!(
                    "Something went wrong while executing an command (`{}`). Exit code {}.",
                    command, exit_code
                )));
            }
        }
        Ok(())
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
    fn execute(&self) -> Result<(), TaskError> {
        let session = connect_ssh(&self.address.to_string(), &self.user, &self.password)?;

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
        Ok(())
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
    fn execute(&self) -> Result<(), TaskError> {
        let session = connect_ssh(&self.address.to_string(), &self.user, &self.password)?;

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
        Ok(())
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
    fn execute(&self) -> Result<(), TaskError> {
        let session = connect_ssh(&self.address.to_string(), &self.user, &self.password)?;

        let sftp = session.sftp().unwrap();

        let stat = match sftp.stat(&self.remote_path) {
            Ok(stat) => stat,
            Err(error) => {
                return Err(TaskError::from_error(
                    format!(
                        "Error while getting stats of remote_path({})",
                        &self.remote_path.to_str().unwrap()
                    ),
                    Box::new(error),
                ))
            }
        };

        if stat.is_file() {
            if self.local_path.is_file() {
                return Err(TaskError::from_message(format!(
                    "File {} already exists",
                    &self.local_path.to_str().unwrap()
                )));
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
            // check if directory exists
            if self.local_path.is_dir() {
                return Err(TaskError::from_message(
                    "Directory already exists".to_string(),
                ));
            }
            // check if parent directory exists
            if !self.local_path.parent().unwrap().is_dir() {
                return Err(TaskError::from_message(format!(
                    "Path {} does not exist",
                    self.local_path.parent().unwrap().to_str().unwrap()
                )));
            }
            std::fs::create_dir(&self.local_path).unwrap();
            download_sftp_dir(&sftp, &self.local_path, &self.remote_path);
        } else {
            return Err(TaskError::from_message(format!(
                "Remote path {} does not exist",
                self.remote_path.to_str().unwrap()
            )));
        }
        Ok(())
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

// TODO: return result
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
// TODO: return result
fn download_sftp_file(sftp: &ssh2::Sftp, local_path: &Path, remote_path: &Path) {
    let mut remote_file = sftp.open(remote_path).unwrap();

    let mut contents = Vec::new();
    remote_file.read_to_end(&mut contents).unwrap();

    let mut local_file = std::fs::File::create(local_path).unwrap();
    local_file.write_all(&contents).unwrap();
}

#[derive(Debug)]
pub struct SftpUpload {
    address: std::net::Ipv4Addr,
    user: String,
    password: String,
    remote_path: PathBuf,
    local_path: PathBuf,
}

impl RemoteTransfer for SftpUpload {
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

impl Display for SftpUpload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            format!("{:?}", self)
                .replace(&self.password, "***Not displayed for security reasons***")
        )
    }
}

impl Task for SftpUpload {
    fn execute(&self) -> Result<(), TaskError> {
        // check if local stuff is valid
        if !self.local_path.is_dir() && !self.local_path.is_file() {
            panic!("Local {} does not exists", {
                self.local_path.to_str().unwrap()
            });
        }

        let session = connect_ssh(&self.address.to_string(), &self.user, &self.password)?;

        let sftp = session.sftp().unwrap();

        if self.local_path.is_file() {
            upload_sftp_file(&sftp, &self.local_path, &self.remote_path);
        } else {
            if sftp.stat(&self.remote_path).is_ok() {
                return Err(TaskError::from_message(format!(
                    "Remote path {} already exists",
                    &self.remote_path.to_str().unwrap()
                )));
            }
            sftp.mkdir(&self.remote_path, 0o774)
                .expect("Could not create dir");
            upload_sftp_directory(&sftp, &self.local_path, &self.remote_path);
        }
        Ok(())
    }
}

// TODO: return result
fn upload_sftp_directory(sftp: &ssh2::Sftp, local_path: &Path, remote_path: &Path) {
    for dir_entry in std::fs::read_dir(local_path).unwrap() {
        let dir_entry = dir_entry.unwrap();
        if dir_entry.file_type().unwrap().is_file() {
            upload_sftp_file(
                sftp,
                &dir_entry.path(),
                &remote_path.join(dir_entry.path().file_name().unwrap()),
            );
        } else {
            sftp.mkdir(
                &remote_path.join(dir_entry.path().file_name().unwrap()),
                0o774,
            )
            .expect("Error while creating directory");
            upload_sftp_directory(
                sftp,
                &dir_entry.path(),
                &remote_path.join(dir_entry.path().file_name().unwrap()),
            );
        }
    }
}

// uploads a file via the sftp connection -> asserts the paths are valid
// TODO: return result
fn upload_sftp_file(sftp: &ssh2::Sftp, local_path: &Path, remote_path: &Path) {
    // read local file
    let mut local_file = std::fs::File::open(local_path).unwrap();
    let mut content = Vec::new();
    local_file.read_to_end(&mut content).unwrap();

    // write to remote file
    let mut remote_file = sftp.create(remote_path).unwrap();
    remote_file.write_all(&content).unwrap();
}
