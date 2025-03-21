((self) => {
	async function runTask(ctx, specifier) {
		const module = await import(specifier);

		const ret = await module.default({
			async push(task) {
				if (typeof task === "function") {
					ctx.push((ctx) => handle(ctx, task));
				} else {
					ctx.push(task);
				}
			},
		});

		if (ret[Symbol.asyncIterator]) {
			for await (const i of ret) {
			}
		} else if (ret[Symbol.iterator]) {
			for (const i of ret) {
			}
		}
	}

	async function handle(ctx, task) {}

	if (this.runTask) {
		this.runTask = runTask;
	}
})(globalThis);
