function delay(n) {
  return new Promise((res) => setTimeout(res, n));
}

export default async function* test() {
  for (let i = 0; i < 10; i++) {
    await delay(600);
    yield {
      name: "outout.json",
      content: {
        count: i,
      },
      mime: "application/json",
    };
  }
}
