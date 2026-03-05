import { describe, it, expect, vi, beforeEach } from "vitest";
import { spawnSync, SpawnSyncReturns } from "child_process";
import {
  getPreviousTag,
  hasSourceCodeChanged,
  getLatestVersion,
  getVersion,
  setVersion,
} from "./get-next-version";

vi.mock("child_process", () => ({
  spawnSync: vi.fn(),
}));

const mockedSpawnSync = vi.mocked(spawnSync);

function mockSpawnResult(
  overrides: Partial<SpawnSyncReturns<string>> = {}
): SpawnSyncReturns<string> {
  return {
    pid: 1,
    output: [],
    stdout: "",
    stderr: "",
    status: 0,
    signal: null,
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.spyOn(console, "log").mockImplementation(() => {});
  vi.spyOn(process.stderr, "write").mockImplementation(() => true);
});

describe("getPreviousTag", () => {
  it("returns the tag when git describe succeeds", () => {
    mockedSpawnSync.mockReturnValue(
      mockSpawnResult({ stdout: "app-3\n", status: 0 })
    );

    expect(getPreviousTag("app")).toBe("app-3");
    expect(mockedSpawnSync).toHaveBeenCalledWith(
      "git",
      ["describe", "--tags", "--match=app-[1-9]*", "--abbrev=0"],
      { encoding: "utf-8" }
    );
  });

  it("returns null when git describe fails", () => {
    mockedSpawnSync.mockReturnValue(
      mockSpawnResult({ stdout: "", status: 128 })
    );

    expect(getPreviousTag("app")).toBeNull();
  });

  it("returns null when stdout is empty", () => {
    mockedSpawnSync.mockReturnValue(
      mockSpawnResult({ stdout: "", status: 0 })
    );

    expect(getPreviousTag("app")).toBeNull();
  });

  it("prints stderr when present", () => {
    mockedSpawnSync.mockReturnValue(
      mockSpawnResult({ stderr: "some warning\n", stdout: "app-1\n" })
    );

    getPreviousTag("app");

    expect(process.stderr.write).toHaveBeenCalledWith("some warning\n");
  });
});

describe("hasSourceCodeChanged", () => {
  it("returns true when diff output is non-empty", () => {
    mockedSpawnSync.mockReturnValue(
      mockSpawnResult({ stdout: "src/main.ts\n" })
    );

    expect(hasSourceCodeChanged(".", "app-1", [])).toBe(true);
    expect(mockedSpawnSync).toHaveBeenCalledWith(
      "git",
      ["diff", "--name-only", "HEAD", "app-1", "--", "."],
      { cwd: ".", encoding: "utf-8" }
    );
  });

  it("returns false when diff output is empty", () => {
    mockedSpawnSync.mockReturnValue(mockSpawnResult({ stdout: "" }));

    expect(hasSourceCodeChanged(".", "app-1", [])).toBe(false);
  });

  it("passes ignore paths as git pathspec", () => {
    mockedSpawnSync.mockReturnValue(mockSpawnResult({ stdout: "" }));

    hasSourceCodeChanged("src", "app-1", ["docs", "tests"]);

    expect(mockedSpawnSync).toHaveBeenCalledWith(
      "git",
      [
        "diff",
        "--name-only",
        "HEAD",
        "app-1",
        "--",
        ".",
        "':!docs' ':!tests'",
      ],
      { cwd: "src", encoding: "utf-8" }
    );
  });

  it("prints stderr and stdout when present", () => {
    mockedSpawnSync.mockReturnValue(
      mockSpawnResult({ stdout: "file.ts\n", stderr: "warn\n" })
    );

    hasSourceCodeChanged(".", "app-1", []);

    expect(process.stderr.write).toHaveBeenCalledWith("warn\n");
    expect(console.log).toHaveBeenCalledWith("file.ts\n");
  });
});

describe("getLatestVersion", () => {
  it("returns the version number from the latest tag", () => {
    mockedSpawnSync.mockReturnValue(
      mockSpawnResult({ stdout: "app-3\napp-2\napp-1\n" })
    );

    expect(getLatestVersion("app")).toBe(3);
    expect(mockedSpawnSync).toHaveBeenCalledWith(
      "git",
      ["tag", "--list", "--sort=-v:refname", "app-[1-9]*"],
      { encoding: "utf-8" }
    );
  });

  it("returns null when no tags exist", () => {
    mockedSpawnSync.mockReturnValue(
      mockSpawnResult({ stdout: "", status: 0 })
    );

    expect(getLatestVersion("app")).toBeNull();
  });

  it("returns null when git command fails", () => {
    mockedSpawnSync.mockReturnValue(
      mockSpawnResult({ stdout: "", status: 1 })
    );

    expect(getLatestVersion("app")).toBeNull();
  });
});

