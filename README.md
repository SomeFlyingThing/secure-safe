<div align="center">
  <h1>🔐 secure_safe</h1>
  <p><strong>An experimental local encrypted-file vault written in Rust.</strong></p>
  <p>
    <img alt="Rust 2024" src="https://img.shields.io/badge/Rust-2024-DEA584?logo=rust&amp;logoColor=white">
    <img alt="XChaCha20-Poly1305" src="https://img.shields.io/badge/encryption-XChaCha20--Poly1305-6E56CF">
    <img alt="Argon2" src="https://img.shields.io/badge/key%20derivation-Argon2-1F8AC0">
    <img alt="Status: unsafe pre-alpha" src="https://img.shields.io/badge/status-unsafe%20pre--alpha-D73A49">
  </p>
</div>

<hr>

<blockquote>
  <p><strong>Do not use this version with important files.</strong> The current <code>add</code> path contains a critical logic bug: after creating the encrypted vault entry, it targets that new vault entry for deletion instead of deleting the source plaintext. With a simple filename, the encrypted copy is removed and the plaintext remains. This project is not ready to protect real data.</p>
</blockquote>

<h2>Intended design</h2>

<table>
  <tbody>
    <tr>
      <td>🗜️ <strong>Compression</strong></td>
      <td>Zstandard level 5 before encryption.</td>
    </tr>
    <tr>
      <td>🧂 <strong>Key derivation</strong></td>
      <td>Argon2 with a fresh random 16-byte salt per file.</td>
    </tr>
    <tr>
      <td>🔒 <strong>Encryption</strong></td>
      <td>Authenticated XChaCha20-Poly1305 with a fresh 24-byte nonce.</td>
    </tr>
    <tr>
      <td>📍 <strong>Path binding</strong></td>
      <td>The original path is stored and authenticated as associated data.</td>
    </tr>
    <tr>
      <td>🧹 <strong>Secret handling</strong></td>
      <td>Derived keys and entered passwords use zeroizing wrappers.</td>
    </tr>
  </tbody>
</table>

<p>These primitives describe the implementation, not a security guarantee. The program has not been audited, and its current file lifecycle is unsafe.</p>

<h2>Current commands</h2>

<pre><code>secure_safe &lt;COMMAND&gt; [PATH]</code></pre>

<table>
  <thead>
    <tr>
      <th align="left">Command</th>
      <th align="left">Current behavior</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><code>add &lt;PATH&gt;</code></td>
      <td>Compresses and encrypts a source file, but then incorrectly deletes or rejects the vault entry as described above. Unsafe.</td>
    </tr>
    <tr>
      <td><code>rm &lt;NAME&gt;</code></td>
      <td>Asks for confirmation, overwrites the named vault entry with zero bytes, and removes it.</td>
    </tr>
    <tr>
      <td><code>mo &lt;NAME&gt;</code></td>
      <td>Decrypts and decompresses the named entry to its recorded path, then removes the vault entry.</td>
    </tr>
    <tr>
      <td><code>check</code></td>
      <td>Attempts to authenticate every stored entry with the supplied password.</td>
    </tr>
    <tr>
      <td><code>help</code></td>
      <td>Prints command help.</td>
    </tr>
  </tbody>
</table>

<p>Long forms are also parsed: <code>--add</code>, <code>--rm</code>, <code>--mo</code>, <code>--check</code>, and <code>--help</code>.</p>

<h2>Build for development</h2>

<pre><code>cargo build
cargo test</code></pre>

<p>The optimized profile enables fat LTO and strips symbols:</p>

<pre><code>cargo build --release</code></pre>

<h2>Storage layout</h2>

<p>On first use, the program creates:</p>

<table>
  <thead>
    <tr>
      <th align="left">Path</th>
      <th align="left">Purpose</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><code>~/.safe_dir/</code></td>
      <td>Default encrypted-entry directory.</td>
    </tr>
    <tr>
      <td><code>~/secure_safe.settings</code></td>
      <td>TOML settings file containing <code>enc_dir</code>.</td>
    </tr>
  </tbody>
</table>

<p>A different vault directory can be configured with:</p>

<pre><code>enc_dir = "/absolute/path/to/my-vault"</code></pre>

<h2>Known safety limitations</h2>

<ul>
  <li><code>add</code> currently has the critical deletion-target bug described at the top of this page.</li>
  <li>There is no password recovery.</li>
  <li>Overwriting a file before unlinking does not guarantee physical erasure on SSDs, copy-on-write filesystems, snapshots, journals, or remote storage.</li>
  <li>Several I/O and parsing paths still use <code>unwrap</code>, <code>expect</code>, or <code>panic</code>; interruption can leave partial state.</li>
  <li>Vault entries are keyed only by the source base filename, so equal filenames from different directories conflict.</li>
  <li>The format is pre-alpha and may change without migration support.</li>
</ul>

<h2>Before this can be trusted</h2>

<ol>
  <li>Fix the <code>add</code> deletion target and add regression tests proving the source and vault states.</li>
  <li>Make writes atomic and define recovery behavior for interruption or partial failure.</li>
  <li>Replace panic-based error handling on all file and cryptographic paths.</li>
  <li>Document and version the on-disk format.</li>
  <li>Obtain focused security review before claiming safe storage.</li>
</ol>

<div align="center">
  <sub>Pre-alpha cryptography code: inspect first, trust later.</sub>
</div>
