# Change Logs

## v0.5.7 - not released

## v0.5.6 - 2017-06-25

- Now we can parse following command lines correctly:
    - ``` export OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include ```
    - ``` echo "`date` and `go version`" ```
    - ``` echo `date` and `go version` ```
- updated logger

## v0.5.5 - 2017-06-22

- Added `make` completion.
- Added `ssh` completion.
- Fixed an alias extension issue.
