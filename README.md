<!--
SPDX-FileCopyrightText: 2026 roolrz
SPDX-License-Identifier: Apache-2.0
-->

# coreutils

Small filesystem and stream utilities for Native userspace.

The repository contains cat, chmod, cp, echo, ln, ls, mkdir, mv, rm, rmdir and touch.
Each command has its own Cargo package; component.json owns the delivered executables.
Filesystem access uses the Native Rust standard-library port and the caller’s namespace.

See the [implementation entry](component.json) and [component declaration](component.json)
for the public interfaces, build dependencies and delivered artifacts.

## Usage

In a Native session with the required capabilities:

```sh
ls /system/bin
cat /etc/fstab
cp --help
```

## Boundaries

These are the implemented HypeR utilities, not a claim of complete GNU coreutils
compatibility. Consult each command’s help. Partial listing failures preserve successful
output and return failure; copying symlinks follows the documented overwrite/no-clobber
behavior.

## Build and test

Use this checkout inside the [repo-managed workspace](https://github.com/HypeR-Proj/HypeR).
From this repository directory:

```sh
make
make test
```

An absolute `OUT=/path/to/out` selects a shared output directory. The module
entry delegates to build/system, which prepares the installed SDK and declared
dependencies. Native binaries require HypeR; host module checks cover the tests
selected by component.json. A standalone clone does not contain the workspace’s
other source repositories or Native SDK.


`ls` preserves readable entries when a child disappears or its backing service
fails. Such partial listings report diagnostics and exit with status 1; complete
listings exit with status 0. Output errors remain failures. The listing library
exposes these as `ListingOutcome` rather than an inverted success boolean.
