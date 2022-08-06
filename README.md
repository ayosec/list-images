# list-images

A CLI program to show images in a terminal, using the iTerm2 image protocol.

## Building

The program uses [libjpeg-turbo], so it has to be installed in the system.

For example, to build the program in a Debian system:

```console
$ sudo apt-get install libturbojpeg0-dev

$ export TURBOJPEG_SOURCE=pkg-config

$ export RUSTFLAGS="-C target-cpu=native"

$ cargo build --profile dist
```

The binary will be available in `target/dist/list-images`.

See [the `turbojpeg-sys` crate][turbojpeg-sys] for other options.

[libjpeg-turbo]: https://www.libjpeg-turbo.org/
[turbojpeg-sys]: https://github.com/honzasp/rust-turbojpeg/tree/HEAD/turbojpeg-sys
