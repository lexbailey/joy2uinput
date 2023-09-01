# joy2uinput

Maps your joypad to your keyboard on linux.

Works with any software! It presents a virtual keyboard and mouse to the kernel, so it doesn't matter if you're using X or Wayland, or something else, it will always work.

## Installation

TODO: get this packaged in Debian and the Arch linux AUR. (wanna help with this? awesome! see the packaging guide: [PACKAGING.md])

currently, to install this, you need to clone and build it from scratch (see "Building" section below)

## Running

simply run this command:

    joy2uinput

It will give you useful output!

It might give some errors about mapping files being missing, if that's the case, try joy2u-mapgen

    joy2u-mapgen

## Building

This project is built with cargo

    git clone git@github.com:lexbailey/joy2uinput.git
    cd joy2uinput
    cargo build

The results should be in the `target` directory

If you want a relase version, you can either do that in the normal cargo way, or use the build script which creates a ready-to-package version in a directory called `package_release`:

    ./build_release
