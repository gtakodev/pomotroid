# Contributing to Pomotroid

Thank you so much for contributing to Pomotroid. We appreciate your time and help.
Here are some guidelines to help you get started.

## Code of Conduct

Be kind and respectful to the members of the community. Take time to educate others who are seeking help. Harassment of any kind will not be tolerated.

## Questions

If you have questions regarding Pomotroid, feel free to open an issue with the `Question` tag, or reach out directly via email at `flyweight@pm.me`.

## Filing a Bug or Feature

1. Before filing an issue, please check the existing issues to see if a similar one was already opened. If there is one already opened, feel free to comment on it.
2. If you believe you've found a bug, please provide detailed steps of reproduction, the version of Pomotroid and anything else you believe will be useful to help troubleshoot it (e.g. OS environment, environment variables, etc...). Also state the current behavior vs. the expected behavior.
3. If you'd like to see a feature or an enhancement please open an issue with a clear title and description of what the feature is and why it would be beneficial to the project and its users.

## Build and run Pomotroid

Install [rustup](https://rustup.rs/). Cargo uses the toolchain pinned in `rust-toolchain.toml`.

- On Windows, install the MSVC C++ build tools and Windows SDK.
- On macOS, install the Xcode command-line tools with `xcode-select --install`.
- On Debian or Ubuntu, install Rune's Wayland and keyboard libraries and the audio library headers. Install `libxkbcommon-x11-0` for windowed runs and `libnotify-bin` for desktop notifications.

  ```sh
  sudo apt-get install pkg-config libasound2-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-0 libnotify-bin
  ```

From the repository root, run these commands to start Pomotroid, run the tests, and compile a release binary.

```sh
cargo run --locked
cargo test --locked
cargo build --locked --release
```

To run the [timer model checker](verification/README.md) without compiling the desktop app, use its standalone manifest.

```sh
cargo test --locked --offline --manifest-path verification/Cargo.toml -- --nocapture
```

On macOS, build and open the app bundle to test desktop notifications. The bundle script signs the app ad hoc for local testing.

```sh
cargo build --locked --release
bash scripts/bundle-macos.sh
open target/release/Pomotroid.app
```

## Submit a change

1. Fork the project and clone your fork.
2. Create a branch for your change.
3. Run the app and tests before you commit.
4. Push your branch and open a pull request.
