import * as core from "@actions/core";
import { spawnSync } from "child_process";
import { getVersion, setVersion } from "./get-next-version";

// Ensure we have the full history and all tags
const unshallow = spawnSync("git", ["fetch", "--tags", "--unshallow"], { stdio: "inherit" });
if (unshallow.status !== 0) {
  spawnSync("git", ["fetch", "--tags"], { stdio: "inherit" });
}

const prefix = core.getInput("prefix", { required: true });
const src = core.getInput("src") || ".";
const ignore = core.getInput("ignore")
  ? core.getInput("ignore").split(" ").filter(Boolean)
  : [];

const version = getVersion({ src, tagPrefix: prefix, ignore });

if (version === null) {
  core.setOutput("version", "");
  core.setOutput("hasNextVersion", "false");
} else {
  core.setOutput("version", version.toString());
  core.setOutput("hasNextVersion", "true");
}
