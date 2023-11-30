# techtalk


1. Go to the workspace that contains the game.
2. Run `cargo init` in the workspace.
   - At this point, your structure should look like this:

   ```markdown
   Workspace
   ├── game folder
   ├── engine folder
   ├── content folder
   ├── src folder
   ├── main.rs
   ├── target folder
   ├── Cargo.lock
   └── Cargo.toml

3. In the new `Cargo.toml` file of the workspace:
```toml
[workspace]
members = ["run-wasm", "connect4"]

4. Create a new folder called run-wasm by typing the following command in the terminal:


