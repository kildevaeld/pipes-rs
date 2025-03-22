function delay(n) {
  return new Promise((res) => setTimeout(res, n));
}

export default async function* test() {
  for (let i = 0; i < 5; i++) {
    console.log("waiting");
    await delay(600);

    yield new Package("output.json", { count: i }, "application/json");
    console.log("dones");
  }
}
