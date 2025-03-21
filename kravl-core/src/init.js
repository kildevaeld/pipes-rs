((self) => {
	async function* runTask(ctx, specifier) {
		const module = await import(specifier);

		const ret = await module.default(ctx);

		if (ret[Symbol.asyncIterator]) {
			for await (const i of ret) {
				yield processValue(i);
			}
		} else if (ret[Symbol.iterator]) {
			for (const i of ret) {
				yield processValue(i);
			}
		} else {
			yield processValue(ret);
		}
	}

	async function processValue(value) {
		if (!value) {
			return value;
		}
		if (value instanceof Package) {
			return value;
		}

		if (typeof value === "string") {
			return new Package("output.txt", value, "text/plain");
			// biome-ignore lint/style/noUselessElse: <explanation>
		} else if (ArrayBuffer.isView(value)) {
			return new Package("output.bin", value, "application/octet-stream");
			// biome-ignore lint/style/noUselessElse: <explanation>
		} else {
			return new Package("output.json", value, "application/json");
		}
	}

	class Package {
		constructor(name, content, mime) {
			this.name = name;
			this.content = content;
			this.mime = mime;
		}
	}

	if (!self.runTask) {
		self.runTask = runTask;
		self.Package = Package;
	}
})(globalThis);
