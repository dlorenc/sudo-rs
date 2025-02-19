# sudo-rs

A safety oriented and memory safe implementation of sudo and su written in Rust.

## ⚠️ WARNING

**Sudo-rs is currently under active development and is not suited for any
production environment. Using sudo-rs is only recommended for development and
testing purposes, but you should expect any system that has sudo-rs installed to
break easily and to not be secure.**

Sudo-rs currently only supports Linux-based operating systems, even if it compiles
on a different system, it will not run properly.

## Quick start

**We do not offer binary packages yet**

Sudo-rs is written in Rust. The minimum required Rust version is 1.70. If your
Linux distribution does not package that version (or a later one), you can always
install the most recent version through [rustup]. You also need the C development
files for PAM (`libpam0g-dev` on Debian, `pam-devel` on Fedora).

With dependencies installed, building sudo-rs is a simple matter of:
```
cargo build --release
```

This produces a binary `target/release/sudo`. However, this binary must have
the setuid flag set and must be owned by the root user in order to provide any
useful functionality. If you are unsure about how to set this up, then the
current version of sudo is not intended for you.

Sudo-rs needs the sudoers configuration file. The sudoers configuration file
will be loaded from `/etc/sudoers-rs` if that file exists, otherwise the
original `/etc/sudoers` location will be used. You must make sure that a valid
sudoers configuration exists at that location. For an explanation of the
sudoers syntax you can look at the
[original sudo man page](https://www.sudo.ws/docs/man/sudoers.man/).

[rustup]: https://rustup.rs/

## Differences from original sudo

sudo-rs supports less functionality than sudo. Some of this is by design. In
most cases you will get a clear error if you try something that is not
supported (e.g. use a configuration flag or command line option that is not
implemented).

Exceptions to the above, with respect to your `/etc/sudoers` configuration:

* `use_pty` is enabled by default, but can be disabled.
* `env_reset` is ignored --- this is always enabled.
* `visiblepw` is ignored --- this is always disabled.
* `verifypw` is currently ignored; a password is always necessary for `sudo -v`.
* `mail_badpass`, `always_set_home`, `always_query_group_plugin` and
  `match_group_by_gid` are not applicable to our implementation, but ignored for
  compatibility reasons.

Some other notable restrictions to be aware of:

* Some functionality is not yet supported; in particular `sudoedit`, `sudo --list`,
  and we also do not yet provide our own `visudo`.
* Sudo-rs always uses PAM for authentication at this time, your system must be
  set up for PAM. Sudo-rs will use the `sudo` service configuration. This also means
  that resource limits, umasks, etc have to be configured via PAM and not through
  the sudoers file.
* sudo-rs will not include the sendmail support of original sudo.
* Per-user, per-command, per-host `Defaults` sudoers entries are not supported.
* The sudoers file must be valid UTF-8.

If you find a common use case for original sudo missing, please create a feature
request for it in our issue tracker.

## Aim of the project

Our current target is to build a drop-in replacement for all common use cases of
sudo. For the sudoers config syntax this means that we support the default
configuration files of common Linux distributions. Our implementation should support
all commonly used command line options from the original sudo implementation.

Some parts of the original sudo are explicitly not in scope. Sudo has a large
and rich history and some of the features available in the original sudo
implementation are largely unused or only available for legacy platforms. In
order to determine which features make it we both consider whether the feature
is relevant for modern systems, and whether it will receive at very least
decent usage. Finally, of course, a feature should not compromise the safety of
the whole program.

Our `su` implementation is made using the building blocks we created for our
sudo implementation.  It will be suitable replacement for the `su` distributed
by [util-linux].

[util-linux]: https://github.com/util-linux/util-linux

## Future work

While our initial target is a drop-in replacement for most basic use cases of
sudo, our work may evolve beyond that target. We are also looking into
alternative ways to configure sudo without the sudoers config file syntax and to
extract parts of our work in usable crates for other people.
