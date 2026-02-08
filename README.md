# Lopez

**Terminal-based ISO flasher for Linux.** Like balenaEtcher, but in the TUI and powered by `dd`.  
Linux only · keyboard-only · safety first.

---

## Features

- **Removable drive detection** — Lists only removable block devices via `lsblk`; system disks are excluded.
- **ISO / IMG selection** — Pick a file from the CLI (`--iso`) or type the path in the TUI. Validates `.iso` and `.img` and that the file exists.
- **Safety checks**
  - Must be run as **root** to flash (clear error if not).
  - **Blocks flashing** if the target device is mounted.
  - Requires **typing the full device path** (e.g. `/dev/sdb`) to confirm before writing.
- **Live progress** — Uses `dd` with `status=progress`; progress is streamed and shown in the TUI (no fake progress).
- **Clean failure handling** — Errors are reported in a result screen instead of silent failure.

## Requirements

- **Linux** (primary target: Arch Linux)
- **Root** for flashing (e.g. `sudo lopez`)
- **Rust** (stable) to build

## Installation

### From source

```bash
git clone https://github.com/kaleab49/Lopez.git
cd Lopez
cargo build --release
```

The binary will be at `target/release/lopez`. Optionally:

```bash
sudo cp target/release/lopez /usr/local/bin/
```

### With Cargo (after publish or from git)

```bash
cargo install --git https://github.com/kaleab49/Lopez.git
```

## Usage

Run with root when you intend to flash:

```bash
sudo lopez
```

Optional: pass an ISO (or IMG) to pre-fill the path:

```bash
sudo lopez --iso /path/to/image.iso
```

### TUI flow

1. **Drive list** — ↑/↓ to select a removable drive, **Enter** to confirm.
2. **ISO path** — Type the path to your `.iso` or `.img`, **Enter** to validate and continue.
3. **Confirm** — Type the **exact** device path shown (e.g. `/dev/sdb`) and **Enter** to start flashing.
4. **Progress** — `dd` output is shown live until completion.
5. **Result** — Success or error; **Esc** to go back to the drive list, **q** to quit.

### Keys

| Key   | Action                    |
|-------|---------------------------|
| ↑ / ↓ | Move selection            |
| Enter | Confirm / submit           |
| Esc   | Back or quit (on first screen) |
| q     | Quit                      |

## Safety

- **Do not** select a non-removable or system disk; the app only lists removable devices, but double-check.
- **Unmount** the target device (and its partitions) before flashing, or the app will refuse and show an error.
- You must **type the full device path** to confirm; copy-paste is allowed, but the string must match exactly.

## Building and testing

```bash
cargo build
cargo test
```

Run without root to see the “must be run as root” screen; run with `sudo` to use the full flow.

## License

MIT

## Repository

[https://github.com/kaleab49/Lopez](https://github.com/kaleab49/Lopez)
