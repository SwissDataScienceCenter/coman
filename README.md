# coman

[![Lint and Test](https://github.com/SwissDataScienceCenter/coman/actions/workflows/test.yaml/badge.svg)](https://github.com/SwissDataScienceCenter/coman/actions/workflows/test.yaml)
[![Release](https://github.com/SwissDataScienceCenter/coman/actions/workflows/release.yaml/badge.svg)](https://github.com/SwissDataScienceCenter/coman/actions/workflows/release.yaml)

Compute Manager for managing HPC compute


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
See `coman cscs job submit -h` for more options.

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


### TUI

To run the TUI, simply run `coman` without any arguments:

![TUI screenshot](/assets/tui_1.jpg)

The TUI should be pretty self-explanatory. It gives an overview of your jobs on the selected system,
refreshed every couple of seconds, lets you see the logs and all the other functionality of the CLI,
just in an interactive way.



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