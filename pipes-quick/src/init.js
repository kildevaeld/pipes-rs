((self) => {
  async function* runTask(specifier) {
    const module = await import(specifier);

    const ret = await module.default();

    if (ret[Symbol.asyncIterator]) {
      for await (const i of ret) {
        if (i) yield processValue(i);
      }
    } else if (ret[Symbol.iterator] && !ArrayBuffer.isView(ret)) {
      for (const i of ret) {
        if (i) yield processValue(i);
      }
    } else {
      if (ret) yield processValue(ret);
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
    } else if (value instanceof Json) {
      return new Package("output.json", value.value, "application/json");
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

  class Json {
    constructor(value) {
      this.value = value;
    }
  }

  if (!self.__runTask) {
    self.__runTask = runTask;
    self.Package = Package;
    self.Json = Json;

    // self.fetchDom = async (url, requestInit) => {
    //   const { parse } = await import("@klaver/dom");
    //   const response = await fetch(
    //     url,
    //     requestInit ?? {
    //       headers: {
    //         "User-Agent":
    //           "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0",
    //         "Cache-Control": "no-cache",
    //       },
    //     }
    //   );

    //   const text = await response.text();
    //   return parse(text);
    // };

    // self.readFile = async (path, encoding) => {
    //   const fs = await import("@klaver/fs");
    //   const buffer = await fs.read(path);
    //   return encoding ? new TextDecoder(encoding).decode(buffer) : buffer;
    // };

    self.json = (value) => new Json(value);
  }
})(globalThis);
