const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));

const requiredFields = [
  'publisher',
  'icon',
  'galleryBanner',
  'categories',
  'keywords',
  'repository',
  'bugs',
  'homepage',
  'engines',
];

const failures = [];

for (const field of requiredFields) {
  if (!pkg[field] || (Array.isArray(pkg[field]) && pkg[field].length === 0)) {
    failures.push(`package.json missing ${field}`);
  }
}

for (const category of ['Programming Languages', 'Linters', 'Debuggers', 'Testing']) {
  if (!pkg.categories?.includes(category)) {
    failures.push(`package.json categories missing ${category}`);
  }
}

for (const keyword of ['arukellt', 'ark', 'wasm', 'webassembly']) {
  if (!pkg.keywords?.includes(keyword)) {
    failures.push(`package.json keywords missing ${keyword}`);
  }
}

for (const file of [pkg.icon, 'README.md', 'CHANGELOG.md', 'RELEASE.md']) {
  if (!fs.existsSync(path.join(root, file))) {
    failures.push(`required marketplace file missing: ${file}`);
  }
}

const readme = fs.readFileSync(path.join(root, 'README.md'), 'utf8');
for (const phrase of [
  'media/command-palette.png',
  'Supported Targets',
  'Troubleshooting',
  'Packaging and Release',
]) {
  if (!readme.includes(phrase)) {
    failures.push(`README.md missing ${phrase}`);
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(failure);
  }
  process.exit(1);
}

console.log('marketplace metadata OK');
