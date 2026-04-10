#!/usr/bin/env node

const { spawnSync } = require("child_process");
const path = require("path");

function getBinaryPath() {
  const platform = process.platform;
  const arch = process.arch;

  let pkgName;
  if (platform === "darwin" && arch === "x64") {
    pkgName = "@chenpu17/serve-here-darwin-x64";
  } else if (platform === "darwin" && arch === "arm64") {
    pkgName = "@chenpu17/serve-here-darwin-arm64";
  } else if (platform === "linux" && arch === "x64") {
    pkgName = "@chenpu17/serve-here-linux-x64";
  } else if (platform === "linux" && arch === "arm64") {
    pkgName = "@chenpu17/serve-here-linux-arm64";
  } else if (platform === "win32" && arch === "x64") {
    pkgName = "@chenpu17/serve-here-windows-x64";
  } else {
    console.error("Unsupported platform: " + platform + "-" + arch);
    process.exit(1);
  }

  const binaryName = platform === "win32" ? "serve-here.exe" : "serve-here";
  try {
    return require.resolve(pkgName + "/bin/" + binaryName);
  } catch (e) {
    console.error("Could not find binary for " + pkgName + ".");
    console.error("Try reinstalling the package.");
    process.exit(1);
  }
}

const binaryPath = getBinaryPath();
const args = process.argv.slice(2);
const result = spawnSync(binaryPath, args, { stdio: "inherit" });
if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}
process.exit(result.status ?? 1);
