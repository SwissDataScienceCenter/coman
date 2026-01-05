# coman

[![Lint and Test](https://github.com/SwissDataScienceCenter/coman/actions/workflows/test.yaml/badge.svg)](https://github.com/SwissDataScienceCenter/coman/actions/workflows/test.yaml)
[![Release](https://github.com/SwissDataScienceCenter/coman/actions/workflows/release.yaml/badge.svg)](https://github.com/SwissDataScienceCenter/coman/actions/workflows/release.yaml)

Compute Manager for managing HPC compute

Table of contents
=================

<!--ts-->
   * [Installation](#installation)
      * [Linux](#linux)
      * [Macos](#macos)
      * [Windows](#windows)
   * [Usage](#usage)
      * [Logging in](#logging-in)
      * [CLI](#cli)
      * [Terminal UI](#tui)
      * [SSH](#ssh)
      * [coman.toml config file](#comantoml-config-file)
        * [Editing the config](#editing-the-config)
   * [Development](#development)
     * [Prerequisites](#prerequisites)
     * [Install binaries](#install-binaries)
<!--te-->

## Installation

### Linux

```shell
curl -LO https://github.com/SwissDataScienceCenter/coman/releases/latest/download/coman-Linux-x86_64-musl.tar.gz
sudo tar -xzf coman-Linux-x86_64-musl.tar.gz -C /usr/local/bin/
sudo chmod +x /usr/local/bin/coman
```

### Macos

```shell
curl -LO https://github.com/SwissDataScienceCenter/coman/releases/latest/download/coman-Darwin-x86_64.tar.gz
sudo tar -xzf coman-Darwin-x86_64.tar.gz -C /usr/local/bin/
sudo chmod +x /usr/local/bin/coman
```

### Windows
Run as Admin:

```powershell
# Download the ZIP file
Invoke-WebRequest -Uri "https://github.com/SwissDataScienceCenter/coman/releases/latest/download/coman-Windows-aarch64.zip" -OutFile "coman-Windows-aarch64.zip"

# Extract the ZIP file
Expand-Archive -Path "coman-Windows-aarch64.zip" -DestinationPath ".\coman_temp" -Force

# Move the binary to C:\Program Files\coman
New-Item -ItemType Directory -Path "C:\Program Files\coman" -Force
Move-Item -Path ".\coman_temp\coman.exe" -Destination "C:\Program Files\coman\coman.exe" -Force

# Permanent PATH addition (requires Administrator privileges)
[Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";C:\Program Files\coman", "Machine")

# Clean up temporary files
Remove-Item -Path "coman-Windows-aarch64.zip" -Force
Remove-Item -Path ".\coman_temp" -Recurse -Force
```


## Usage

Coman can be used in two ways, one is as a normal CLI tool the other is as an interactive terminal UI (TUI).

### Logging in
To use Coman, you need to log in to CSCS.

For this you need go to the [CSCS Developer portal](https://developer.svc.cscs.ch) and go to `Applications`
![CSCS Dev Portal](/assets/cscs_app_1.jpg)

After logging in with your CSCS credentials, pick the `DefaultApplication`
![CSCS Dev Portal](/assets/cscs_app_2.jpg)

Click on `Production Keys`
![CSCS Dev Portal](/assets/cscs_app_3.jpg)

Then click on `Generate Keys` at the bottom, without changing any settings.
![CSCS Dev Portal](/assets/cscs_app_4.jpg)

You should now have a Consumer Key (`Client id` in Coman) and a Consumer Secret (`Client Secret` in Coman),
keep note of this for later
![CSCS Dev Portal](/assets/cscs_app_5.jpg)

We also need to enable Firecrest subscriptions for the CSCS API, go to `Subscriptions` and click `Subscribe APIS`
![CSCS Dev Portal](/assets/cscs_app_6.jpg)

Then search for `Firecrest` and subscribe to the `Firecrest v2` API
![CSCS Dev Portal](/assets/cscs_app_7.jpg)

Run `coman cscs login` and provide the Client id and Client secret when prompted. Both are securely stored in
your operating systems secure storage and don't need to be entered again. You also don't need to repeat this
step unless your keys change.


### CLI

To create a Coman config for a project/folder, run

```shell
coman init <folder>
```

This creates a `coman.toml` file that you can customize with settings if you wish.

Select what system you want to run on

```shell
coman cscs system ls # this lists systems you can see.
┌───────┬────────────────────────────┐
│ name  │ services_health            │
├───────┼────────────────────────────┤
│ eiger │ ╔══════════════╦═════════╗ │
│       │ ║ service_type ║ healthy ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Scheduler    ║ true    ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Ssh          ║ true    ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Filesystem   ║ true    ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Filesystem   ║ true    ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Filesystem   ║ true    ║ │
│       │ ╚══════════════╩═════════╝ │
├───────┼────────────────────────────┤
│ daint │ ╔══════════════╦═════════╗ │
│       │ ║ service_type ║ healthy ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Scheduler    ║ true    ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Ssh          ║ true    ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Filesystem   ║ true    ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Filesystem   ║ true    ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Filesystem   ║ true    ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Filesystem   ║ true    ║ │
│       │ ╠══════════════╬═════════╣ │
│       │ ║ Filesystem   ║ true    ║ │
│       │ ╚══════════════╩═════════╝ │
└───────┴────────────────────────────┘
```

Then set the system (e.g. `daint`) with 
```
coman cscs system set daint
```

To execute a job on CSCS, run a command like

```shell
coman cscs job submit -i ubuntu:latest -- echo test
```
This will run the command `echo test` using the `ubuntu:latest` docker image and default settings.
See `coman cscs job submit -h` for more options. This will also automatically set up an ssh connection for
the job (use `--no-ssh` to prevent this), see the [SSH](#ssh) section for more details.

You can list your jobs using

```shell
coman cscs job list
┌─────────┬────────────────┬──────────┬──────────┬────────────────────────────┬────────────────────────────┐
│ id      │ name           │ status   │ user     │ start_date                 │ end_date                   │
├─────────┼────────────────┼──────────┼──────────┼────────────────────────────┼────────────────────────────┤
│ 2104427 │ job-name       │ Finished │ user     │ 2025-11-19 12:13:26 +01:00 │ 2025-11-19 12:13:55 +01:00 │
└─────────┴────────────────┴──────────┴──────────┴────────────────────────────┴────────────────────────────┘
```

Get details for a job with
```shell
coman cscs job get <id>
┌───────────────┬────────────────────────────────────────────────────────┐
│ Id            │ 2127021                                                │
├───────────────┼────────────────────────────────────────────────────────┤
│ Name          │ job-name                                               │
├───────────────┼────────────────────────────────────────────────────────┤
│ Start Date    │ 2025-11-24 10:15:50 +01:00                             │
├───────────────┼────────────────────────────────────────────────────────┤
│ End Date      │ 2025-11-24 10:17:32 +01:00                             │
├───────────────┼────────────────────────────────────────────────────────┤
│ Status        │ Finished                                               │
├───────────────┼────────────────────────────────────────────────────────┤
│ Status Reason │ None                                                   │
├───────────────┼────────────────────────────────────────────────────────┤
│ Exit Code     │ 0                                                      │
├───────────────┼────────────────────────────────────────────────────────┤
│ stdin         │ /dev/null                                              │
├───────────────┼────────────────────────────────────────────────────────┤
│ stdout        │ /capstor/scratch/cscs/rgrubenm/coman/slurm-2127021.out │
├───────────────┼────────────────────────────────────────────────────────┤
│ stderr        │                                                        │
└───────────────┴────────────────────────────────────────────────────────┘ 
```

Get the logs from a job

```shell
coman cscs job log <id>
```

You can also manage files with coman.
List a remote directory:

```shell
coman cscs file list /capstor/scratch/cscs/your_user
```

Download a file:

```shell
coman cscs file download /capstor/scratch/cscs/your_user/your_file /local/target_file
```

Upload a file:

```shell
coman cscs file upload /my/local/file /capstor/scratch/cscs/your_user/your_file
```

You can set up shell completions as follows:

```shell
# Bash
mkdir -p ~/.local/share/bash-completion/completions
coman completions bash > ~/.local/share/bash-completion/completions/coman

# Bash (macOS/Homebrew)
mkdir -p $(brew --prefix)/etc/bash_completion.d/
coman completions bash > $(brew --prefix)/etc/bash_completion.d/coman.bash-completion

# Fish
mkdir -p ~/.config/fish/completions
coman completions fish > ~/.config/fish/completions/coman.fish

# Zsh
mkdir ~/.zfunc
# Then add the following lines to your `.zshrc` just before
# `compinit`:
# 
#         fpath+=~/.zfunc
coman completions zsh > ~/.zfunc/_coman
```
### TUI

To run the TUI, simply run `coman` without any arguments:

![TUI screenshot](/assets/tui_1.jpg)

The TUI should be pretty self-explanatory. It gives an overview of your jobs on the selected system,
refreshed every couple of seconds, lets you see the logs and all the other functionality of the CLI,
just in an interactive way.

### coman.toml config file

The config file options look as follows:

```toml
name = "myproject" # the name of the project, used to generate job names

[cscs]
# check https://docs.cscs.ch/access/firecrest/#firecrest-deployment-on-alps for possible system and platform combinations
current_system = "daint" # what system/cluster to execute commands on
current_platform = "HPC" # what platform to execute commands on (valid: HPC, ML or CW)
account = "..." # the project/group account to use on cscs
ssh_key = "path/to/ssh/public/key.pub" # To use a different public key for SSH connections, other than the default auto-detected id_dsa, id_rsa or id_ecdsa

image = "ubuntu" # default docker image to use

command = ["sleep", "1"] # command to execute within the container, i.e. the job you want to run

workdir = "/scratch" # working directory within container

# the sbatch script you want to execute
# this gets templated with values specified in the {{}} and {% %} expressions (see https://keats.github.io/tera/docs/#templates for
# more information on the template language). Note, this can also just be hardcoded without any template parameters.
# Available parameters:
#   name: the name of the job
#   environment_file: the path to the edf environment toml file in the cluster
#   command: the command to run
#   container_workdir: the working directory inside the container
sbatch_script_template = """
#!/bin/bash
#SBATCH --job-name={{name}}
#SBATCH --ntasks=1
#SBATCH --time=10:00
srun {% if environment_file %}--environment={{environment_file}}{% endif %} {{command}}
"""

# the edf environment toml file template
# this gets templated with values specified in the {{}} and {% %} expressions (see https://keats.github.io/tera/docs/#templates for
# more information on the template language). Note, this can also just be hardcoded without any template parameters.
# Available parameters:
#   edf_image: the container image to use, in edf format
#   container_workdir: the working directory to use within the container
#   env: a dictionary of key/value pairs for environment variables to set in the container
#   mount: a dictionary of key/value pairs for folders to mount to the container, with key being the path in the cluster and value being the path in the container
#   ssh_public_key: path to the ssh public key on the remote
edf_file_template = """
{% if edf_image %}image = "{{edf_image}}"{% endif %}
mounts = [{% for source, target in mount %}"{{source}}:{{target}}",{% endfor %}]
workdir = "{{container_workdir}}"

[env]
{% for key, value in env %}
{{key}} = "{{value}}"
{% endfor %}

[annotations]
{% if ssh_public_key %}
com.hooks.ssh.enabled = "true"
com.hooks.ssh.authorize_ssh_key = "{{ ssh_public_key }}"
com.hooks.ssh.port = 15263
"""

# set environment variables that should be passed to a job
[cscs.env]
ENV_VAR = "env_value"

```
#### Editing the config

You can edit the config file directly or (safer) use coman commands to do so:
```shell
coman config get cscs.current_system
```

```shell
coman config set cscs.current_system "daint"
```

### SSH

`coman cscs job submit` will automatically create an SSH connection for the job. It will search for an
`id_dsa.pub`, `id_rsa.pub` or `id_ecdsa.pub` file in your `.ssh` folder and use that for the connection,
unless you specify another key using the `--ssh-key` argument or the `cscs.ssh_key` setting in the config file.

Creating the ssh connection involves several steps, all handled by coman:

- Uploading your ssh public key into the remote coman project folder
- Setting the public key in the [CSCS SSH hook](https://docs.cscs.ch/software/container-engine/resource-hook/#ssh-hook)
- Uploading a squash file containing the coman executable to the remote coman project folder
- Mounting the coman squash file into the container so the coman executable is available in the container
- Creating an [iroh](https://github.com/n0-computer/iroh) secret key to use for the [QUIC tunnel](https://en.wikipedia.org/wiki/QUIC)
- Using the coman executable as the entrypoint of the container (wrapping the original command), which
  allows coman to create an iroh/QUIC tunnel for remote connections, as well as properly handling pid1
  process signals in the container
- Creating a local SSH config in the coman data dir (`~/.local/share/coman` by default) containing connection
  information and the correct iroh proxy command
- Including the SSH config in `.ssh/config` so it's accessible in other tools
- Garbage collecting old SSH connections for jobs that are not running anymore

## Development

### Prerequisites

Make sure you have:

- Rust and Cargo installed (get from https://rustup.rs/)
- OpenSSL development headers
- oas3-gen (install with `cargo install oas3-gen@0.21.1`)


```
cargo build
cargo run
```


### Install binaries

Build the binaries and
```
cargo build --release
```

Then copy it to the bin folder
```
sudo cp target/release/coman /usr/local/bin/
sudo chmod +x /usr/local/bin/coman
```

If you want to use cargo to install `coman`, make sure to remove any version of coman from `/usr/local/bin/` and run

```
cargo install --path ./coman
```
