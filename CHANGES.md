# Changes in 0.4.1-RC

-   Update dependencies to get security fixes

# Changes in 0.4.0

-   Upgrade clap for a better CLI experience

-   Support reading from stdin

-   Use tempfile to read from stdin

    This can be achieved by giving "-" as a filename. Internally, we create
    a temporary file, write the contents of stdin to it and read from it on
    each iteration. This way, we can use input from stdin even in daemon
    mode, where file handles to stdin, stdout, and stderr are closed.

-   Internal optimizations

-   Do not bail out on partly wrong config

    The config is read in entry by entry. If one entry is badly formatted,
    do not bail out and kill the whole application. Rather, write an error
    to the log and continue with the next entry.

-   Support config file in JSON format

-   Support CIDR notation in IP address

    This way, we can give an IP address as a range to match against
    connected interfaces. This is useful if we don't know our current IP
    address but we know the DHCP configuration of our router.

    Such an IP address might be `192.168.0.10` or `192.168.0.0/24` or even
    `192.168.0`.

    More examples can be found in the responsible library's documentation:
    https://docs.rs/cidr-utils/0.5.10/cidr_utils/index.html

-   Make pid file configurable

    Custom PID files may now be configured via the `--pid-file` option.

-   Support arbitrary CSV delimiters

    With the new `--csv-delimiter` option, you can configure your CSV file
    as you like. By default, we use the semicolon, but if you instead prefer
    a usual comma, you can just say so with `--csv-delimiter ','`.

    Please be aware that your shell might interpret the delimiter (for
    example, the semicolon is used in bash to separate two commands), so be
    sure to correctly escape it.

## Special Thanks

-   Succubyss

    For creating feature requests to add support for reading from stdin, JSON
    configs, and CIDR IP ranges.

# Changes in 0.3.1

-   Update dependencies to get security fixes

# Changes in 0.3.0

-   Make only-close actually work

    Due to a missing boolean check, the `--only-close-ports` flag did not
    work standalone, it must have been accompanied by
    `--close-ports-on-exit` to work.

    This restriction is fixed now, `--only-close-ports` now works standalone
    as intended.

# Changes in 0.2.0

-   Make daemonize specific to Unix

    Since the daemonize library only works on Unix like systems, make
    everything related to it also specific to Unix. This makes the program
    buildable and usable under Windows systems, too.

-   Add ctrlc as dependency

-   Use quitter channel to coordinate clean shutdown

-   Introduce method to delete ports

-   Support closing ports on exit

    The new command line flag `--close-ports-on-exit` triggers a last run
    through the config file on exit, where every defined port will be
    deleted from the open port mapping table on the router.

-   Support only closing ports

    The new command line flag `--only-close-ports` will not trigger the
    usual run to open ports, but instead just deletes the defined ports from
    the open ports mapping on the router and then exits.

## Special Thanks

-   Suyash Shandliya (PrisionMike)

    For notifying me about build problems on Windows machines. Hence,
    daemonize is now a UNIX-only feature.

# Changes in 0.1.0

-   Add first working prototype

-   Turn file path to absolute

    When daemonizing, the working directory will most likely be changed,
    therefore we need the absolute (canonical) path of the file to properly
    find it on the file system.

-   Daemonize and repeat program infinitely

    After each run, there is a waiting period of one minute.

-   Add flag to start program in foreground

    Some service monitoring programs expect daemons to run in foreground, so
    they can handle the state of the services with their own means.

-   Add onehot flag to stop program after one round

    This might be used for testing configurations, for example.

-   Add debug output if port in use

-   Add info output for each port mapping

-   Add customizable update interval

-   Rename ttl to duration

    In this context, "duration" is the correct term, ttl could be confusing.
