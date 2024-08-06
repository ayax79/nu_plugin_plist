# Plist plugin for Nushell

> [!IMPORTANT]
>
> The functionality of the plist plugin is moving to [nu_plugin_formats](https://github.com/nushell/nushell/tree/main/crates/nu_plugin_formats) for nushell release 0.97.
> Release 0.96 is the final release of this plugin.

Provides the ability to read and write Apple plists.

To read a plist:
```nushell
open /System/Library/LaunchDaemons/bootps.plist
```

or 

```
cat /System/Library/LaunchDaemons/bootps.plist | from plist
``` 

to write a plist:

```
ps | to plist
```

Note: this requires Nushell 0.92 or later

To install:

```
> cargo install --path .
```

To register (from inside Nushell):
```
> register <path to installed plugin> 
```

Usage:
```
open file.plist 
```

```
open --raw file.plist | from plist
```
