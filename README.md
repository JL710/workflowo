# workflowo

Workflowo is a tool to create simple local pipelines.

The configuration of pipelines is in a yaml file. There are Jobs located that can be executed from the cli.

Be careful when executing workflowo jobs from others. They may contain malicious code!


## Jobs
Jobs are the largest abstraction layer. 
A Job has children. 
These children are Tasks. 
When a Job gets executed all of its children get executed in order.
```yaml
somejob:
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
someJob:
    - cmd: 'echo "Hello World"'
```

Long use:
```yaml
someJob:
    - bash:
        command: 'mkdir test'
        work_dir: "/home/someUser"
```

### OS Dependent Task
Sometimes it is needed to execute only if you are on a specific os.
Therefore there is a solution.
```yaml
somejob:
    - on-linux:
        - bash: 'echo "Hello World!"'  # will only be executed if you are on Linux
    - on-windows:
        - cmd: 'echo "Hello World!"'  # will only be executed if you are on Windows
```