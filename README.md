# joy2uinput

Maps your joypad to your keyboard on linux.

Works with any software! It presents a virtual keyboard and mouse to the kernel, so it doesn't matter if you're using X or Wayland, or something else, it will always work.

[![codecov](https://codecov.io/gh/lexbailey/joy2uinput/graph/badge.svg?token=6P1FPS9APZ)](https://codecov.io/gh/lexbailey/joy2uinput)

## Installation

TODO: get this packaged in Debian and the Arch linux AUR. (wanna help with this? awesome! see the packaging guide: [PACKAGING.md](PACKAGING.md))

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

If you want a release version, you can either do that in the normal cargo way, or use the build script which creates a ready-to-package version in a directory called `package_release`:

    ./build_release

## Config

config is read from

1. `$JOY2UINPUT_CONFDIR/joy2uinput.conf`, if it exists
2. or `$XDG_CONFIG_HOME/joy2uinput/joy2uinput.conf`, if `XDG_CONFIG_HOME` is not empty and the file exists
3. or `~/.config/joy2uinput/joy2uinput.conf`, if it exists
4. or `/opt/joy2uinput/joy2uinput.conf`, if it exists
5. or `/etc/joy2uinput/joy2uinput.conf` as a last resort
or nowhere, and the program exits with an error.

mapping files are searched for in a similar pattern. All `.j2umap` files from the first directory that exists in the search order for `joy2uinput.conf` are scanned for mapping files. Additionally `/etc/joy2uinput` is always scanned for mapping files if it exists.

The default config file `/etc/joy2uinput/joy2uinput.conf` looks like this

    # Arrows
    Up=key(Up)
    Down=key(Down)
    Left=key(Left)
    Right=key(Right)
    
    # Right thumb buttons
    A=key(Enter)
    B=key(Escape)
    X=key(LControl)
    Y=key(LShift)
    
    # Mouse is on the left stick and right shoulder buttons
    LeftX=axis(MouseX, 15)
    LeftY=axis(MouseY, 15)
    RShoulder=MouseButton(left)
    RTrigger=MouseButton(right)
    
    # Scroll with right stick
    RightX=axis(ScrollX, 0.5)
    RightY=axis(ScrollY, -0.5)
    
    # Misc
    Select=key(Tab)
    Start=key(Space)
    LShoulder=key(pgup)
    LTrigger=key(pgdn)

However, this is not all you need. joy2uinput depends on device specific mapping files (because USB gamepads can't agree on what each button is called)

These are tricky to write manually, use joy2u-mapgen if you can.

Here is an example from a cheap USB SNES-style joypad that I happen to have:

    button(1) = a
    button(2) = b
    button(0) = x
    button(3) = y
    button(9) = start
    button(8) = select
    button(4) = lshoulder
    button(5) = rshoulder
    axis_as_button(1,-32767) = up
    axis_as_button(1,32767) = down
    axis_as_button(0,-32767) = left
    axis_as_button(0,32767) = right

## Config and mapping language reference

Both the config file and the mapping files ignore blank lines and lines starting with `#`

The mapping files accepts lines of the form `joydev_event = joypad_event`

The config file accepts lines of the form `joypad_event = uinput_event`

### joydev_event
Raw events from joydev devices (used only in .j2umap files) are as follows:

    - button(N)
    - axis_as_button(N, VAL)
    - axis(N, MIN, MAX)

where N is the event ID number, VAL is the exact axis value that triggers the button press, and MIN/MAX are the limits of an axis range
it is recommended to use a tool such as joy2u-mapgen to produce these for you, since the numbers can only be determined empirically

### joypad_event
Events in the abstract internal namespace (.j2umap files map *to* these, the main config maps *from* these)

These events are just internal names, as such you can use them as you please

    Buttons
    - up
    - down
    - left
    - right
    - start
    - select
    - a
    - b
    - c
    - d
    - w
    - x
    - y
    - z
    - lshoulder
    - rshoulder
    - ltrigger
    - rtrigger
    - menu
    - home
    - lstick
    - rstick
    - plus
    - minus
    - custom_button(N)

    Axes
    - leftx
    - lefty
    - leftz
    - rightx
    - righty
    - rightz
    - throttle
    - brake
    - scrollx
    - scrolly
    - scrollz
    - roll
    - pitch
    - yaw
    - custom_axis(N)

where N can be almost none of the natural numbers
    

### uinput_event
The events to be sent to the virtual keyboard or mouse device (these are only used in the main config file)

These are the most numerous. Linux supports lots of event types, this is only a small subset of linux's support.
If you need a button that is supported in linux, but isn't listed here, please open an issue on github (see the
bugs/improvements section). Or, even better, send me a pull request! I'll be happy to add extra button names and
whatnot.

    - mousebutton(left)
    - mousebutton(right)
    - mousebutton(middle)
    - mousebutton(side)
    - mousebutton(extra)
    - mousebutton(forward)
    - mousebutton(back)

    - key(up)
    - key(down)
    - key(left)
    - key(right)
    - key(escape) or key(esc)
    - key(return) or key(enter)
    - key(space)
    - key(pageup)
    - key(pagedown)
    - key(home)
    - key(end)
    - key(delete)
    - key(tab)
    - key(lctrl) or key(lcontrol)
    - key(rctrl) or key(rcontrol)
    - key(lshift)
    - key(rshift)
    - key(lsuper)
    - key(rsuper)
    - key(lalt)
    - key(ralt)
    - key(menu)
    - key(volup) or key(volumeup)
    - key(voldown) or key(volumedown)

    - key(a)
    - key(b)
        ...
    - key(z)

    - key(0)
    - key(1)
        ...
    - key(9)

    - key(f1)
    - key(f2)
        ...
    - key(f24)

    - key(numpad0)
    - key(numpad1)
        ...
    - key(numpad9)

    - key(-)
    - key(equals)
    - key([)
    - key(])
    - key(;)
    - key(')
    - key(comma)
    - key(.)
    - key(/)
    - key(\)

    - axis(mousex,M)
    - axis(mousey,M)
    - axis(scrollx,M)
    - axis(scrolly,M)
    - axis(pageupdown,M)
    - axis(leftright,M)
    - axis(updown,M)
    - axis(volupdown,M)

    - toggle_enabled (not actually a uinput event, maps a button to enable or disable all other mappings)

(where M is a multiplier for controlling the speed of the input. M can be negative to invert an axis)

## FAQ

Q. How do I change the sensitivity of the analog inputs when I have them mapped to mouse movement?

A. Change the multiplier value in your `joy2uinput.conf` file. For example: `axis(mousex,10.0)` will move half as fast as `axis(mousex,20.0)`

---------

Q. How do I invert an axis output?

A. Change the multiplier value in your `joy2uinput.conf` from positive to negative (or vice versa) For example: `axis(mousex,-10.0)` will be the inverse of `axis(mousex,10.0)`

---------

Q. There is no appropriate name in the `joypad_input` list for one of the buttons or axes on my joypad, what should I do?

A1. Use the custom specifier, with a number of your choice. For example `custom_button(0)` or `custom_axis(1)`. You can map this key or axis as normal.

A2. Submit an issue or pull request to the github repo for joy2uinput to get additional names added (if they are generally useful)

---------

Q. I had to generate a mapping for my joypad because it didn't work out of the box. Would you like it?

A. Yes! Thankyou! Open an issue or pull request on the github repo for joy2uinput. Be sure to share the brand and model of controller, so I can check some information about it and make sure the .j2umap file looks okay.

---------

Q. I have two joypads connected, but one of them has an axis inverted and the other doesn't. what's going on?

A. Ahhh, sorry about that, it's probably a mistake in a mapping file. It happens. Have a look for the mapping file for your controller in the appropriate config directory, copy that file to your user config directory if it's not already there, then edit the file to swap the min and max values for the axis around. for example `axis(lefty,-32767,32767)` becomes `axis(lefty,32767,-32767)`. If there is a mistake in a default mapping file, please open an issue on the github page and I'll try and fix it.
