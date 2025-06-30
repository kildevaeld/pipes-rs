export default function *() {

  let i = 0;
  while (i++  < 10) {
    console.log('Hello',i);
    yield i
  }
}
