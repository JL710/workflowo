use super::{task_error_panic, task_might_panic, task_panic, Task, TaskError};
use std::{
    fmt,
    fmt::Display,
    io::{Read, Write},
    path::{Path, PathBuf},
};

fn connect_ssh(addr: &str, username: &str, password: &str) -> Result<ssh2::Session, TaskError> {
    // create connection with handshake etc.
    let tcp = task_might_panic!(
        std::net::TcpStream::connect(addr.to_string() + ":22"),
        "Connecting failed"
    );
    let mut session = task_might_panic!(ssh2::Session::new(), "Failed to create ssh Session");
    session.set_tcp_stream(tcp);
    task_might_panic!(session.handshake(), "ssh handshake failed");

    // authenticate
    if let Err(error) = session.userauth_password(username, password) {
        task_error_panic!("Authentication failed", error);
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
fn execute_on_session(session: &ssh2::Session, command: &str) -> Result<(String, i32), TaskError> {
    let mut channel = session.channel_session().unwrap();

    task_might_panic!(
        channel.exec(command),
        "Error while executing command via ssh"
    );

    let mut stdout = String::new();
    task_might_panic!(
        channel.read_to_string(&mut stdout),
        "Failed to read output of ssh channel"
    );

    channel.wait_close().unwrap();

    Ok((stdout, channel.exit_status().unwrap()))
}

impl Task for SshCommand {
    fn execute(&self) -> Result<(), TaskError> {
        let sess = connect_ssh(&self.address.to_string(), &self.user, &self.password)?;

        // execute command
        for command in &self.commands {
            let (_stdout, exit_code) = execute_on_session(&sess, command)?;
            if exit_code != 0 {
                task_panic!(format!(
                    "Something went wrong while executing an command (`{}`). Exit code {}.",
                    command, exit_code
                ));
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
        let (mut remote_file, _stat) =
            task_might_panic!(session.scp_recv(&self.remote_path), "Error opening file");
        let mut contents = Vec::new();
        task_might_panic!(
            remote_file.read_to_end(&mut contents),
            "Error while reading file"
        );

        // close channel and wait for the content to be transferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();

        // write content to local file
        let mut file = task_might_panic!(
            std::fs::File::create(&self.local_path),
            format!("Error while creating file {:?}", self.local_path)
        );
        task_might_panic!(
            file.write_all(&contents),
            format!("Error while reading file {:?}", self.local_path)
        );
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
        let mut file = task_might_panic!(
            std::fs::File::open(&self.local_path),
            format!("Error while opening file {:?}", self.local_path)
        );
        let mut content = Vec::new();
        task_might_panic!(
            file.read_to_end(&mut content),
            format!("Error while reading file {:?}", self.local_path)
        );

        // upload file
        let mut remote_file = task_might_panic!(
            session.scp_send(&self.remote_path, 0o644, content.len() as u64, None),
            format!(
                "Error while creating file {:?} on remote machine",
                self.remote_path
            )
        );
        task_might_panic!(
            remote_file.write_all(&content),
            format!("Error while writing to file {:?}", self.remote_path)
        );

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

        let sftp = task_might_panic!(session.sftp(), "Could not create sftp subsystem");

        let stat = task_might_panic!(
            sftp.stat(&self.remote_path),
            format!(
                "Error while getting stats of remote_path({})",
                &self.remote_path.to_str().unwrap()
            )
        );

        if stat.is_file() {
            if self.local_path.is_file() {
                task_panic!(format!(
                    "File {} already exists",
                    &self.local_path.to_str().unwrap()
                ));
            } else if self.local_path.is_dir() {
                // use file name on remote as local file
                download_sftp_file(
                    &sftp,
                    &self.local_path.join(self.remote_path.file_name().unwrap()),
                    &self.remote_path,
                )?;
            } else {
                download_sftp_file(&sftp, &self.local_path, &self.remote_path)?;
            }
        } else if stat.is_dir() {
            // check if directory exists
            if self.local_path.is_dir() {
                task_panic!("Directory already exists");
            }
            // check if parent directory exists
            if !self.local_path.parent().unwrap().is_dir() {
                task_panic!(format!(
                    "Path {} does not exist",
                    self.local_path.parent().unwrap().to_str().unwrap()
                ));
            }
            task_might_panic!(
                std::fs::create_dir(&self.local_path),
                format!("Error while creating directory {:?}", self.local_path)
            );
            download_sftp_dir(&sftp, &self.local_path, &self.remote_path)?;
        } else {
            task_panic!(format!(
                "Remote path {} does not exist",
                self.remote_path.to_str().unwrap()
            ));
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

fn download_sftp_dir(
    sftp: &ssh2::Sftp,
    local_path: &Path,
    remote_path: &Path,
) -> Result<(), TaskError> {
    for (path, file_stat) in sftp.readdir(remote_path).unwrap() {
        if file_stat.is_file() {
            download_sftp_file(sftp, &local_path.join(path.file_name().unwrap()), &path)?;
        } else {
            task_might_panic!(
                std::fs::create_dir(local_path.join(path.file_name().unwrap())),
                format!(
                    "Error while creating directory {:?}",
                    local_path.join(path.file_name().unwrap())
                )
            );
            download_sftp_dir(sftp, &local_path.join(path.file_name().unwrap()), &path)?;
        }
    }
    Ok(())
}

// will download a file via sftp -> assumes that the paths are valid
fn download_sftp_file(
    sftp: &ssh2::Sftp,
    local_path: &Path,
    remote_path: &Path,
) -> Result<(), TaskError> {
    let mut remote_file = task_might_panic!(
        sftp.open(remote_path),
        format!("Could not open file {:?}", remote_path)
    );

    let mut contents = Vec::new();
    task_might_panic!(
        remote_file.read_to_end(&mut contents),
        format!("Error while reading file {:?}", remote_path)
    );

    let mut local_file = task_might_panic!(
        std::fs::File::create(local_path),
        format!("Could not create local file {:?}", local_path)
    );
    task_might_panic!(
        local_file.write_all(&contents),
        format!("Error while writing to file {:?}", local_path)
    );
    Ok(())
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

        let sftp = task_might_panic!(session.sftp(), "Could not create sftp subsystem");

        if self.local_path.is_file() {
            upload_sftp_file(&sftp, &self.local_path, &self.remote_path)?;
        } else {
            if sftp.stat(&self.remote_path).is_ok() {
                task_panic!(format!(
                    "Remote path {} already exists",
                    &self.remote_path.to_str().unwrap()
                ));
            }
            task_might_panic!(
                sftp.mkdir(&self.remote_path, 0o774),
                format!("Could not create dir {:?}", self.remote_path)
            );
            upload_sftp_directory(&sftp, &self.local_path, &self.remote_path)?;
        }
        Ok(())
    }
}

fn upload_sftp_directory(
    sftp: &ssh2::Sftp,
    local_path: &Path,
    remote_path: &Path,
) -> Result<(), TaskError> {
    for dir_entry in std::fs::read_dir(local_path).unwrap() {
        let dir_entry = dir_entry.unwrap();
        if dir_entry.file_type().unwrap().is_file() {
            upload_sftp_file(
                sftp,
                &dir_entry.path(),
                &remote_path.join(dir_entry.path().file_name().unwrap()),
            )?;
        } else {
            task_might_panic!(
                sftp.mkdir(
                    &remote_path.join(dir_entry.path().file_name().unwrap()),
                    0o774,
                ),
                format!(
                    "Error while creating directory {:?}",
                    &remote_path.join(dir_entry.path().file_name().unwrap())
                )
            );
            upload_sftp_directory(
                sftp,
                &dir_entry.path(),
                &remote_path.join(dir_entry.path().file_name().unwrap()),
            )?;
        }
    }
    Ok(())
}

/// uploads a file via the sftp connection -> asserts the paths are valid
fn upload_sftp_file(
    sftp: &ssh2::Sftp,
    local_path: &Path,
    remote_path: &Path,
) -> Result<(), TaskError> {
    // read local file
    let mut local_file = task_might_panic!(
        std::fs::File::open(local_path),
        format!("open local file failed {:?}", local_path)
    );
    let mut content = Vec::new();
    task_might_panic!(
        local_file.read_to_end(&mut content),
        format!("error wile reading file {:?}", local_path)
    );

    // write to remote file
    let mut remote_file = task_might_panic!(
        sftp.create(remote_path),
        format!("Could not open remote file {:?}", remote_path)
    );
    task_might_panic!(
        remote_file.write_all(&content),
        format!("Error while writing to file {:?}", remote_path)
    );
    Ok(())
}
