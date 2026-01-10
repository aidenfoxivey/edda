(Disclaimer: this is tested on MacOS only.)

To connect meshtastic, first run:

```bash
ls -l /dev/tty.*
```

The device should look something like `/dev/tty.usbmodem2101`.

Then run:

```bash
cargo run -- <path to Meshtastic board>
```
