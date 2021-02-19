# Observable-react
So you want to use wasm_bindgen to add rust to you existing React app, or you're not quite ready for the rust web frameworks? 

There are lots of examples out there demonstrating WASM for computationally intensive workloads, but how do you hook into React component rendering?

This crate 


# Example Usage
```javascript
import React, { useMemo, useReducer } from "react";
import { useObserve } from "observable-rs";

function App({ wasm }: { wasm: any }) {
  let [listVisible, toggleShow] = useReducer((show: boolean) => { return !show }, true);

  let [thing, the_list] = useMemo(() => {
    let thing = wasm.create_rust_thing();
    setInterval(() => thing.do_something(), 1000);
    return [thing, thing.get_the_list()];
  }, [wasm]);

  return (
    <div className="App">
      <button onClick={toggleShow}>{listVisible ? "Hide the list" : "Show the List"} </button><br />
      { listVisible ? <TheList the_list={the_list} /> : ''}
    </div>
  );
}

export default App;

function TheList({ the_list }: { the_list: any }) {
  // Bind this observable to the react component
  useObserve(the_list);

  return (
    <div>The List:<br />
      <ul>
        {the_list.map((v: any) => (
          <li key={v}>{v}</li>
        ))}
      </ul>
    </div>
  )
}
```

# Example setup:
First, follow the setup instructions here to create a react app with a bundled rust crate:  
https://dev.to/lokesh007/webassembly-with-rust-and-react-using-create-react-app-67  
NOTES:  
* Suggest you use the typescript flag when creating your react app:
  `npx create-react-app your_react_app --template typescript`
* Change .config-overrides.json `extraArgs: "--no-typescript"` to `extraArgs: ""`
* your wasm build output dir (specified in config-overrides.json) must be inside or symlinked within your react app src directory in order to be bundled

Then:

```bash
cd your_react_app

# This adds the useObserve() helper function to your react app
npm i observable-rs

# Your rust crate for application logic, to be compiled to WASM
cd your_wasm_crate_dir

# Add this crate
cargo add observable-react

cd your_react_app
npm serve
```