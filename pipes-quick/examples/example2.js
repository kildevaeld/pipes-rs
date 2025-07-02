
function sleep(ms) { 
  return new Promise(resolve => setTimeout(resolve, ms));
}

export default async function *() {

  let i = 0;
  while (i++  < 10) {
    await sleep(100);
    console.log('Moyn',i);
    yield i
  }
}
