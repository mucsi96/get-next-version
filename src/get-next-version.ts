import { spawnSync } from "child_process";

export function getPreviousTag(tagPrefix: string): string | null {
  const result = spawnSync(
    "git",
    ["describe", "--tags", `--match=${tagPrefix}-[1-9]*`, "--abbrev=0"],
    { encoding: "utf-8" }
  );

  if (result.stderr) {
    process.stderr.write(result.stderr);
  }

  if (result.status || !result.stdout) {
    return null;
  }

  return result.stdout.trim();
}

export function hasSourceCodeChanged(
  src: string,
  prevTag: string,
  ignore: string[]
): boolean {
  const ignoreStr = ignore.map((x) => `':!${x}'`).join(" ");

  console.log(`Detecting changes in ${src} since ${prevTag}`);

  const args = [
    "git",
    "diff",
    "--name-only",
    "HEAD",
    prevTag,
    "--",
    ".",
    ignoreStr,
  ].filter(Boolean);

  const result = spawnSync(args[0], args.slice(1), {
    cwd: src,
    encoding: "utf-8",
  });

  if (result.stderr) {
    process.stderr.write(result.stderr);
  }

  if (result.stdout) {
    console.log(result.stdout);
  }

  return Boolean(result.stdout);
}

export function getLatestVersion(tagPrefix: string): number | null {
  const result = spawnSync(
    "git",
    ["tag", "--list", "--sort=-v:refname", `${tagPrefix}-[1-9]*`],
    { encoding: "utf-8" }
  );

  if (result.stderr) {
    process.stderr.write(result.stderr);
  }

  if (result.status || !result.stdout) {
    return null;
  }

  const tags = result.stdout.split("\n").filter(Boolean);
  const latestTag = tags[0];

  return parseInt(latestTag.replace(new RegExp(`^${tagPrefix}-`), ""), 10);
}

export function getVersion(options: {
  src: string;
  tagPrefix: string;
  ignore?: string[];
}): number | null {
  const { src, tagPrefix, ignore = [] } = options;

  const prevTag = getPreviousTag(tagPrefix);

  if (prevTag) {
    if (hasSourceCodeChanged(src, prevTag, ignore) === false) {
      const version = prevTag.replace(new RegExp(`^${tagPrefix}-`), "");
      console.log(
        `No changes detected since ${tagPrefix}:${version} in ${src}.`
      );
      return null;
    }
  }

  const latestVersion = getLatestVersion(tagPrefix);
  let newVersion: number;

  if (latestVersion) {
    newVersion = latestVersion + 1;
  } else {
    newVersion = 1;
  }

  console.log(`Changes detected for ${tagPrefix}. New version: ${newVersion}`);

  return newVersion;
}

export function setVersion(tagPrefix: string, version: number): void {
  console.log(`Tagging with ${tagPrefix}-${version}`);
  spawnSync("git", ["tag", `${tagPrefix}-${version}`], { stdio: "inherit" });
  spawnSync("git", ["push", "--tags"], { stdio: "inherit" });
}
