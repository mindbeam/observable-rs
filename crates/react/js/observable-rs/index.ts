import React, { useReducer, useEffect } from "react";

export function useObserve(observable: any) {
  const [_, forceUpdate] = useReducer((x: number) => x + 1, 0);
  useEffect(() => {
    let unsub = observable.subscribe((v: any) => {
      forceUpdate();
    });
    return () => {
      unsub();
    };
  }, []);
}
