# UPnP daemon

[![badge github]][url github]
[![badge crates.io]][url crates.io]
[![badge docs.rs]][url docs.rs]
[![badge license]][url license]

[badge github]: https://img.shields.io/badge/github-FloGa%2Fupnp--daemon-green
[badge crates.io]: https://img.shields.io/crates/v/upnp-daemon
[badge docs.rs]: https://img.shields.io/docsrs/upnp-daemon
[badge license]: https://img.shields.io/crates/l/upnp-daemon

[url github]: https://github.com/FloGa/upnp-daemon
[url crates.io]: https://crates.io/crates/upnp-daemon
[url docs.rs]: https://docs.rs/upnp-daemon
[url license]: https://github.com/FloGa/upnp-daemon/blob/develop/LICENSE

A daemon for continuously opening ports via UPnP.

## Motivation

There are quite some programs out there that need certain network ports to be
open to work properly, but do not provide the capability for opening them
automatically via UPnP. Sure, one could always argue about the security
implications that come with UPnP, but if you are willing to take the risk, it
is just annoying, that for example your webserver is not reachable from the
internet, because you forgot to open port 80, or your router rebooted and
cleared the table of open ports. Or your machine does for whatever reason not
have a static IP address, so you cannot add a consistent port mapping.

Because of this frustration, I created `upnp-daemon`, a small service written
in Rust, that will periodically check a file with your defined port mappings
and send them to your router. The main usage will be that you start it once
and let it run as a background service forever. The file with the port
mappings will be newly read in on each iteration, so you can add new mappings
on the fly.

## Installation

upnp-daemon can be installed easily through Cargo via `crates.io`:

```shell script
cargo install --locked upnp-daemon
```

Please note that the `--locked` flag is necessary here to have the exact same
dependencies as when the application was tagged and tested. Without it, you
might get more up-to-date versions of dependencies, but you have the risk of
undefined and unexpected behavior if the dependencies changed some
functionalities. The application might even fail to build if the public API of
a dependency changed too much.

Alternatively, pre-built binaries can be downloaded from the [GitHub
releases][gh-releases] page.

[gh-releases]: https://github.com/FloGa/upnp-daemon/releases

## Usage

<!--% !cargo --quiet run -- --help | tail -n+3 %-->

```text
Usage: upnp-daemon [OPTIONS] --file <FILE>

Options:
  -f, --file <FILE>                    The file (or "-" for stdin) with the port descriptions
      --format <FORMAT>                The format of the configuration file [default: csv] [possible values: csv, json]
  -d, --csv-delimiter <CSV_DELIMITER>  Field delimiter when using CSV files [default: ;]
  -F, --foreground                     Run in foreground instead of forking to background
  -1, --oneshot                        Run just one time instead of continuously
  -n, --interval <INTERVAL>            Specify update interval in seconds [default: 60]
      --close-ports-on-exit            Close specified ports on program exit
      --only-close-ports               Only close specified ports and exit
      --pid-file <PID_FILE>            Absolute path to PID file for daemon mode [default: /tmp/upnp-daemon.pid]
  -h, --help                           Print help
  -V, --version                        Print version
```

In the most basic case, a call might look like so:

```shell script
upnp-daemon --file ports.csv
```

