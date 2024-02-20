# workflowo

Workflowo is a tool to create simple local pipelines.

The configuration of pipelines is in a yaml file. There are Jobs located that can be executed from the cli.

Be careful when executing workflowo jobs from others. They may contain malicious code!

## Todo
- [ ] resolve relative/`~` paths
- [ ] default for inputs
- [X] improve error display

## Installation
```
cargo install --git https://github.com/JL710/workflowo
```
> Cargo/Rust needs to be installed => https://www.rust-lang.org/tools/install

## Jobs
Jobs are the largest abstraction layer. 
A Job has children. 
These children are Tasks. 
When a Job gets executed all of its children get executed in order.
```yaml
example_job:
    - child1
    - child2
```
> note that here a few placeholder values that would not be valid in a real context were used


## Tasks
A Task is something that can be executed. This can be a Job or other more specific tasks. For example a `bash` or `cmd` command.
```yaml
job1:
    - task1
    - task2

job2:
    - task3
    - job1
    - task4
```
> note that here a few placeholder values that would not be valid in a real context were used

### bash / cmd
`bash` and `cmd` are to similar tasks. The only difference is that one will execute the command with bash and the other with cmd.

Short use:
```yaml
example_job:
    - cmd: 'echo "Hello World"'
```

Long use:
```yaml
example_job:
    - bash:
        command: 'mkdir test'
        work_dir: "/home/someUser"
```

### OS Dependent Task
Sometimes it is needed to execute only if you are on a specific os.
Therefore there is a solution.
```yaml
example_job:
    - on-linux:
        - bash: 'echo "Hello World!"'  # will only be executed if you are on Linux
    - on-windows:
        - cmd: 'echo "Hello World!"'  # will only be executed if you are on Windows
```

### SSH
```yaml
example_job:
  - ssh:
      address: 192.128.114.12
      username: "some_user"
      password: "some_good_password"
      commands:
        - "mkdir newly_created_directory"
        - "rmdir newly_created_directory"
```
If you want to specify the allowed exit codes of a command you can use this representation (the default allowed exit code is 0):
```yaml
commands:
  - "ls 1"
  - command:
      command: "ls 2"
      exit_codes: [1, 2, 3]
``` 

### SCP Download
With the `scp-download` task you can download files from a remote computer via ssh onto your local.
```yaml
example_job:
  - scp-download:
      address: 192.128.114.12
      username: "some_user"
      password: "some_good_password"
      remote_path: "/home/some_user/remote_file.txt"
      local_path: "some_local_file"
```

### SCP Upload
With the `scp-upload` task you can upload files from a remote computer via ssh onto your local.
```yaml
example_job:
  - scp-upload:
      address: 192.128.114.12
      username: "some_user"
      password: "some_good_password"
      remote_path: "/home/some_user/remote_file.txt"
      local_path: "some_local_file.txt"
```

### Print
Prints a value into the prompt.
```yaml
example_job:
  - print: "Hello World!"
```

### SFTP Download
With `sftp-download` you can download files or directories from a remote computer via sftp onto your local.
```yaml
example_job:
  - sftp-upload:
      address: 192.128.114.12
      username: "some_user"
      password: "some_good_password"
      remote_path: "/home/some_user/remote_file.txt"
      local_path: "some_local_file.txt"
```

### SFTP Upload
With `sftp-upload` you can upload files or directories to a remote computer via sftp from your local.
```yaml
example_job:
  - sftp-upload:
      address: 192.128.114.12
      username: "some_user"
      password: "some_good_password"
      remote_path: "/home/some_user/remote_file.txt"
      local_path: "some_local_file.txt"
```


## IGNORE
Sometimes you want to create content that should be ignored in the file. Therefore is the `IGNORE` section. 

Could be useful to define secrets(passwords etc.) once and use it anywhere.
```yaml
IGNORE:
    whatever_you_do_here_will_be_ignored
```


## Custom Types
### !StrF
```yaml
!StrF ["Hello ", "World!"]
```
Formats to:
```yaml
"Hello World!"
```

### !Input
```yaml
!Input "Enter Something: "
```
> use `!HiddenInput` to hide input

Asks via command line for an input. The input will be used as the value.

### Id
Is able to be used on `!Input` Tags (on everything else as well). When using the yaml default, you have to input at every use. With `!Input` once.
```yaml
IGNORE:
  - taken_input: &taken_input !Id ["taken_input", !Input "Input: "]

example_job:
  - print: *taken_input
```
or
```yaml
IGNORE:
  - taken_input: &taken_input !Id { "id": "taken_input", "value": !Input "Input: " }

example_job:
  - print: *taken_input
```
