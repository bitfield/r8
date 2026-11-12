# Licences and distribution

John Arundel's existing R8 source retains its original notices and licence grants. The root `LICENSE` supplies his MIT notice. Existing crate metadata also offers Apache-2.0. This work does not remove those grants or change their copyright holder.

New JIT compiler, integration and test files identify `AGPL-3.0-or-later` in their SPDX notices. [Zayn Otley](https://ZaynOtley.com) retains copyright in his original additions. See also [IntuitionEngine](https://IntuitionEngine.io). JIT integration additions to existing RX82 files also use `AGPL-3.0-or-later`; existing portions retain their original grants. The modified RX82 package metadata therefore records `(MIT OR Apache-2.0) AND AGPL-3.0-or-later`. The other crates retain their metadata.

No IE implementation source has been copied. Later authorised adaptations must identify their source and preserve applicable notices.

`tools/package_jit.py` creates a release bundle containing RX82 with its linked JIT compiler, AGPL text, the original MIT notice, Go and Rust toolchain notices, dependency notices, and a source archive. That archive contains the build tools, Cargo lockfile, vendored Rust dependencies and Cargo source configuration. It uses the current workspace contents, so its hash identifies the source more precisely than the recorded base Git revision alone. Keep these files together when distributing the release bundle.

Third-party dependencies retain their own licences. In the release bundle, `package.json` lists versions and licence expressions for vendored Cargo dependencies; their source and notices are included. This document does not replace any licence text.

The RX82 Cargo package includes `LICENSE`, `LICENSING.md` and `LICENSES/AGPL-3.0-or-later.txt` inside the crate directory. The Cargo packaging regression checks that these copies match the repository notices. Update both copies together.
