type Mime = string;

type PrimitiveValue = string | boolean | number;

type ObjectValue = Record<string, Value>;

type ListValue = Value[];

type Value = PrimitiveValue | ObjectValue | ListValue;

declare class Package {
  name: string;
  mime?: Mime;
  content: PackageContent;
  constructor(name: string, content: PackageContent, mime?: Mime);
}

declare class Json {
  value: Value;
  constructor(value: Value);
}

type PackageContent = string | ArrayBuffer | Json;

interface Meta {
  name?: string;
}

// function fetchDom(
//   url: string,
//   req?: RequestInit
// ): Promise<import("@klaver/dom").Document>;

// function readFile(path: string): Promise<ArrayBuffer>;
// function readFile(path: string, encoding: string): Promise<string>;
declare function json(value: Value): Json;

type HandlerResult =
  | Package
  | PackageContent
  | Iterable<Package | PackageContent>
  | AsyncIterable<Package | PackageContent>
  | undefined;

type Handler = () => HandlerResult | Promise<HandlerResult>;
