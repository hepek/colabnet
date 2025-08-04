Colabnet
========

Analyzes git logs for interesting patterns.

    colabnet
    Usage: colabnet <COMMAND>

    Commands:
      scan
      owners
      cousins
      help     Print this message or the help of the given subcommand(s)

    Options:
      -h, --help  Print help

# Scan

You can add special git parameter to your scan to delimit how long in git
history you want to go.

    colabnet scan -- --since=1year

# Owners

You can query file owners (people who edited it according to scanned logs)

    $ colabnet authors src/main.rs
    CHANGES AUTHOR
    ================================================================================
    866     Milan Markovic <zivotinja@gmail.com>
    
# Cousins

Print files that usually get edited together with this file in the same commit.

    $ colabnet cousins Cargo.toml
    TOTAL CHANGES: 11
    %       FILE
    ================================================================================
    100.00  Cargo.toml
    100.00  Cargo.lock
