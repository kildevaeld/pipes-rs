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
      yield processValue(value);
    }
  }

  async function processValue(value) {
    return value;
  }

  if (this.runTask) {
    this.runTask = runTask;
  }
})(globalThis);
