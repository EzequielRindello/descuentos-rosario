// Starts the daily.yml workflow on main through GitHub's workflow_dispatch API.
// Runs on the cron in ../wrangler.jsonc. Each day is independent: a failed dispatch is logged
// and the maintainer runs it by hand; the next day's trigger fires anyway.

const DISPATCH_URL =
  "https://api.github.com/repos/EzequielRindello/descuentos-rosario/actions/workflows/daily.yml/dispatches";

export default {
  async scheduled(_controller, env) {
    const response = await fetch(DISPATCH_URL, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${env.GITHUB_TOKEN}`,
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "ofertas-rosario-cron (+https://github.com/EzequielRindello/descuentos-rosario)",
      },
      body: JSON.stringify({ ref: "main" }),
    });

    // GitHub answers 204 on success. Anything else (401: expired or revoked token) marks this
    // run as failed in the Worker's logs.
    if (!response.ok) {
      throw new Error(`GitHub rejected the dispatch: ${response.status} ${await response.text()}`);
    }
  },
};
