# `embedded_hal` based driver for the XBee Zigbee

Implements a few core frame types of Zigbee API

Setup Notes:
- Used Ubuntu 18.04
- install rust from website 
- rustup update
- rustup toolchain install nightly-2018-04-08
- rustup toolchain default nightly-2018-04-08
- rustup target: thumbv7em-none-eabihf
- rustup component: rls-preview
- install openocd
- install gdb-multiarch
- VSCode, https://marketplace.visualstudio.com/items?itemName=rust-lang.rust
- build code (cargo.lock file needed)
- when running gdb-multiarch ensure settings in launch.json:
"gdbpath": "gdb-multiarch",
            "debugger_args": [
                {"-ex":"set arch arm"},
            ],
