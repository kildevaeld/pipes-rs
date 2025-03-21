export default async function () {
	const resp = await fetch("https://loppen.dk", {
		headers: {
			"User-Agent":
				"Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0",
		},
	});

	return new Package("loppen.html", await resp.text(), "text/html");
}
