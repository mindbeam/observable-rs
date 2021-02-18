import React, { useMemo, useReducer, useEffect } from "react";
import { ReactObservable } from "./your-app-specific-crate-build";
import logo from './logo.svg';
import './App.css';

function App({ wasm }: { wasm: any }) {

  let [thing, the_list] = useMemo(() => {
    let thing = wasm.create_rust_thing();

    setInterval(() => {
      thing.do_something();
    }, 1000);

    return [thing, thing.get_the_list()];
  }, [wasm]);

  // Bind this observable to the react component
  the_list.useObserve();

  return (
    <div className="App">
      <header className="App-header">
        The List:<br />
        <ul>
          {the_list.map((v: any) => (
            <li>{v}</li>
          ))}
        </ul>
      </header>
    </div>
  );
}

export default App;

// function useObserve(observable: ReactObservable) {
//   // This is dumb. Increasinly not a fan of React hooks
//   const [_ignored, forceUpdate] = useReducer((x) => x + 1, 0);
//   let unsub: Function = () => { };

//   useEffect(() => {
//     unsub = observable.subscribe((v: any) => {
//       forceUpdate();
//     });
//     return () => {
//       unsub();
//     };
//   }, []);
// }
