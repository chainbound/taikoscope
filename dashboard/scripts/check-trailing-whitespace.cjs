#!/usr/bin/env node
/* eslint-env node */
const { execSync } = require('child_process');
const fs = require('fs');

const files = execSync('git ls-files dashboard')
  .toString()
  .split('\n')
  .filter((f) => /\.(tsx?|jsx?|css|md|html)$/.test(f));

let hasTrailing = false;
for (const file of files) {
  const content = fs.readFileSync(file, 'utf8');
  const lines = content.split(/\r?\n/);
  lines.forEach((line, idx) => {
    if (/[ \t]+$/.test(line)) {
      console.log(`${file}:${idx + 1}: trailing whitespace`);
      hasTrailing = true;
    }
  });
}

if (hasTrailing) {
  console.error('Trailing whitespace found.');
  process.exit(1);
}
