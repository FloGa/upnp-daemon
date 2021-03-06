# Changes since latest release

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
