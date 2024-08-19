# Example of using `wgpu` with `objc2`

An example of rendering with `wgpu` directly to a view controlled by `objc2`.

This uses AppKit when targetting macOS, and UIKit otherwise.

To run this with Mac Catalyst, you will need to bundle your application. This can be done with `cargo bundle` as follows:
```sh
cargo +nightly bundle --format=ios --target=aarch64-apple-ios-macabi
./target/aarch64-apple-ios-macabi/debug/bundle/ios/wgpu-objc2-example.app/wgpu-objc2-example
```

## Configurations

See [`Cargo.toml`](./Cargo.toml) for the list of features that change the mode of execution.
