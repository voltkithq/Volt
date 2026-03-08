import { useState } from 'react';

export function App() {
  const [count, setCount] = useState(0);

  return (
    <div className="app">
      <h1>Welcome to Volt!</h1>
      <p>Edit <code>src/App.tsx</code> and save to see changes.</p>
      <button onClick={() => setCount(c => c + 1)}>
        Count: {count}
      </button>
    </div>
  );
}
