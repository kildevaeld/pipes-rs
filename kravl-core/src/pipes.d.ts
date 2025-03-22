type Result<T> = T | Promise<T>;
type PipeStream<T> = AsyncIterable<T> | Iterable<T>;
declare function enumerate<T>(
  stream: PipeStream<T>
): AsyncGenerator<[item: T, idx: number]>;
declare function map<T, R>(
  stream: PipeStream<T>,
  func: (item: T, idx: number) => Result<R>
): AsyncGenerator<Awaited<R>, void, unknown>;
declare function filter<T>(
  stream: PipeStream<T>,
  func: (item: T, idx: number) => Result<boolean>
): AsyncGenerator<T>;
declare function take<T>(
  stream: PipeStream<T>,
  count: number
): AsyncGenerator<Awaited<T>, void, unknown>;
declare function skip<T>(
  stream: PipeStream<T>,
  count: number
): AsyncGenerator<Awaited<T>, void, unknown>;
declare function peek<T>(
  stream: PipeStream<T>,
  peekFn: (item: T, idx: number) => unknown
): AsyncGenerator<Awaited<T>, void, unknown>;
type Flatten<T> = T extends AsyncIterable<infer InnerArr>
  ? InnerArr
  : T extends Iterable<infer InnerArr>
  ? InnerArr
  : T;
declare function flatten<T>(
  stream: PipeStream<AsyncIterable<T> | Iterable<T> | T>
): AsyncGenerator<T, void, unknown>;
// Collectors
declare function forEach<T>(
  stream: PipeStream<T>,
  func: (item: T, idx: number) => Result<void>
): Promise<void>;
declare function collect<T>(
  stream: PipeStream<T>,
  count?: number
): Promise<T[]>;
declare function fold<T, R>(
  stream: PipeStream<T>,
  acc: (prev: R, cur: T, idx: number) => Result<R>,
  init: R
): Promise<R>;
declare function find<T>(
  stream: PipeStream<T>,
  find: (item: T, idx: number) => Result<boolean>
): Promise<[item: T, idx: number] | undefined>;
declare function join<T>(
  stream: PipeStream<T>,
  joiner: string
): Promise<string>;
// Utilities
declare function next<T>(stream: AsyncIterator<T> | Iterator<T>): Promise<T>;
type IterItem<T> = T | PromiseLike<T>;
declare function fromIterable<T>(
  iterable: Iterable<IterItem<T>>
): AsyncGenerator<Awaited<T>, void, unknown>;
declare function fromPromise<T>(
  promise: PromiseLike<T>
): AsyncGenerator<Awaited<T>, void, unknown>;
type Combine<T> = (Iterable<T> | AsyncIterable<T>)[];
declare function combine<T>(
  combine: Combine<T> | Promise<Combine<T>>
): AsyncGenerator<any, void, unknown>;
declare function chain<T>(
  stream1: AsyncIterable<T>,
  stream2: AsyncIterable<T>
): AsyncGenerator<Awaited<T>, void, unknown>;
declare class Pipe<T> implements AsyncIterable<T> {
  #private;
  constructor(
    stream: AsyncIterableIterator<T>,
    errOnMove?: boolean,
    moveOnChain?: boolean
  );
  set errOnMove(on: boolean);
  get errOnMove(): boolean;
  set moveOnChain(on: boolean);
  get moveOnChain(): boolean;
  filter(func: (item: T, idx: number) => Result<boolean>): Pipe<T>;
  map<R>(func: (item: T, id: number) => Result<R>): Pipe<R>;
  take(count: number): Pipe<T>;
  skip(count: number): Pipe<T>;
  peek(peekFn: (item: T) => unknown): Pipe<T>;
  chain(nextStream: AsyncIterable<T>): Pipe<T>;
  combine(streams: Combine<T>): Pipe<T>;
  flat(): Pipe<Flatten<T>>;
  first(): Promise<T | undefined>;
  // Collect
  forEach(func: (item: T, idx: number) => Result<void>): Promise<void>;
  collect(count?: number): Promise<T[]>;
  fold<R>(acc: (prev: R, cur: T) => Result<R>, init: R): Promise<R>;
  find(
    func: (item: T) => Result<boolean>
  ): Promise<[item: T, idx: number] | undefined>;
  join(joiner: string): Promise<string>;
  [Symbol.asyncIterator](): AsyncIterator<T>;
}
type PipeInput<T> =
  | AsyncIterable<T>
  | Iterable<T>
  | Promise<AsyncIterable<T> | Iterable<T>>
  | T;
declare function pipe<T>(...input: PipeInput<T>[]): Pipe<T>;
export {
  type Result,
  type PipeStream,
  enumerate,
  map,
  filter,
  take,
  skip,
  peek,
  type Flatten,
  flatten,
  forEach,
  collect,
  fold,
  find,
  join,
  next,
  fromIterable,
  fromPromise,
  type Combine,
  combine,
  chain,
  Pipe,
  type PipeInput,
  pipe,
};
