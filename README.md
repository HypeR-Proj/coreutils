<!--
SPDX-FileCopyrightText: 2026 roolrz
SPDX-License-Identifier: Apache-2.0
-->

# coreutils

A HypeR component. See [HypeR](https://github.com/HypeR-Proj/HypeR) for
workspace setup and the installed SDK build boundary.

## Workspace tests

Use the [HypeR workspace](https://github.com/HypeR-Proj/HypeR) to resolve dependencies:

```sh
python3 project.py test coreutils
```

Paths in the documentation are relative to the repo-managed workspace root.
Edit this repository and prepare the workspace again before building.

`ls` preserves readable entries when a child disappears or its backing service
fails. Such partial listings report diagnostics and exit with status 1; complete
listings exit with status 0. Output errors remain failures. The listing library
exposes these as `ListingOutcome` rather than an inverted success boolean.
