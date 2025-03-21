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
