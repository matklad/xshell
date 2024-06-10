# Changelog

## Unreleased

- MSRV is raised to 1.63.0

## 0.2.6

- Implement `Clone` for `Shell`.

## 0.2.5

- Improve error message when a working directory for `cmd!` does not exist.

## 0.2.3

- Fix bug where `Cmd::run` would ignore specified stdin.

## 0.2.2

- Add `Shell::path_exists`.

## 0.2.1

- `Shell::remove_path` returns `Ok` if the path does not exist (ie the function
  is now idempotent).

## 0.2.0

A major release with significant changes to the API:

- All global state is removed in favor of explicitly passing a `Shell` instance.
- Some methods are renamed to better match Rust naming conventions.
- New APIs for controlling working directory and environment.
- MSRV is raised to 1.59.0.
- Improved reliability across the board: the crate aims to become a dependable
  1.0 tool in the future (no ETA).
- This is expected to be the last *large* API reshuffle.

## 0.1.17

- Allow panics to transparently pass through xshell calls.
  This removes some internal lock poisoned errors.

## 0.1.16

- Add `xshell::hard_link`.

## 0.1.15

- Correctly handle multiple internal read guards.

## 0.1.14

- Correctly handle commands name starting with quote.

## 0.1.13

- Add `ignore_stdout`, `ignore_stderr` functions.

## 0.1.12

- Add `env`, `env_revome`, `env_clear` functions.

## 0.1.11

- `write_file` now creates the intervening directory path if it doesn't exit.

## 0.1.10

- `echo_cmd` output goes to stderr, not stdout.

## 0.1.9

- `mktemp_d` creates an (insecure, world readable) temporary directory.
- Fix cp docs.

## 0.1.8

- Add option to not echo command at all.
- Add option to censor command contents when echoing.
- Add docs.

## 0.1.7

- `cp(foo, bar)` copies `foo` _into_ `bar`, if `bar` is an existing directory.
- Tweak reading API.

## 0.1.6

- `.read()` chomps `\r\n` on Windows.
- Prevent cwd/env races when using `.read()` or `.run()`.
- Better spans in error messages.

## 0.1.5

- Improve proc-macro error messages.

## 0.1.4

- No changelog until this point :(
