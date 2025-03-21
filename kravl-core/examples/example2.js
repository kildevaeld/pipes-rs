function delay(n) {
	return new Promise((res) => setTimeout(res, n));
}

export default async function* test() {
	for (let i = 10; i < 15; i++) {
		await delay(100 * i);
		yield {
			iter: i,
		};
	}
}
