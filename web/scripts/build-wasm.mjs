import { spawnSync } from "node:child_process";
import { homedir } from "node:os";
import { dirname, join } from "node:path";

const cargoBin = join(homedir(), ".cargo", "bin");
process.env.PATH = `${cargoBin}:${process.env.PATH ?? ""}`;

function run(command, args, options = {}) {
  return spawnSync(command, args, {
    stdio: "inherit",
    env: process.env,
    ...options,
  });
}

function has(command, args = ["--version"]) {
  return spawnSync(command, args, {
    stdio: "ignore",
    env: process.env,
  }).status === 0;
}

function output(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    env: process.env,
  });
  if (result.status !== 0) {
    return null;
  }
  return result.stdout.trim();
}

function requireSuccess(result, label) {
  if (result.status !== 0) {
    process.exitCode = result.status ?? 1;
    throw new Error(`${label} failed`);
  }
}

function ensureRust() {
  if (has("rustup")) {
    return;
  }

  console.log("rustup not found; installing minimal stable Rust...");
  requireSuccess(
    run("sh", [
      "-c",
      "curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain stable",
    ]),
    "Rust install",
  );
}

function configureRustupToolchain() {
  process.env.RUSTUP_TOOLCHAIN = process.env.RUSTUP_TOOLCHAIN ?? "stable";
  requireSuccess(
    run("rustup", ["target", "add", "wasm32-unknown-unknown"]),
    "wasm32 target install",
  );

  const rustc = output("rustup", ["which", "rustc"]);
  const cargo = output("rustup", ["which", "cargo"]);
  if (rustc) {
    process.env.RUSTC = rustc;
  }
  if (cargo) {
    process.env.CARGO = cargo;
    process.env.PATH = `${dirname(cargo)}:${process.env.PATH ?? ""}`;
  }
}

function ensureWasmPack() {
  if (has("wasm-pack")) {
    return;
  }

  console.log("wasm-pack not found; trying the official rustwasm installer...");
  const officialInstall = run("sh", [
    "-c",
    "curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh",
  ]);
  if (officialInstall.status === 0 && has("wasm-pack")) {
    return;
  }

  console.log("Official wasm-pack installer failed; falling back to cargo install...");
  requireSuccess(
    run("cargo", ["install", "wasm-pack", "--locked"]),
    "wasm-pack cargo install",
  );
}

ensureRust();
configureRustupToolchain();
ensureWasmPack();

requireSuccess(
  run("wasm-pack", [
    "build",
    "../wasm",
    "--target",
    "web",
    "--release",
    "--out-dir",
    "../web/src/wasm-pkg",
  ]),
  "wasm-pack build",
);
