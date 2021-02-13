import logo from './logo.svg';
import './App.css';

function App({ wasm }) {

  let thing;
  let the_list;
  useEffect(() => {
    thing = wasm.create_thing();

    setInterval(() => {
      thing.do_something();
    }, 1000);

    the_list = thing.get_the_list();
  }, [input]);

  // Bind this observable to the react component
  useObserve(the_list);

  return (
    <div className="App">
      <header className="App-header">
        The List:<br />
        <ul>
          {the_list.map(v => (
            <li>{v}</li>
          ))}
        </ul>
      </header>
    </div>
  );
}

export default App;