describe("getVersion", () => {
  it("returns 1 when no tags exist", () => {
    // getPreviousTag returns null
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "", status: 128 })
    );
    // getLatestVersion returns null
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "", status: 0 })
    );

    expect(getVersion({ src: ".", tagPrefix: "app" })).toBe(1);
    expect(console.log).toHaveBeenCalledWith(
      "Changes detected for app. New version: 1"
    );
  });

  it("returns null when tag exists and no changes detected", () => {
    // getPreviousTag
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "app-1\n" })
    );
    // hasSourceCodeChanged - no diff output
    mockedSpawnSync.mockReturnValueOnce(mockSpawnResult({ stdout: "" }));

    expect(getVersion({ src: ".", tagPrefix: "app" })).toBeNull();
    expect(console.log).toHaveBeenCalledWith(
      "No changes detected since app:1 in .."
    );
  });

  it("returns next version when tag exists and changes detected", () => {
    // getPreviousTag
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "app-1\n" })
    );
    // hasSourceCodeChanged - has diff
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "src/main.ts\n" })
    );
    // getLatestVersion
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "app-1\n" })
    );

    expect(getVersion({ src: ".", tagPrefix: "app" })).toBe(2);
  });

  it("picks highest version from multiple tags", () => {
    // getPreviousTag - returns the tag reachable from HEAD
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "app-3\n" })
    );
    // hasSourceCodeChanged - has changes
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "src/main.ts\n" })
    );
    // getLatestVersion - returns 3 (highest)
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "app-3\napp-2\napp-1\n" })
    );

    expect(getVersion({ src: ".", tagPrefix: "app" })).toBe(4);
  });

  it("uses different prefix correctly", () => {
    // getPreviousTag
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "v-1\n" })
    );
    // hasSourceCodeChanged
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "src/main.ts\n" })
    );
    // getLatestVersion
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "v-1\n" })
    );

    expect(getVersion({ src: ".", tagPrefix: "v" })).toBe(2);
  });

  it("ignores non-matching prefix tags", () => {
    // getPreviousTag for "v" - only matches v-*
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "v-2\n" })
    );
    // hasSourceCodeChanged
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "src/main.ts\n" })
    );
    // getLatestVersion for "v" - only returns v-* tags
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "v-2\n" })
    );

    expect(getVersion({ src: ".", tagPrefix: "v" })).toBe(3);
  });

  it("returns null when src directory has no changes", () => {
    // getPreviousTag
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "app-1\n" })
    );
    // hasSourceCodeChanged - no changes in src dir
    mockedSpawnSync.mockReturnValueOnce(mockSpawnResult({ stdout: "" }));

    expect(getVersion({ src: "src", tagPrefix: "app" })).toBeNull();
  });

  it("returns next version when src directory has changes", () => {
    // getPreviousTag
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "app-1\n" })
    );
    // hasSourceCodeChanged - src dir has changes
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "src/main.txt\n" })
    );
    // getLatestVersion
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "app-1\n" })
    );

    expect(getVersion({ src: "src", tagPrefix: "app" })).toBe(2);
  });

  it("returns null when only ignored files changed", () => {
    // getPreviousTag
    mockedSpawnSync.mockReturnValueOnce(
      mockSpawnResult({ stdout: "app-1\n" })
    );
    // hasSourceCodeChanged with ignore - no changes after excluding ignored
    mockedSpawnSync.mockReturnValueOnce(mockSpawnResult({ stdout: "" }));

    expect(
      getVersion({
        src: ".",
        tagPrefix: "app",
        ignore: ["src/generated.txt"],
      })
    ).toBeNull();
  });
});

describe("setVersion", () => {
  it("creates a tag and pushes", () => {
    mockedSpawnSync.mockReturnValue(mockSpawnResult());

    setVersion("app", 5);

    expect(mockedSpawnSync).toHaveBeenCalledWith(
      "git",
      ["tag", "app-5"],
      { stdio: "inherit" }
    );
    expect(mockedSpawnSync).toHaveBeenCalledWith(
      "git",
      ["push", "--tags"],
      { stdio: "inherit" }
    );
    expect(console.log).toHaveBeenCalledWith("Tagging with app-5");
  });
});
