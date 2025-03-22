function isIterable(i) {
  return typeof i[Symbol.iterator] === "function";
}
function isAsyncIterable(i) {
  return typeof i[Symbol.asyncIterator] === "function";
}
function isPromise(obj) {
  return (
    !!obj &&
    (typeof obj === "object" || typeof obj === "function") &&
    typeof obj.then === "function"
  );
}

async function* enumerate(stream) {
  let idx = 0;
  for await (const next of stream) {
    yield [next, idx++];
  }
}
async function* map(stream, func) {
  for await (const next of enumerate(stream)) {
    yield await func(...next);
  }
}
async function* filter(stream, func) {
  for await (const next of enumerate(stream)) {
    if (await func(...next)) {
      yield next[0];
    }
  }
}
async function* take(stream, count) {
  for await (const [next, idx] of enumerate(stream)) {
    if (idx == count) {
      break;
    }
    yield next;
  }
}
async function* skip(stream, count) {
  for await (const [next, idx] of enumerate(stream)) {
    if (idx < count) {
      continue;
    }
    yield next;
  }
}
function peek(stream, peekFn) {
  return map(stream, async (item, idx) => {
    await peekFn(item, idx);
    return item;
  });
}
async function* flatten(stream) {
  for await (const item of stream) {
    if (isAsyncIterable(item)) {
      for await (const inner of item) {
        yield inner;
      }
    } else if (isIterable(item)) {
      for await (const inner of item) {
        yield inner;
      }
    } else {
      yield item;
    }
  }
}
// Collectors
async function forEach(stream, func) {
  for await (const next of enumerate(stream)) {
    await func(...next);
  }
}
async function collect(stream, count) {
  const out = [];
  const s = typeof count === "number" ? take(stream, count) : stream;
  for await (const item of s) {
    out.push(item);
  }
  return out;
}
async function fold(stream, acc, init) {
  await forEach(stream, async (next, idx) => {
    init = await acc(init, next, idx);
  });
  return init;
}
async function find(stream, find) {
  for await (const item of enumerate(stream)) {
    if (await find(...item)) {
      return item;
    }
  }
}
async function join(stream, joiner) {
  return fold(
    stream,
    (prev, cur, idx) => {
      if (idx > 0) prev += joiner;
      return prev + String(cur);
    },
    ""
  );
}
// Utilities
async function next(stream) {
  return (await stream.next())?.value;
}
async function* fromIterable(iterable) {
  for (const next of iterable) {
    yield await Promise.resolve(next);
  }
}
async function* fromPromise(promise) {
  yield await promise;
}
async function* combine(combine) {
  const input = await Promise.resolve(combine);
  const asyncIterators = Array.from(input, (o) => {
    if (isAsyncIterable(o)) {
      return o[Symbol.asyncIterator]();
    } else {
      return o[Symbol.iterator]();
    }
  });
  const results = [];
  let count = asyncIterators.length;
  const never = new Promise(() => {});
  async function getNext(asyncIterator, index) {
    const result = await asyncIterator.next();
    return {
      index,
      result,
    };
  }
  const nextPromises = asyncIterators.map(getNext);
  try {
    while (count) {
      const { index, result } = await Promise.race(nextPromises);
      if (result.done) {
        nextPromises[index] = never;
        results[index] = result.value;
        count--;
      } else {
        nextPromises[index] = getNext(asyncIterators[index], index);
        yield result.value;
      }
    }
  } finally {
    for (const [index, iterator] of asyncIterators.entries())
      if (nextPromises[index] != never && iterator.return != null)
        iterator.return();
    // no await here - see https://github.com/tc39/proposal-async-iteration/issues/126
  }
}
async function* chain(stream1, stream2) {
  for await (const next of stream1) {
    yield next;
  }
  for await (const next of stream2) {
    yield next;
  }
}

async function* noop() {}
async function* noopErr() {
  throw new Error("use after move");
}
class Pipe {
  #stream;
  #errOnMoved;
  #moveOnChain;
  constructor(stream, errOnMove = false, moveOnChain = true) {
    this.#stream = stream;
    this.#errOnMoved = errOnMove;
    this.#moveOnChain = moveOnChain;
  }
  set errOnMove(on) {
    this.#errOnMoved = on;
  }
  get errOnMove() {
    return this.#errOnMoved;
  }
  set moveOnChain(on) {
    this.#moveOnChain = on;
  }
  get moveOnChain() {
    return this.#moveOnChain;
  }
  filter(func) {
    return this.#chained(filter(this.#stream, func));
  }
  map(func) {
    return this.#chained(map(this.#stream, func));
  }
  take(count) {
    return this.#chained(take(this.#stream, count));
  }
  skip(count) {
    return this.#chained(skip(this.#stream, count));
  }
  peek(peekFn) {
    return this.#chained(peek(this.#stream, peekFn));
  }
  chain(nextStream) {
    return this.#chained(chain(this.#stream, nextStream));
  }
  combine(streams) {
    return this.#chained(combine([this.#stream, ...streams]));
  }
  flat() {
    return this.#chained(flatten(this.#stream));
  }
  async first() {
    const stream = this.take(1)[Symbol.asyncIterator]();
    return (await stream.next()).value;
  }
  // Collect
  async forEach(func) {
    return forEach(this.#move(), func);
  }
  async collect(count) {
    return collect(this.#move(), count);
  }
  async fold(acc, init) {
    return await fold(this.#move(), acc, init);
  }
  async find(func) {
    return await find(this.#move(), func);
  }
  async join(joiner) {
    return await join(this.#move(), joiner);
  }
  [Symbol.asyncIterator]() {
    return this.#stream[Symbol.asyncIterator]();
  }
  #chained(next) {
    if (this.#moveOnChain) {
      this.#stream = this.#noopFn();
      return new Pipe(next, this.#errOnMoved, this.#moveOnChain);
    } else {
      this.#stream = next;
      return this;
    }
  }
  #move() {
    const stream = this.#stream;
    if (this.#moveOnChain) {
      this.#stream = this.#noopFn();
    }
    return stream;
  }
  #noopFn() {
    return this.#errOnMoved ? noopErr() : noop();
  }
}
function pipe(...input) {
  const o = input.map((m) => {
    if (isPromise(m)) {
      return flatten(fromPromise(m));
    } else if (isIterable(m) || isAsyncIterable(m)) {
      return m;
    } else {
      return [m];
    }
  });
  return new Pipe(combine(o));
}

export {
  Pipe,
  chain,
  collect,
  combine,
  enumerate,
  filter,
  find,
  flatten,
  fold,
  forEach,
  fromIterable,
  fromPromise,
  join,
  map,
  next,
  peek,
  pipe,
  skip,
  take,
};
