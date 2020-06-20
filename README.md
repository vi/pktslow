# pktslow
Virtual network for slowing down or dropping packets selectively

# Instructions

1. Build (or download pre-built version from Github releases) `pktslow`
2. Start it as root, specifying two interfaces
3. Move one of those interfaces into another [Linux network namespace](https://man7.org/linux/man-pages/man8/ip-netns.8.html)
4. Set up addresses and routing, like you typically do with [veth](https://man7.org/linux/man-pages/man4/veth.4.html)
5. Specify interactive commands to `pktslow`'s stdin. You may want to issue `delay 0` or `setup 0 1 0` to remove the default 50ms delay. Then you probably would use `mon` command to dump some bytes from packets (to find the byte you need). Then you'll `setup` the matcher (only one masked byte is supported). Then you'll `delay` or `drop` or `ramp`.

(I don't belive the program would be useful enough to warrant writing a proper manual for it)

# Usage messages

Main usage message:

```
Usage: pktslow <tun1n> <tun2n>

simple program that creates a veth-like pair of TUN interfaces and allows to selectively delay some packets  Other options are specified as interactive stdin commands

Options:
  --help            display usage information
```

Interactive commands usage message:

```
> --help
Usage:  <command> [<args>]

interactive options

Options:
  --help            display usage information

Commands:
  quit              Exit from process
  delay             Adjust the delay
  ramp              Ramp the delay
  mon               Print packets content to stdout
  stats             Show statistics
  setup             Setup matcher that would decide whether to delay packets
  drop              Drop matching packets instead of delaying them. Reset with
                    `delay` command.
```
