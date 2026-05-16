#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const vsceNpm = require("../node_modules/@vscode/vsce/out/npm.js");

if (typeof globalThis.File === "undefined") {
  globalThis.File = class File {};
}

function productionDependenciesFromLock(cwd) {
  const lockPath = path.join(cwd, "package-lock.json");
  const lock = JSON.parse(fs.readFileSync(lockPath, "utf8"));
  const packages = lock.packages || {};
  const result = [cwd];
  const seen = new Set();
  const queue = Object.keys(packages[""]?.dependencies || {});

  while (queue.length > 0) {
    const name = queue.shift();
    if (seen.has(name)) {
      continue;
    }
    seen.add(name);

    const packagePath = `node_modules/${name}`;
    const metadata = packages[packagePath];
    if (!metadata || metadata.dev) {
      continue;
    }

    result.push(path.join(cwd, packagePath));
    for (const dependency of Object.keys(metadata.dependencies || {})) {
      queue.push(dependency);
    }
  }

  return result;
}

const getDependencies = vsceNpm.getDependencies;
vsceNpm.getDependencies = async (cwd, dependencies, packagedDependencies) => {
  const lockPath = path.join(cwd, "package-lock.json");
  if (dependencies !== "none" && fs.existsSync(lockPath) && !packagedDependencies) {
    return productionDependenciesFromLock(cwd);
  }
  return getDependencies(cwd, dependencies, packagedDependencies);
};

require("../node_modules/@vscode/vsce/vsce");
