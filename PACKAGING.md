# How to package joy2uinput

In my humble opinion: the ideal way to package joy2uinput is as follows:

Create three packages

1. joy2uinput:
    contains the joy2uinput binary and its man page
    also contains a default /etc/joy2uinput/joy2uinput.conf file that can be modified as appropriate for the platform this is being packaged for
    (if your platform needs to store the default config elsewhere, you might need to modify the config file search logic in the code)

    depends on:
        joydev (the kernel module)

2. joy2uinput-mappings:
    contains the .j2umap files normally found in /etc/joy2uinput/
    the joy2uinput package does not need to depend on this package. If your package system allows for recommended installation of other packages, then joy2uinput should recommend the joy2uinput-mappings package

3. joy2uinput-mapgen:
    contains the joy2u-mapgen binary and its man page
    as with the mappings package, this is not a dependency for joy2uinput, but should be recommended if possible.

    depends on:
        this package depends on all the same things as joy2uinput does.

Feel free to deviate from this pattern whenever appropriate for the system you are targeting.

## Other packaging related questions:

Q: Should joy2udev be started at boot/login by default?

A: I have no opinion on this. It could interfere with other things that want to use joysticks, so probably not, but feel free to disagree.
