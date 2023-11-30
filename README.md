# techtalk


1. Go to the workspace that contains the game.
2. Run `cargo init` in the workspace.
   - At this point, your structure should look like this:

   ```markdown
   Workspace
   L game folder
   L src folder
   L Cargo.toml

3. Paste in the new `Cargo.toml` file of the workspace:
```toml
[workspace]
members = ["run-wasm", "<game_name>"]
```

4. Create a new folder called run-wasm by typing the following command in the terminal:
`cargo new --bin run-wasm`

    ```markdown
   Workspace
   L game folder
   L src folder
   L run-wasm folder
      L src folder
         L main.rs
      L Cargo.toml
   L Cargo.toml

5. Paste this in the Cargo.toml inside the run-wasm folder
   ```
   [package]
   name = "run-wasm"
   version = "0.1.0"
   edition = "2021"
   
   
   [dependencies]
   cargo-run-wasm = "0.3.0"
   ```

6. Paste this in the main.rs file of the run-wasm folder
   ```
   fn main() {
      cargo_run_wasm::run_wasm_with_css("body { margin: 0px; }");
   }
   ```

7. Inside workspace create a `.cargo/config` and paste this inside:
   ```
   [alias]
   run-wasm = "run --release --package run-wasm --"
   ```

   so your workspace looks like
      ```markdown
      Workspace
      ├── .cargo
         ├── config
      ├── game folder
      ├── engine folder
      ├── src folder
         ├── main.rs
      ├── Cargo.lock
      └── Cargo.toml

8. In the workspace terminal run `cargo run-wasm –bin <game folder name>`. This will create a run-wasm folder.

9. In the Cargo.toml inside your game folder
   Add to the dependencies:
   ```
   getrandom = { version = "0.2", features = ["js"] }
   ```

   And add below:
   ```
   [target.'cfg(target_arch = "wasm32")'.dependencies]
   js-sys = "0.3.64"
   console_error_panic_hook = "0.1.7"
   console_log = "1"
   wasm-bindgen-futures = "0.4.34"
   web-sys = { version = "0.3.64", features = ["Location", "Blob", "RequestInit", "RequestMode", "Request", "Response", "WebGl2RenderingContext", "CanvasRenderingContext2d"] }
   ```
10. Run `RUSTFLAGS=--cfg=web_sys_unstable_apis cargo run-wasm --package <game_name>`. Now we need to add our png and content for access. The cheat solution is just to put the content folder inside the target/wasm-examples/<your_game_name> folder
      ```markdown
      Workspace
      L game folder
      L engine folder
      L src folder
      L run-wasm folder
      L target folder
         L wasm-examples folder
            L content folder
      L .cargo folder
      L Cargo.lock
      L Cargo.toml

      You should get something like *http://localhost:8000* which you can paste into your browser. This will be blank because our png files are local.

**If you wish to implement asset manager, go to step 12 and then come back.**

11. Run this command in the workspace:
`RUSTFLAGS=--cfg=web_sys_unstable_apis cargo run-wasm --package <game_name>`


**To implement Asset Manager**  \newline
12. Add the following in the Cargo.toml file of your game
      - Under [dependencies] ->  `assets_manager = { version="0.10", features=["png","gltf","embedded"] }`
      - Under [features] ->`webgl = ["wgpu/webgl", "vbuf"]`

13. Add the following imports in the main.rs file of your game folder
      ```
      use image::DynamicImage;
      use assets_manager::source::{Embedded, Filesystem};
      ```
14. In the main.rs of your game folder
      - At the top of the file
         ```
         #[cfg(target_arch = "wasm32")]
         // type AssetCacheType = AssetCache<Embedded>;
         type AssetCacheType<'a> = AssetCache<Embedded<'a>>;
         
         #[cfg(not(target_arch = "wasm32"))]
         type AssetCacheType = AssetCache<FileSystem>;
         ```
      - In the main() function under the instantiation of event_loop and window.
           We can see a block that looks like:
           ```
           #[cfg(not(target_arch = "wasm32"))]
             {
                 env_logger::init();
                 pollster::block_on(run(event_loop, window));
             }
           ```
           We want to paste these:
           ```
           let source = assets_manager::source::FileSystem::new("./content").unwrap();
           let cache = AssetCache::with_source(source);
           ```
           So it looks like
           ```
          #[cfg(not(target_arch = "wasm32"))]
             {
                 let source = assets_manager::source::FileSystem::new("./content").unwrap();
                 let cache = AssetCache::with_source(source);
                 env_logger::init();
                 pollster::block_on(run(event_loop, window, cache));
             }
          ```
      - Under *#[cfg(target_arch = "wasm32")]*
          ```
          let source = assets_manager::source::Embedded::from(source::embed!("./content"));
          let cache = AssetCache::with_source(source);
          ```
          to look like
           ```
           #[cfg(target_arch = "wasm32")]
             {
                 let source = assets_manager::source::Embedded::from(source::embed!("./content"));
                 let cache = AssetCache::with_source(source);    
                 std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                 console_log::init_with_level(log::Level::Trace).expect("could not initialize logger");
                 use winit::platform::web::WindowExtWebSys;
                 // On wasm, append the canvas to the document body
                 web_sys::window()
                     .and_then(|win| win.document())
                     .and_then(|doc| doc.body())
                     .and_then(|body| {
                         body.append_child(&web_sys::Element::from(window.canvas()))
                             .ok()
                     })
                     .expect("couldn't append canvas to document body");
                 wasm_bindgen_futures::spawn_local(run(event_loop, window));
             }
           ```
      - Alter run function to take `cache: AssetCacheType<'_>` as a parameter
      - Add `cache` as a parameter to calls to run
      - Alter parameters of load_texture to take `image: &DynamicImage` instead of `path: impl AsRef<std::path::Path>`
      - Delete all code for loading img in load_texture and replace with `let image = image.to_rgba8();`
      - Replace calls to load texture with the following template
          ```
          let sprite = cache
          .load::<assets_manager::asset::Png>("nameofspritepng")
          .unwrap();
          let (sprite_tex, _sprite_img) = load_texture(&sprite.read().0, None, &device, &queue).unwrap();
           ```
15. Return to step 11








Resources:
- [Cargo run wasm](https://github.com/rukai/cargo-run-wasm)
- [Implement a WebAssembly WebGL viewer using Rust](https://blog.logrocket.com/implement-webassembly-webgl-viewer-using-rust/)





