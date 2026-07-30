<div align="center">
  <h1>🔐 secure_safe</h1>
  <p><strong>An experimental, local encrypted-file vault written in Rust.</strong></p>
  <p>
    <img alt="Rust 2024" src="https://img.shields.io/badge/Rust-2024-DEA584?logo=rust&amp;logoColor=white">
    <img alt="XChaCha20-Poly1305" src="https://img.shields.io/badge/encryption-XChaCha20--Poly1305-6E56CF">
    <img alt="Argon2" src="https://img.shields.io/badge/key%20derivation-Argon2-1F8AC0">
    <img alt="Status: experimental" src="https://img.shields.io/badge/status-experimental-D73A49">
  </p>
</div>

> [!CAUTION]
> Do not use this version as the only copy of important files. The project is unaudited and pre-alpha. Although add and restore now durably commit their output before removing the prior copy, the complete vault workflow is not transactional and has not been audited for every filesystem and failure mode.

`secure_safe` compresses a file, encrypts it into a vault on the local filesystem, and then attempts to remove the plaintext source. It can later authenticate, decrypt, decompress, and restore the file to the path recorded when it was added.

## How it works

When a file is added, `secure_safe`:

1. Reads the entire file into memory and compresses it with Zstandard level 5.
2. Generates a random 16-byte salt and derives a 32-byte key from the password using the default Argon2 parameters.
3. Generates a random 24-byte nonce and encrypts the compressed bytes with XChaCha20-Poly1305.
4. Authenticates the original path as associated data.
5. Writes a mode-`0600` temporary vault entry and syncs its contents to stable storage.
6. Atomically publishes the entry under the source file's basename without replacing an existing entry, then syncs the vault directory.
7. Attempts to delete the original file only after the encrypted entry is durable.

Passwords and derived keys are held in zeroizing wrappers. Each file has its own salt and may use a different password; there is no global vault password or password database.

These are implementation details, not a security guarantee. The vault filename and recorded original path are **not encrypted**. Only the compressed file contents are encrypted; the path is plaintext but authenticated.

## Requirements and build

- A Unix-like operating system. The current implementation uses Unix-specific filesystem APIs.
- A Rust toolchain with Rust 2024 edition support.
- A C toolchain required by the `zstd` dependency.

Build the release binary:

```console
cargo build --release
./target/release/secure_safe help
```

Run the test suite during development:

```console
cargo test
```

## Usage

```text
secure_safe <COMMAND> [PATH]
```

| Command | Behavior |
| --- | --- |
| `add [PATH]` | Compresses and encrypts a file, stores it in the vault, then attempts to remove the source. Opens the source-file explorer when `PATH` is omitted. |
| `mo [NAME]` | Restores a vault entry to its recorded path, then overwrites and removes the vault entry. Opens the vault explorer when `NAME` is omitted. |
| `rm [NAME]` | After confirmation, overwrites the selected vault entry with zero bytes and removes it. Opens the vault explorer when `NAME` is omitted. |
| `check` | Tries to authenticate every regular file in the vault with the entered password. Successful filenames are printed; invalid or unauthenticated entries are reported to stderr. |
| `help` | Prints command help. |

The long forms `--add`, `--mo`, `--rm`, `--check`, and `--help` are also accepted. `-h` and `--h` are accepted as help aliases.

Every operation that asks for a password also asks for confirmation. A wrong password during `mo` prompts again; press `Ctrl+C` to stop retrying.

### Examples

```console
# Encrypt a file and attempt to remove the plaintext source.
# An absolute path makes the later restore location unambiguous.
secure_safe add /home/alice/Documents/secret.txt

# Restore the entry to /home/alice/Documents/secret.txt
secure_safe mo secret.txt

# Authenticate entries that use this password
secure_safe check

# Permanently remove an entry without restoring it
secure_safe rm secret.txt
```

`NAME` must be a bare vault filename, not a path. Because vault entries use only the source basename, `report.txt` is the entry name regardless of the source directory.

## Interactive file explorer

Run `add`, `mo`, or `rm` without the optional argument to choose a file interactively.

- `add` starts in the home directory and permits directory navigation.
- `mo` and `rm` show regular files directly inside the configured vault; directories are hidden and cannot be opened.

| Key | Action |
| --- | --- |
| `↑` / `↓` | Move the selection. |
| `→` | Open the selected directory in the `add` explorer. |
| `←` | Go to the parent directory in the `add` explorer. |
| `Enter` | Choose the selected file. |
| `q` / `Esc` | Quit. |

The explorer requires an interactive terminal.

## Configuration and storage

On the first non-help command, `secure_safe` creates:

| Path | Purpose |
| --- | --- |
| `~/.safe_dir/` | Default vault directory containing encrypted entries. |
| `~/secure_safe.settings` | TOML settings file containing the `enc_dir` path. |

To use another vault directory, edit `~/secure_safe.settings`:

```toml
enc_dir = "/absolute/path/to/my-vault"
```

The directory is created automatically when the settings are loaded.

### Entry format

Each vault entry currently contains, in order:

```text
16-byte salt
24-byte nonce
4-byte little-endian original-path length
original path bytes (plaintext, authenticated)
XChaCha20-Poly1305 ciphertext and authentication tag
```

The format has no magic bytes or version field and may change without migration support.

## Important limitations

- **No password recovery:** forgetting a file's password makes that file unrecoverable.
- **Not a backup:** a successful `add` is designed to delete the source. Keep independent backups of important data.
- **Non-transactional workflow:** add and restore sync their new file and parent directory before removing the prior copy, but each full operation still spans multiple filesystem actions. An interruption can leave redundant source, temporary, or vault files that require manual cleanup.
- **Whole-file memory use:** adding, restoring, and authenticating an entry load its encrypted or plaintext contents into memory. This is unsuitable for very large files.
- **Metadata exposure:** vault filenames reveal source basenames, and each entry stores its original path in plaintext.
- **Basename collisions:** two source files with the same basename map to the same vault entry. The second `add` fails while the first entry exists.
- **Relative paths:** the exact path supplied to `add` is recorded. If it is relative, restoration resolves it from the process's current working directory, which may not be the original directory.
- **Restore conflicts:** restoration can replace an existing file at the recorded path on Unix. It also fails if the sibling temporary path (with extension `secure_safe.tmp`) already exists or if the parent directory is missing.
- **Limited checking:** `check` authenticates ciphertext but does not decompress it, validate its eventual restore destination, or produce a failing process status merely because an individual entry fails authentication.
- **Best-effort erasure:** overwriting before unlinking does not guarantee physical erasure on SSDs, copy-on-write filesystems, journaled filesystems, snapshots, backups, or remote storage.
- **Terminal behavior:** password prompts and the file explorer expect an interactive terminal. Passwords must currently be entered twice even for restore and check operations.
- **Unaudited cryptography:** the design and implementation have not received an independent security review.

## Development priorities

Before this project should be considered for production use, it needs transactional and durable file operations, explicit restore-conflict handling, a versioned format, streaming I/O, clearer command outcomes and exit statuses, broader interruption and failure-path tests, and an independent security review.

## License

Licensed under the [Apache License 2.0](LICENSE).

<div align="center">
  <sub>Experimental cryptography code: inspect first, trust later.</sub>
</div>
