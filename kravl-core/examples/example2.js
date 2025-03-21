function delay(n) {
  return new Promise((res) => setTimeout(res, n));
}

export default async function* test() {
  for (let i = 10; i < 20; i++) {
    await delay(100 * i);
    yield {
      name: "outout.json",
      content: {
        iter: i,
      },
      mime: "application/json",
    };
  }
}
