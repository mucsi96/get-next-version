import { execFileSync } from "child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "fs";
import { tmpdir } from "os";
import { join, resolve } from "path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

const action = resolve("dist/index.js");
const identity = {
  ...process.env,
  GIT_AUTHOR_NAME: "Version action test",
  GIT_AUTHOR_EMAIL: "version-test@example.invalid",
  GIT_COMMITTER_NAME: "Version action test",
  GIT_COMMITTER_EMAIL: "version-test@example.invalid",
};

let root: string;
let checkout: string;

function git(...args: string[]): string {
  return execFileSync("git", args, {
    cwd: checkout,
    env: identity,
    encoding: "utf-8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function commit(message: string): void {
  git("add", ".");
  git("commit", "-m", message);
}

function runAction(prefix: string): Record<string, string> {
  const output = join(root, `${prefix}-output`);
  writeFileSync(output, "");
  execFileSync(process.execPath, [action], {
    cwd: checkout,
    env: {
      ...identity,
      INPUT_PREFIX: prefix,
      INPUT_SRC: prefix,
      INPUT_IGNORE: "",
      GITHUB_OUTPUT: output,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  const values = readFileSync(output, "utf-8");
  return Object.fromEntries(
    [...values.matchAll(/^(\w+)<<([^\n]+)\n([\s\S]*?)\n\2(?:\n|$)/gm)]
      .map((match) => [match[1], match[3]])
  );
}

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "get-next-version-"));
  checkout = join(root, "checkout");
  mkdirSync(checkout);
  git("init", "--bare", join(root, "origin.git"));
  git("init", "-b", "main");
  git("remote", "add", "origin", join(root, "origin.git"));
  mkdirSync(join(checkout, "server"));
  mkdirSync(join(checkout, "client"));
  writeFileSync(join(checkout, "server", "source.txt"), "server release\n");
  writeFileSync(join(checkout, "client", "source.txt"), "client release\n");
  commit("Initial source");
});

afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

describe("bundled action outputs", () => {
  it("starts at version 1 when there are no release tags", () => {
    expect(runAction("server")).toEqual({
      version: "1", tag: "server-1", hasNextVersion: "true",
    });
  });

  it("returns current versions for unchanged components without publishing", () => {
    git("tag", "server-42");
    git("tag", "client-17");
    writeFileSync(join(checkout, "README.md"), "Workflow-only maintenance\n");
    commit("Documentation change");

    expect(runAction("server")).toEqual({
      version: "42", tag: "server-42", hasNextVersion: "false",
    });
    expect(runAction("client")).toEqual({
      version: "17", tag: "client-17", hasNextVersion: "false",
    });
  });

  it("resolves a new server version while retaining the unchanged client", () => {
    git("tag", "server-42");
    git("tag", "client-17");
    writeFileSync(join(checkout, "server", "source.txt"), "updated server\n");
    commit("Server change");

    expect(runAction("server")).toEqual({
      version: "43", tag: "server-43", hasNextVersion: "true",
    });
    expect(runAction("client")).toEqual({
      version: "17", tag: "client-17", hasNextVersion: "false",
    });
  });

  it("uses reachable releases when unchanged and global numbering for new releases", () => {
    git("tag", "server-42");
    git("switch", "-c", "unrelated-release");
    writeFileSync(join(checkout, "server", "source.txt"), "unrelated release\n");
    commit("Unrelated release");
    git("tag", "server-99");
    git("switch", "main");

    expect(runAction("server")).toEqual({
      version: "42", tag: "server-42", hasNextVersion: "false",
    });

    writeFileSync(join(checkout, "server", "source.txt"), "main server change\n");
    commit("Main server change");
    expect(runAction("server")).toEqual({
      version: "100", tag: "server-100", hasNextVersion: "true",
    });
  });
});
