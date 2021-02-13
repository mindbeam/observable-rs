import React from 'react';
import ReactDOM from 'react-dom';
import './index.css';
import App from './App';
import reportWebVitals from './reportWebVitals';

// Delay rendering of the app until we have the wasm loaded.
import("./wasm_build_output").then((wasm) => {
  ReactDOM.render(
    <React.StrictMode>
      <App wasm={wasm} />
    </React.StrictMode>,
    document.getElementById('root')
  );
});



// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals();
