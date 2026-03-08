let count = 0;

const button = document.getElementById('counter')!;
button.addEventListener('click', () => {
  count++;
  button.textContent = `Count: ${count}`;
});

console.log('Volt enterprise starter is running.');
