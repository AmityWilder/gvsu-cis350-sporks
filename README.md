# The Sporks

Management software for generating schedules based on tasks, timeslots, and availability.

# Team Members and Roles

* [Conor Buchkowski](https://github.com/Conbear/CIS350-HW2-Buchkowski) (Python frontend)
* [Henry Lachman](https://github.com/AmityWilder/CIS350-HW2-Lachman) (Rust backend)

# Prerequisites

- Python

# Run Instuctions

## Debug

With `python` installed on your machine, in the `gvsu-cis350-sporks` directory:
(**Note:** `cargo build` requires having `rustc` and `cargo` installed)
```bash
cargo build # skip if you already have a debug instance of the Rust server already built
python ./src/clt/GUITests.py
```

To build using a downloaded debug binary of the Rust server, create a `target/debug` directory within `gvsu-cis350-sporks` and place `gvsu-cis350-sporks.exe` inside it.
Then run
```bash
python ./src/clt/GUITests.py
```

## Release

There is no release version of the software yet.
