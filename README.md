# Plist plugin for Nushell

Note: this requires Nushell 0.89 or later

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
