import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const ignoredDirectories = new Set([
  ".git",
  "node_modules",
  "target",
  "dist",
  "outputs",
  "coverage",
]);
const textExtensions = new Set([
  ".css",
  ".html",
  ".js",
  ".json",
  ".md",
  ".mjs",
  ".rs",
  ".toml",
  ".ts",
  ".tsx",
  ".yaml",
  ".yml",
]);
const patterns = [
  { name: "OpenAI/DeepSeek-style key", regex: /\bsk-[A-Za-z0-9_-]{20,}\b/g },
  { name: "Bearer token", regex: /\bBearer\s+[A-Za-z0-9._-]{24,}\b/g },
  {
    name: "Assigned credential",
    regex:
      /\b(?:api[_-]?key|access[_-]?token|client[_-]?secret|password)\b\s*[:=]\s*["'][A-Za-z0-9_./+-]{24,}["']/gi,
  },
  {
    name: "Private key",
    regex: /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/g,
  },
];

const findings = [];

function scanFile(filePath) {
  const extension = path.extname(filePath).toLowerCase();
  if (!textExtensions.has(extension)) return;
  const relative = path.relative(root, filePath);
  if (relative.startsWith("docs/screenshots/")) return;

  const text = fs.readFileSync(filePath, "utf8");
  const lines = text.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    for (const pattern of patterns) {
      pattern.regex.lastIndex = 0;
      if (pattern.regex.test(lines[index])) {
        findings.push({
          file: relative,
          line: index + 1,
          type: pattern.name,
        });
      }
    }
  }
}

function walk(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && ignoredDirectories.has(entry.name)) continue;
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(fullPath);
    else scanFile(fullPath);
  }
}

walk(root);

if (findings.length > 0) {
  console.error("Potential secrets found:");
  for (const finding of findings) {
    console.error(`- ${finding.file}:${finding.line} (${finding.type})`);
  }
  process.exit(1);
}

console.log("Secret scan passed.");
