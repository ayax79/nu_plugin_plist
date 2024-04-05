# Plist plugin for Nushell
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
