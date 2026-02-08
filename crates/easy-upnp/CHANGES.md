# Changes in 0.3.1-RC

-   Update dependencies and fix version conflicts

# Changes in 0.3.0

-   Use latest versions of `igd-next` and `if-addrs`

    The `get_if_addrs` crate depends on a `winapi` version lower than 0.3, which prevents compilation for the Windows
    ARM target. In addition, both `igd` and `get_if_addrs` have been archived and are no longer maintained.

    Special thanks to TianHua Liu (Taoister39) for this PR!

-   Update dependencies

# Changes in 0.2.1

-   Update dependencies for security fixes

# Changes in 0.2.0

-   Add thiserror as dependency

-   Use thiserror instead raw errors

-   Properly propagate errors

-   Use re-exported Ipv4Cidr

-   Move error output to main, use Result in lib

-   Update dependencies

# Changes in 0.1.1

-   Update dependencies for security fixes

# Changes in 0.1.0

Initial release after splitting.
