# bevy_ed2d (experiment)

Easily edit 2D Bevy scenes with one line of code.

## Motivation

I was tinkering with making 2D platforming physics crate, and I wanted to easily see what I was doing in the different examples with a minimal amount of code.

The goal of this package is not to be super customizable or feature-rich, but simply and quickly get something up and running that supports:

- [x] picking
- [x] inspector
- [x] moving and zooming the camera
- [ ] highlighting selected objects
- [ ] gizmo for moving selected objects

Hopefully, Bevy will get an official editor soon, and this package will be obsolete.

## Usage

```rust
use bevy::prelude::*;
use bevy_ed2d::Ed2dPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Ed2dPlugin::default()))
        .run();
}
```

## Bevy Version Support

The `main` branch targets the latest bevy release.

|bevy|bevy_ed2d|
|----|---------|
|0.14|main     |

## License

`bevy_ed2d` is dual-licensed under either

- MIT License (./LICENSE-MIT or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 (./LICENSE-APACHE or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.

## Credit

Heavily based on [bevy_editor_pls](https://github.com/jakobhellermann/bevy_editor_pls)
