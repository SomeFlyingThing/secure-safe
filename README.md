<div align="center">
  <h1>🔐 secure_safe</h1>
  <p><strong>A fully local encrypted-file vault written in Rust.</strong></p>
  <p>
    <img alt="Rust 2024" src="https://img.shields.io/badge/Rust-2024-DEA584?logo=rust&amp;logoColor=white">
    <img alt="ChaCha20-Poly1305" src="https://img.shields.io/badge/encryption-ChaCha20--Poly1305-6E56CF">
    <img alt="Argon2" src="https://img.shields.io/badge/key%20derivation-Argon2-1F8AC0">
  </p>
</div>

`secure_safe` stores files in a local encrypted vault. Files are encrypted with ChaCha20-Poly1305 using a key derived from the login password with Argon2. The original path is stored in the entry header and authenticated as AEAD associated data, so changing it causes decryption to fail.

## How it works

On first use, `secure_safe` asks for a password, generates a random 16-byte salt, derives a 32-byte key with Argon2, stores the salt in `~/salt.sf`, and creates an encrypted `pass-check` entry used to verify future logins.

When a file is added:

1. The file is read into memory.
2. Its original path is placed in the entry header.
3. A random 12-byte nonce is generated.
4. The file contents are encrypted with ChaCha20-Poly1305.
5. The path bytes are supplied as additional authenticated data (AAD), so the path remains readable but cannot be changed without invalidating authentication.
6. A BLAKE3 hash of the encrypted payload is stored in the header as an additional corruption check.
7. The vault entry is written through a temporary mode-`0600` file, synced, renamed into place, and the vault directory is synced.
8. After the vault entry has been written successfully, the original plaintext file is removed.

Passwords and derived keys are held in zeroizing wrappers.

## Build

The current implementation targets Unix-like systems and uses Unix-specific filesystem APIs.

```console
cargo build --release
```

Run it with:

```console
./target/release/secure_safe <COMMAND> <ARGUMENT>
```

Run the tests with:

```console
cargo test
```

or, with cargo-nextest installed:

```console
cargo nextest run
```

## Commands

The CLI currently accepts these commands exactly:

| Command | Behavior |
| --- | --- |
| `add <PATH>` | Encrypts the file, stores it in `~/secure-safe` under its basename, then removes the original file. |
| `restore <NAME>` | Loads the named vault entry, verifies and decrypts it, and writes the plaintext back to the original path recorded in its header. The encrypted vault entry is kept. |
| `delete <PATH>` | Deletes a vault file after confirmation. The supplied path must resolve inside `~/secure-safe`. Depending on configuration, the file may be overwritten with zeroes before unlinking. |
| `watchd <DIRECTORY>` | Watches a directory and automatically encrypts a file after it is closed following a write, or when a file is moved into the directory. The encrypted entry is stored in `~/secure-safe`, then the plaintext source file is removed. |
| `about` | Prints a short description of the project. |

All commands go through password authentication before executing.

### Examples

```console
# Add a file to the vault
secure_safe add /home/alice/Documents/secret.txt

# Restore ~/secure-safe/secret.txt to the path recorded when it was added
secure_safe restore secret.txt

# Delete a vault entry
secure_safe delete ~/secure-safe/secret.txt

# Watch a directory and secure each completed file written or moved into it
secure_safe watchd /home/alice/Documents/to-secure

# Print the project description
secure_safe about
```

For `restore`, `NAME` must be a bare filename such as `secret.txt`; paths such as `../secret.txt` are rejected.

### Watching a directory

`watchd` runs until it is stopped. It watches only the specified directory, not its subdirectories. A file is secured when it is closed after writing or moved into the watched directory. Directories and other non-file events are ignored. As with `add`, the original plaintext file is deleted only after its encrypted vault entry has been written successfully.

## Storage

The vault directory is:

```text
~/secure-safe/
```

The password-derivation salt is stored at:

```text
~/salt.sf
```

The optional configuration file is:

```text
~/secure-safe.toml
```

It currently supports:

```toml
overwrite_times = 0
```

`overwrite_times` controls how many zero-overwrite passes `delete` performs before unlinking the file. The default is `0`.

## Entry format

Each encrypted vault entry is stored in this order:

```text
11 bytes   marker: "secure_safe"
8 bytes    original-path length, big-endian u64
N bytes    original path, plaintext and authenticated as AAD
32 bytes   BLAKE3 hash of the encrypted payload
12 bytes   ChaCha20-Poly1305 nonce
M bytes    ciphertext + 16-byte Poly1305 authentication tag
```

The BLAKE3 hash covers the encrypted payload beginning with the nonce. ChaCha20-Poly1305 provides the keyed authentication: both the ciphertext and the path supplied as AAD must match for decryption to succeed.

## Restore behavior

`restore <NAME>` reads the path from the encrypted entry's header, authenticates that exact path through ChaCha20-Poly1305 AAD, decrypts the contents, and writes them through a temporary file before renaming it to the recorded destination.

Because the entry name is based on the original file's basename, a file added as:

```text
/home/alice/Documents/report.txt
```

is stored as:

```text
~/secure-safe/report.txt
```

while its full original path remains recorded in the entry header for restoration.

## License

Licensed under the [Apache License 2.0](LICENSE).
