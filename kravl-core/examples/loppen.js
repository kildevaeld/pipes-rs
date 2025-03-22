import { parse } from "@klaver/dom";

export const meta = {
  name: "Loppen",
};

export default async function () {
  const dom = await fetchDom("https://loppen.dk");

  const title = dom.querySelector("head title").innerText;
  console.log("Title");

  return [new Package("loppen-title.txt", title, "text/plain")];
}
