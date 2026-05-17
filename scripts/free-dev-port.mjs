import { execFileSync } from "node:child_process";

const port = Number.parseInt(process.argv[2] ?? "1420", 10);

if (!Number.isInteger(port) || port <= 0 || port > 65_535) {
  console.error(`Invalid port: ${process.argv[2]}`);
  process.exit(1);
}

const run = (command, args, options = {}) =>
  execFileSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    ...options,
  });

const getWindowsListeners = () => {
  const script = [
    `$connections = Get-NetTCPConnection -LocalPort ${port} -State Listen -ErrorAction SilentlyContinue`,
    "$connections | Select-Object -ExpandProperty OwningProcess -Unique",
  ].join("; ");

  try {
    return run("powershell.exe", [
      "-NoProfile",
      "-ExecutionPolicy",
      "Bypass",
      "-Command",
      script,
    ]);
  } catch {
    return "";
  }
};

const getUnixListeners = () => {
  try {
    return run("lsof", ["-ti", `tcp:${port}`, "-sTCP:LISTEN"]);
  } catch {
    try {
      return run("fuser", ["-n", "tcp", String(port)]);
    } catch {
      return "";
    }
  }
};

const listenerOutput = process.platform === "win32" ? getWindowsListeners() : getUnixListeners();
const processIds = [
  ...new Set(
    listenerOutput
      .split(/\s+/)
      .map((value) => Number.parseInt(value, 10))
      .filter((value) => Number.isInteger(value) && value > 0 && value !== process.pid),
  ),
];

if (processIds.length === 0) {
  process.exit(0);
}

console.error(`Dev port ${port} is already in use by process ${processIds.join(", ")}.`);
console.error("Stop that process, then rerun `npm run dev` or `cargo tauri dev`.");
process.exit(1);
