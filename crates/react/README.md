

# Example setup:
```bash
# Your rust WASM code
cargo new --lib your_wasm_crate
cd your_wasm_crate
cargo add observable-react
# Create your rust application logic
# see contents of example/your_wasm_crate
cd ..
mkdir wasm_build_output;

# Now create your react app
npx create-react-app app
cd app
 # Add rust build step to react app
npm install react-app-rewired wasm-loader -D
# copy config-overrides.js from example directory and modify based on your wasm path
npm serve
```