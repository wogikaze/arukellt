const fs = require('fs')
const path = require('path')

const REPO_PLAYGROUND_URL = 'https://wogikaze.github.io/arukellt/playground/'

function fail(message) {
  console.error(`[playground-endpoint-guard] ${message}`)
  process.exit(1)
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'))
}

const repoRoot = path.resolve(__dirname, '..', '..', '..')
const extensionRoot = path.resolve(__dirname, '..')

const packageJsonPath = path.join(extensionRoot, 'package.json')
const extensionJsPath = path.join(extensionRoot, 'src', 'extension.js')
const readmePath = path.join(extensionRoot, 'README.md')
const docsPlaygroundPath = path.join(repoRoot, 'docs', 'playground', 'index.html')

if (!fs.existsSync(docsPlaygroundPath)) {
  fail('Missing docs/playground/index.html route proof.')
}

const docsPlaygroundHtml = fs.readFileSync(docsPlaygroundPath, 'utf8')
if (!docsPlaygroundHtml.includes('<title>Arukellt Playground</title>')) {
  fail('docs/playground/index.html exists but does not look like the playground entrypoint.')
}

const packageJson = readJson(packageJsonPath)
const setting = packageJson?.contributes?.configuration?.properties?.['arukellt.playgroundUrl']
if (!setting) {
  fail('Missing arukellt.playgroundUrl setting in package.json.')
}

if (setting.default !== REPO_PLAYGROUND_URL) {
  fail(`Expected arukellt.playgroundUrl.default to be ${REPO_PLAYGROUND_URL}, got ${setting.default}`)
}

if (!Array.isArray(setting.enum) || setting.enum.length !== 1 || setting.enum[0] !== REPO_PLAYGROUND_URL) {
  fail('arukellt.playgroundUrl enum must allow only the repo-proved endpoint.')
}

const extensionJs = fs.readFileSync(extensionJsPath, 'utf8')
if (!extensionJs.includes(`const REPO_PLAYGROUND_URL = '${REPO_PLAYGROUND_URL}'`)) {
  fail('extension.js must define REPO_PLAYGROUND_URL with the repo-proved endpoint.')
}
if (!extensionJs.includes('ALLOWED_PLAYGROUND_BASE_URLS.has(normalizedPlaygroundUrl)')) {
  fail('extension.js must guard openInPlayground with ALLOWED_PLAYGROUND_BASE_URLS.')
}

const readme = fs.readFileSync(readmePath, 'utf8')
if (!readme.includes(REPO_PLAYGROUND_URL)) {
  fail('README.md must document the repo-proved playground URL.')
}
if (!readme.includes('Only the repo-proved route is supported')) {
  fail('README.md must state that only the repo-proved route is supported.')
}

console.log('[playground-endpoint-guard] PASS')
