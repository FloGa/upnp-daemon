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
