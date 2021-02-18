

# Example setup:
Follow setup instructions here to create a react app with wasm support:  
https://dev.to/lokesh007/webassembly-with-rust-and-react-using-create-react-app-67  
NOTES:  
* Suggest you use the typescript flag when creating your react app:
  `npx create-react-app your_react_app --template typescript`
* Change .config-overrides.json `extraArgs: "--no-typescript"` to `extraArgs: ""`
* your wasm build output dir (specified in config-overrides.json) must be inside or symlinked within your react app src directory in order to be bundled


```bash
# Your rust crate for application logic, to be compiled to WASM
cd your_wasm_crate_dir

# Add this crate
cargo add observable-react
cd your_react_webapp_dir
npm serve
```