This will start a background process (daemon) that reads in port mappings from
a CSV file (see [config file format](#config-file-format)) every minute and
ask the appropriate routers to open those ports.

The PID of the process will be written to `/tmp/upnp-daemon.pid` by default
and locked exclusively, so that only one instance is running at a time. To
quit it, kill the PID that is written in this file.

Bash can do it like so:

```shell script
kill $(</tmp/upnp-daemon.pid)
```

If you want the PID to be written to another file, maybe because you want to
have multiple running instances intentionally, or because your daemon watcher
expects the file in another place, you can use the `--pid-file` option to
choose your own file path. Due to technical reasons, this PID file must always
be given as an absolute path. Please be aware that this application does not
create folders, so the parent folder of the PID file needs to exist
beforehand. Also, of course, the user running the application needs to have
write permission to the folder.

**A note to Windows users:** The `daemonize` library that is used to send this
program to the background, does only work on Unix like systems. You can still
install and use the program on Windows, but it will behave as if you started
it with the `--foreground` option (see [below](#foreground-operation)).
Therefore, you will also not see the `--pid-file` option on Windows since it
has no use there.

### Reading from standard input

Depending on the actual use case, there might be the need to read in the ports
configuration from stdin. In this case, you can give the pseudo filename "-",
like so:

```shell script
generate-configuration | upnp-daemon --file -
```

The configuration generated by the imaginary `generate-configuration` needs to
have the same format as described in chapter [config file
format](#config-file-format).

**Note:** If you actually want to read in a file with the name `-`, give it as
an absolute or relative file path, like so:

```shell script
upnp-daemon --file ./-
```

### Foreground Operation

Some service monitors expect services to start in the foreground, so they can
handle them with their own custom functions. For this use case, you can use
the `foreground` flag, like so:

```shell script
upnp-daemon --foreground --file ports.csv
```

This will leave the program running in the foreground. You can terminate it by
issuing a `SIGINT` (Ctrl-C), for example.

**A note to Windows users:** This option flag does not exist in the Windows
version of this program. Instead, foreground operation is the default
operation mode, since due to technical limitations, it cannot be sent to the
background there.

### Oneshot Mode

If you just want to test your configuration, without letting the daemon run
forever, you can use the `oneshot` flag, like so:

```shell script
upnp-daemon --foreground --oneshot --file ports.csv
```

You could of course leave off the `foreground` flag, but then you will not
know when the process has finished, which could take some time, depending on
the size of the mapping file.

### Closing Ports

If you want to close your opened ports when the program exits, you can use the
`close-ports-on-exit` flag, like so:

```shell script
upnp-daemon --close-ports-on-exit --file ports.csv
```

If the program later terminates, either by using the `kill` command or by
sending a `SIGINT` in foreground mode, the currently defined ports in the
configuration file will be closed. Errors will be logged, but are not fatal,
so they will not cause the program to panic. Those errors might arise, for
example, when a port has not been opened in the first place.

If you just want to close all defined ports, without even running the main
program, you can use the `--only-close-ports` flag, like so:

```shell script
upnp-daemon --foreground --only-close-ports --file ports.csv
```

The `foreground` flag here is optional, but it is useful if you need to know
when all ports have been closed, since the program only terminates then.

### Logging

If you want to activate logging to have a better understanding what the
program does under the hood, you need to set the environment variable
`RUST_LOG`, like so:

```shell script
RUST_LOG=info upnp-daemon --foreground --file ports.csv
```

To make the logger even more verbose, try to set the log level to `debug`:

```shell script
RUST_LOG=debug upnp-daemon --foreground --file ports.csv
```

Please note that it does not make sense to activate logging without using
`foreground`, since the output (stdout as well as stderr) will not be saved in
daemon mode. This might change in a future release.

## Config File Format

The config file can be given as either CSV (default for now) or JSON (with
`--format json`). The names and contents of the fields are always the same.

### CSV

The format of the port mapping file is a simple CSV file, like the following
example:

```text
address;port;protocol;duration;comment
192.168.0.10;12345;UDP;60;Test 1
;12346;TCP;60;Test 2
```

Please note that the first line is mandatory at the moment, it is needed to
accurately map the fields to the internal options.

With the `--csv-delimiter` option, you can choose an arbitrary character to be
used as a field delimiter in your CSV file. By default, we use the semicolon,
but if you instead prefer a usual comma, you can just say so with
`--csv-delimiter ','`.

Please be aware that your shell might interpret the delimiter (for example,
the semicolon is used in bash to separate two commands), so be sure to
correctly escape it.

### JSON

A config file in JSON format with the above contents could look like this:

```json
[
  {
    "address": "192.168.0.10",
    "port": 12345,
    "protocol": "UDP",
    "duration": 60,
    "comment": "Test 1"
  },
  {
    "address": null,
    "port": 12346,
    "protocol": "TCP",
    "duration": 60,
    "comment": "Test 2"
  }
]
```

The line breaks and indentations are just for readability, you can format it
any way you like. You can even go as far and feed in a single line, as long as
it is a valid JSON array with all required fields in it.

Since `address` is `null` in the second entry, it can also be left out
completely if you prefer. Also, any key that is not documented below will be
silently ignored, so you might use them as internal comments for yourself. So
a config might also look like:

```json
[
  {
    "rationale": "This port is needed for an awesome application!",
    "may-be-deleted": false,
    "port": 12347,
    "protocol": "TCP",
    "duration": 60,
    "comment": "Test 3"
  }
]
```

The keys `rationale` and `may-be-deleted` will be ignored by the daemon.

Also, please note that even if you want to add just one port mapping, you need
to specify a JSON array.

### Fields

-   address

    The IP address for which the port mapping should be added. This field can
    be empty, in which case every connected interface will be tried, until one
    gateway reports success. Useful if the IP address is dynamic and not
    consistent over reboots.
    
    Fill in an IP address if you want to add a port mapping for a foreign
    device, or if you know your machine's address and want to slightly speed
    up the process.

    You can also enter the IP address in CIDR notation. In that case, the IP
    range is checked against all connected interfaces and only matching ones
    are considered. This is useful if you don't know your current IP address
    (or it might change from time to time), but you know the DHCP
    configuration of your router.

    Such an IP address might be `192.168.0.10` or `192.168.0.0/24` or even
    `192.168.0`.

    More examples can be found in the responsible library's documentation:
    <https://docs.rs/cidr-utils/0.5.10/cidr_utils/index.html>

-   port

    The port number to open for the given IP address. Note that upnp-daemon is
    greedy at the moment, if a port mapping is already in place, it will be
    deleted and re-added with the given IP address. This might be configurable
    in a future release.

-   protocol

    The protocol for which the given port will be opened. Possible values are
    `UDP` and `TCP`.

-   duration

    The lease duration for the port mapping in seconds. Please note that some
    UPnP capable routers might choose to ignore this value, so do not
    exclusively rely on this.

-   comment

    A comment about the reason for the port mapping. Will be stored together
    with the mapping in the router.
