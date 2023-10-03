# amd_epp_tool

This small tool is a simple CLI and TUI to configure the `scaling_governor` and `energy_performance_preference` settings 
exposed in `sysfs` by the Linux `amd_pstate_epp` driver. It sets the same profile uniformly across all CPUs. 

![TUI example setting](tui.png) 

## Usage:

It has 4 commands, setting a configuration usually requires `root` level access, so either run with `sudo` or make the binary `setuid root`.
 (`sudo chown root amd-epp-tool; sudo chmod +s amd-epp-tool`)
```
Read or change the amd_pstate_epp kernel driver settings

Usage: amd-epp-tool [COMMAND]

Commands:
  ui    interactive settings UI (default)
  get   get the current settings
  set   set new settings
  mon   monitor CPU speed
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Gotchas

This tool is built specifically to work with the `amd_pstate_epp` driver, which was introduced in Linux 6.3 or available as a patch against 5.x if you compiled your own kernel.

The allowed combination of `scaling_governor` and `energy_performance_preference (epp)` settings is dictated by the driver, if you receive a `device or resource busy` message, that usually means you've chosen an invalid configuration combination, eg. mixing performance and power-saving: 
```
> amd-epp-tool set -s=performance -e=power
Failed to save configuration, perhaps you need ROOT access
Error Device or resource busy (os error 16)
```

The tool doesn't prevent partial updates. If it fails to apply a setting, which usually means some incompatible `energy_performance_preference` combination which got rejected, it may have already updated your `scaling_governor`. It makes no effort to roll back to the prior state. 

## References

[Linux Guide amd-pstate](https://docs.kernel.org/admin-guide/pm/amd-pstate.html) 
