# Lopez

**Terminal-based ISO flasher for Linux.** Like balenaEtcher, but in the TUI and powered by `dd`.  
Linux only · keyboard-only · safety first.

---

## Features

- **Removable drive detection** — Lists only removable block devices via `lsblk`; system disks are excluded. Shows size and model.
- **ISO / IMG selection** — Pick a file from the CLI (`--iso`) or type the path in the TUI. Validates `.iso` and `.img` and that the file exists. Live fuzzy-ish matching against common locations (`/home`, `/mnt`, `/media`, …) as you type.
- **Safety checks**
  - Must be run as **root** to flash (clear error if not).
  - **Blocks flashing** if the target device — or **any of its partitions** — is mounted.
  - **Refuses to flash** if the image is larger than the target device.
  - Requires **typing the full device path** (e.g. `/dev/sdb`) to confirm before writing.
- **Live progress** — Uses `dd` with `status=progress`; output is streamed live and rendered as a progress bar (bytes copied / total, percentage) — no fake progress.
- **Cancellation** — `Esc` during flash kills `dd` immediately and leaves the drive untouched-but-partial (no dangling writer; the app never exits while `dd` is still running).
- **Clean failure handling** — Errors (not root, mounted device, wrong confirmation, oversize image, `dd` failure) are reported in place or on a result screen.

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

Check flags with `lopez --help`; version with `lopez --version`.

### TUI flow

1. **Drive list** — ↑/↓ to select a removable drive, **Enter** to confirm. `q`/`Esc` quits.
2. **ISO path** — Type the path to your `.iso` or `.img` (matches stream in live), **Enter** to validate and continue, `Esc` to go back.
3. **Confirm** — Type the **exact** device path shown (e.g. `/dev/sdb`) and **Enter** to start flashing. Issues (not root, device/partition mounted, image too large, wrong path) are shown inline instead of starting `dd`.
4. **Progress** — Percentage + bytes written, fed directly from `dd` output. **Esc** cancels (kills `dd` immediately).
5. **Result** — Success or error; **Esc** to go back to the drive list, **q** to quit.

### Keys

| Key    | Action                                        |
|--------|-----------------------------------------------|
| ↑ / ↓  | Move selection                                |
| Enter  | Confirm / submit                              |
| Esc    | Back, or **cancel the flash** while flashing  |
| Tab    | Take the highlighted file from the match list |
| Ctrl+Q | Quit while typing (ISO path / confirmation)   |
| q      | Quit on list/result screens                   |

Note: `q` is a regular character while typing paths (so paths containing `q` work); use `Ctrl+Q` there. Quitting is disabled while `dd` is running — cancel with `Esc` first.

## Safety

- **Do not** select a non-removable or system disk; the app only lists removable devices, but double-check.
- **Unmount** the target device (and **all of its partitions**) before flashing, or the app will refuse and show an error — the check covers partitions like `/dev/sdb1`, not just the whole disk.
- The app refuses to write an image **larger than the target device**.
- You must **type the full device path** to confirm; copy-paste is allowed, but the string must match exactly.
- While `dd` is running the app can't quit accidentally; `Esc` kills `dd` before returning to the drive list.

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
