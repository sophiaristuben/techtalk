# techtalk


1. Go to the workspace that contains the game.
2. Run `cargo init` in the workspace.
   - At this point, your structure should look like this:

   ```markdown
   Workspace
   ├── game folder
   ├── engine folder
   ├── src folder
      ├── main.rs
   ├── Cargo.lock
   └── Cargo.toml

3. In the new `Cargo.toml` file of the workspace:
```toml
[workspace]
members = ["run-wasm", "connect4"]
```

4. Create a new folder called run-wasm by typing the following command in the terminal:
`cargo new --bin run-wasm`
Workspace
L game folder
L engine folder
L content folder
L src folder
L target folder
L run-wasm folder
   L src folder
      L main.rs
   L Cargo.toml
L Cargo.lock

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

8. In the workspace terminal run ``cargo run-wasm –bin <game folder name>`

9. Now we need to add our png and content for access. The cheat solution is just to put the content folder inside the target/wasm-examples/<your_game_name> folder
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

10. In the Cargo.toml inside your game folder
   Add to the dependencies:
   ```
   getrandom = { version = "0.2", features = ["js"] }
   ```

   And add below:
   ```
   [dependencies]
   getrandom = { version = "0.2", features = ["js"] }
   
   [target.'cfg(target_arch = "wasm32")'.dependencies]
   js-sys = "0.3.64"
   console_error_panic_hook = "0.1.7"
   console_log = "1"
   wasm-bindgen-futures = "0.4.34"
   web-sys = { version = "0.3.64", features = ["Location", "Blob", "RequestInit", "RequestMode", "Request", "Response", "WebGl2RenderingContext", "CanvasRenderingContext2d"] }
   ```

11. Run this command in the workspace:
`RUSTFLAGS=--cfg=web_sys_unstable_apis cargo run-wasm --package connect4 --features webgl`








