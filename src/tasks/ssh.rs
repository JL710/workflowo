use super::Task;
use anyhow::{bail, Context, Result};
use std::{
    fmt,
    fmt::Display,
    io::{Read, Write},
    path::{Path, PathBuf},
};

fn connect_ssh(addr: &str, username: &str, password: &str) -> Result<ssh2::Session> {
    // create connection with handshake etc.
    let tcp =
        std::net::TcpStream::connect(addr.to_string() + ":22").context("Connecting failed")?;
    let mut session = ssh2::Session::new().context("Failed to create ssh Session")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("ssh handshake failed")?;

    // authenticate
    session
        .userauth_password(username, password)
        .context("Authentication failed")?;
    Ok(session)
}

/// Holds one command with the allowed access codes for that specific command.
#[derive(Debug, PartialEq)]
pub struct SshCommand {
    command: String,
    allowed_exit_codes: Vec<i32>,
}

impl SshCommand {
    pub fn new(command: String, allowed_exit_codes: Vec<i32>) -> Self {
        Self {
            command,
            allowed_exit_codes,
        }
    }

    fn execute(&self, session: &ssh2::Session) -> Result<()> {
        let (_stdout, exit_code) = execute_on_session(session, &self.command)?;
        if !self.allowed_exit_codes.contains(&exit_code) {
            bail!(format!(
                "Something went wrong while executing an command (`{}`). Exit code {}.",
                self.command, exit_code
            ));
        }
        Ok(())
    }
}

/// A task that holds [`SshCommand`]'s with the remote information and can execute them in order.
#[derive(Debug)]
pub struct SshTask {
    address: std::net::Ipv4Addr,
    user: String,
    password: String,
    commands: Vec<SshCommand>,
}

impl SshTask {
    pub fn new(
        address: std::net::Ipv4Addr,
        user: String,
        password: String,
        commands: Vec<SshCommand>,
    ) -> Self {
        Self {
            address,
            user,
            password,
            commands,
        }
    }
}

/// Executes a command on the [`ssh2::Session`]. Returns a Tuple with the Prompt and exit code.
fn execute_on_session(session: &ssh2::Session, command: &str) -> Result<(String, i32)> {
    let mut channel = session
        .channel_session()
        .context("Failed to establish a channel session")?;

    channel
        .exec(command)
        .context("Error while executing command via ssh")?;

    let mut stdout = String::new();
    channel
        .read_to_string(&mut stdout)
        .context("Failed to read output of ssh channel")?;

    channel
        .wait_close()
        .context("Error while closing the channel session")?;

    Ok((
        stdout,
        channel
            .exit_status()
            .context("Failed to read exit status")?,
    ))
}

impl Task for SshTask {
    fn execute(&self) -> Result<()> {
        let sess = connect_ssh(&self.address.to_string(), &self.user, &self.password)
            .context("failed to connect via ssh")?;

        // execute commands
        for command in &self.commands {
            command
                .execute(&sess)
                .context(format!("failed to execute command via ssh: {:?}", &command))?;
        }
        Ok(())
    }
}

