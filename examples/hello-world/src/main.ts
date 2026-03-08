console.log('Volt is running!');

const heading = document.querySelector('h1');
if (heading) {
  heading.addEventListener('click', () => {
    heading.textContent = 'Clicked!';
    setTimeout(() => {
      heading.textContent = 'Hello from Volt!';
    }, 1000);
  });
}
