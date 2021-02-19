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