impl Display for SshTask {
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
    fn execute(&self) -> Result<()> {
        let session = connect_ssh(&self.address.to_string(), &self.user, &self.password)
            .context("Failed to connect via ssh")?;

        // receive file
        let (mut remote_file, _stat) = session
            .scp_recv(&self.remote_path)
            .context("Error opening file")?;
        let mut contents = Vec::new();

        remote_file
            .read_to_end(&mut contents)
            .context("Error while reading file")?;

        // close channel and wait for the content to be transferred
        remote_file.send_eof().context("Error while sending EOF")?;
        remote_file
            .wait_eof()
            .context("Error while waiting for EOF")?;
        remote_file.close().context("Error while closing file")?;
        remote_file
            .wait_close()
            .context("Error while waiting for close file")?;

        // write content to local file
        let mut file = std::fs::File::create(&self.local_path)
            .context(format!("Error while creating file {:?}", self.local_path))?;
        file.write_all(&contents)
            .context(format!("Error while reading file {:?}", self.local_path))?;
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
    fn execute(&self) -> Result<()> {
        let session = connect_ssh(&self.address.to_string(), &self.user, &self.password)
            .context("Failed to connect via ssh")?;

        // read file
        let mut file = std::fs::File::open(&self.local_path)
            .context(format!("Error while opening file {:?}", self.local_path))?;
        let mut content = Vec::new();

        file.read_to_end(&mut content)
            .context(format!("Error while reading file {:?}", self.local_path))?;

        // upload file
        let mut remote_file = session
            .scp_send(&self.remote_path, 0o644, content.len() as u64, None)
            .context(format!(
                "Error while creating file {:?} on remote machine",
                self.remote_path
            ))?;
        remote_file.write_all(&content).context(format!(
            "Error while writing to file {:?}",
            self.remote_path
        ))?;

        // close channel and wait for the content to be transferred
        remote_file.send_eof().context("Error while sending EOF")?;
        remote_file
            .wait_eof()
            .context("Error while waiting for EOF")?;
        remote_file.close().context("Error while closing file")?;
        remote_file
            .wait_close()
            .context("Error while waiting for close file")?;
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
    fn execute(&self) -> Result<()> {
        let session = connect_ssh(&self.address.to_string(), &self.user, &self.password)
            .context("Failed to connect via ssh")?;

        let sftp = session.sftp().context("Could not create sftp subsystem")?;

        let stat = sftp.stat(&self.remote_path).context(format!(
            "Error while getting stats of remote_path({})",
            &self.remote_path.to_str().unwrap()
        ))?;

        if stat.is_file() {
            if self.local_path.is_file() {
                bail!(format!(
                    "File {} already exists",
                    &self.local_path.to_str().unwrap()
                ));
            } else if self.local_path.is_dir() {
                // use file name on remote as local file
                download_sftp_file(
                    &sftp,
                    &self.local_path.join(self.remote_path.file_name().unwrap()),
                    &self.remote_path,
                )
                .context("Error while downloading file via sftp")?;
            } else {
                download_sftp_file(&sftp, &self.local_path, &self.remote_path)
                    .context("Error while downloading file via sftp")?;
            }
        } else if stat.is_dir() {
            // check if directory exists
            if self.local_path.is_dir() {
                bail!("Directory already exists");
            }
            // check if parent directory exists
            if !self.local_path.parent().unwrap().is_dir() {
                bail!(format!(
                    "Path {} does not exist",
                    self.local_path.parent().unwrap().to_str().unwrap()
                ));
            }

            std::fs::create_dir(&self.local_path).context(format!(
                "Error while creating directory {:?}",
                self.local_path
            ))?;
            download_sftp_dir(&sftp, &self.local_path, &self.remote_path)?;
        } else {
            bail!(format!(
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

fn download_sftp_dir(sftp: &ssh2::Sftp, local_path: &Path, remote_path: &Path) -> Result<()> {
    for (path, file_stat) in sftp
        .readdir(remote_path)
        .context("Erro while reading directory via sftp")?
    {
        if file_stat.is_file() {
            download_sftp_file(sftp, &local_path.join(path.file_name().unwrap()), &path)
                .context("Error while downloading file via sftp")?;
        } else {
            std::fs::create_dir(local_path.join(path.file_name().unwrap())).context(format!(
                "Error while creating directory {:?}",
                local_path.join(path.file_name().unwrap())
            ))?;
            download_sftp_dir(sftp, &local_path.join(path.file_name().unwrap()), &path)
                .context("Error while downloading file via sftp")?;
        }
    }
    Ok(())
}

// will download a file via sftp -> assumes that the paths are valid
fn download_sftp_file(sftp: &ssh2::Sftp, local_path: &Path, remote_path: &Path) -> Result<()> {
    let mut remote_file = sftp
        .open(remote_path)
        .context(format!("Could not open file {:?}", remote_path))?;

    let mut contents = Vec::new();

    remote_file
        .read_to_end(&mut contents)
        .context(format!("Error while reading file {:?}", remote_path))?;

    let mut local_file = std::fs::File::create(local_path)
        .context(format!("Could not create local file {:?}", local_path))?;

    local_file
        .write_all(&contents)
        .context(format!("Error while writing to file {:?}", local_path))?;
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
    fn execute(&self) -> Result<()> {
        // check if local stuff is valid
        if !self.local_path.is_dir() && !self.local_path.is_file() {
            bail!(format!(
                "Local {} does not exists",
                self.local_path.to_str().unwrap()
            ));
        }

        let session = connect_ssh(&self.address.to_string(), &self.user, &self.password)
            .context("Error while connect via ssh")?;

        let sftp = session.sftp().context("Could not create sftp subsystem")?;

        if self.local_path.is_file() {
            upload_sftp_file(&sftp, &self.local_path, &self.remote_path)
                .context("Error while uploading file via sftp")?;
        } else {
            if sftp.stat(&self.remote_path).is_ok() {
                bail!(format!(
                    "Remote path {} already exists",
                    &self.remote_path.to_str().unwrap()
                ));
            }
            sftp.mkdir(&self.remote_path, 0o774)
                .context(format!("Could not create dir {:?}", self.remote_path))?;
            upload_sftp_directory(&sftp, &self.local_path, &self.remote_path)
                .context("Error while uploading file via sftp")?;
        }
        Ok(())
    }
}

fn upload_sftp_directory(sftp: &ssh2::Sftp, local_path: &Path, remote_path: &Path) -> Result<()> {
    for dir_entry in std::fs::read_dir(local_path).context(format!(
        "Error while reading directory {}",
        local_path.to_str().unwrap()
    ))? {
        let dir_entry = dir_entry?;
        if dir_entry.file_type().unwrap().is_file() {
            upload_sftp_file(
                sftp,
                &dir_entry.path(),
                &remote_path.join(dir_entry.path().file_name().unwrap()),
            )
            .context("Error while uploading file via sftp")?;
        } else {
            sftp.mkdir(
                &remote_path.join(dir_entry.path().file_name().unwrap()),
                0o774,
            )
            .context(format!(
                "Error while creating directory {:?}",
                &remote_path.join(dir_entry.path().file_name().unwrap())
            ))?;
            upload_sftp_directory(
                sftp,
                &dir_entry.path(),
                &remote_path.join(dir_entry.path().file_name().unwrap()),
            )
            .context("Error while uploading file via sftp")?;
        }
    }
    Ok(())
}

/// uploads a file via the sftp connection -> asserts the paths are valid
fn upload_sftp_file(sftp: &ssh2::Sftp, local_path: &Path, remote_path: &Path) -> Result<()> {
    // read local file
    let mut local_file = std::fs::File::open(local_path)
        .context(format!("open local file failed {:?}", local_path))?;
    let mut content = Vec::new();

    local_file
        .read_to_end(&mut content)
        .context(format!("error wile reading file {:?}", local_path))?;

    // write to remote file
    let mut remote_file = sftp
        .create(remote_path)
        .context(format!("Could not open remote file {:?}", remote_path))?;

    remote_file
        .write_all(&content)
        .context(format!("Error while writing to file {:?}", remote_path))?;
    Ok(())
}
