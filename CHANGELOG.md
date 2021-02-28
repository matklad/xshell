# Changelog

## 0.1.9

- `mktemp_d` creates an (insecure, world readable) temporary directory.
- Fix cp docs

## 0.1.8

- Add option to not echo command at all
- Add option to censor command contents when echoing
- Add docs

## 0.1.7

- `cp(foo, bar)` copies `foo` _into_ `bar`, if `bar` is an existing directory.
- Tweak reading API

## 0.1.6

- `.read()` chomps `\r\n` on Windows.
- Prevent cwd/env races when using `.read()` or `.run()`
- Better spans in error messages

## 0.1.5

- Improve proc-macro error messages.

## 0.1.4

- No changelog until this point :(
