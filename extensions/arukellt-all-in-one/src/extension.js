const vscode = require('vscode')
const cp = require('child_process')
const os = require('os')
const path = require('path')
const fs = require('fs')
const { LanguageClient, TransportKind } = require('vscode-languageclient/node')

let client = null
let isDeactivating = false
let suppressClientRestart = false
let outputChannel = null
let compilerChannel = null
let testChannel = null
let statusBarItem = null
let restartCount = 0
let clientSessionId = 0
let languageStatusItem = null
let testController = null
let projectTreeProvider = null
let componentDiagnostics = null
let testTelemetry = {
  lastErrorMessage: null,
  outputChannelLines: [],
  recordedErrorNotificationCount: 0,
}
let debugSessionState = {
  startCount: 0,
  terminateCount: 0,
  activeSessionName: null,
  activeSessionType: null,
  lastStoppedEvent: null,
  stoppedEventCount: 0,
}

const REPO_PLAYGROUND_URL = 'https://wogikaze.github.io/arukellt/playground/'
const ALLOWED_PLAYGROUND_BASE_URLS = new Set([REPO_PLAYGROUND_URL])

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
  // Output channels — categorized by purpose
  outputChannel = vscode.window.createOutputChannel('Arukellt Language Server')
  compilerChannel = vscode.window.createOutputChannel('Arukellt Compiler')
  testChannel = vscode.window.createOutputChannel('Arukellt Tests')

  statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100)
  statusBarItem.name = 'Arukellt'

  componentDiagnostics = vscode.languages.createDiagnosticCollection('arukellt-component')

  context.subscriptions.push(outputChannel)
  context.subscriptions.push(compilerChannel)
  context.subscriptions.push(testChannel)
  context.subscriptions.push(statusBarItem)
  context.subscriptions.push(componentDiagnostics)

  // Language status item — shows LSP server state
  setupLanguageStatus(context)

  registerCommands(context)
  registerTaskProvider(context)
  setupTestController(context)
  registerDebugAdapter(context)
  setupProjectTreeView(context)

  startLanguageServer(context)

  return {
    __getTestState: getTestState,
    shutdownForTests,
    verifyBootstrap,
  }
}

async function deactivate() {
  isDeactivating = true
  try {
    await shutdownLanguageClient({ timeoutMs: 5000, logFailure: false })
  } finally {
    isDeactivating = false
  }
}

function getConfiguration() {
  return vscode.workspace.getConfiguration('arukellt')
}

/** Returns ordered list of candidate binary paths for arukellt. */
function getCandidatePaths(configuredPath) {
  const candidates = []
  // 1. Explicitly configured server.path (if not the default placeholder)
  if (configuredPath && configuredPath !== 'arukellt') {
    candidates.push({ path: configuredPath, source: 'arukellt.server.path setting' })
    return candidates
  }
  // 2. Repo-local builds when running the extension from a source checkout.
  const exeName = process.platform === 'win32' ? 'arukellt.exe' : 'arukellt'
  const repoRoot = path.resolve(__dirname, '..', '..', '..')
  for (const rel of [
    path.join('target', 'debug', exeName),
    path.join('target', 'release', exeName),
  ]) {
    candidates.push({ path: path.join(repoRoot, rel), source: `repo build: ${rel}` })
  }
  // 3. PATH lookup (default name)
  candidates.push({ path: 'arukellt', source: 'PATH' })
  // 4. Default install locations
  const homeDir = os.homedir()
  const defaultPaths = [
    path.join(homeDir, '.ark', 'bin', 'arukellt'),
    path.join(homeDir, '.cargo', 'bin', 'arukellt'),
    '/usr/local/bin/arukellt',
  ]
  for (const p of defaultPaths) {
    candidates.push({ path: p, source: `default install: ${p}` })
  }
  return candidates
}

function resolveServerCommand() {
  const config = getConfiguration()
  const configuredPath = config.get('server.path', 'arukellt')
  const extraArgs = config.get('server.args', [])
  return { command: configuredPath, extraArgs }
}

function probeServerBinary(command) {
  try {
    const result = cp.spawnSync(command, ['--version'], { encoding: 'utf8' })
    if (result.error) {
      if (result.error.code === 'ENOENT') {
        return { ok: false, message: `binary not found: '${command}'. Set arukellt.server.path to an absolute path.`, notFound: true }
      }
      return { ok: false, message: result.error.message }
    }
    if (result.status !== 0) {
      return { ok: false, message: (result.stderr || result.stdout || 'failed to execute arukellt --version').trim() }
    }
    return { ok: true, version: (result.stdout || result.stderr || '').trim() }
  } catch (error) {
    return { ok: false, message: error.message }
  }
}

/**
 * Search all candidate paths for a working arukellt binary.
 * Logs each probe attempt to the output channel.
 * Returns { command, probe } where probe has ok/version/message.
 */
function discoverBinary(configuredPath) {
  const candidates = getCandidatePaths(configuredPath)
  appendLanguageServerOutputLine('[binary discovery] searching for arukellt binary...')
  for (const candidate of candidates) {
    const probe = probeServerBinary(candidate.path)
    if (probe.ok) {
      appendLanguageServerOutputLine(`[binary discovery] found via ${candidate.source}: ${candidate.path} (${probe.version})`)
    } else {
      appendLanguageServerOutputLine(`[binary discovery] not found via ${candidate.source}: ${probe.message}`)
    }
    if (probe.ok) {
      return { command: candidate.path, probe }
    }
  }
  appendLanguageServerOutputLine('[binary discovery] arukellt binary not found in any location')
  appendLanguageServerOutputLine('[binary discovery] install guide: https://github.com/arukellt/arukellt#installation')
  return { command: configuredPath, probe: { ok: false, message: 'arukellt binary not found. Install via cargo or set arukellt.server.path.' } }
}

function startLanguageServer(context) {
  resetLanguageServerTestTelemetry()
  const config = getConfiguration()
  const configuredPath = config.get('server.path', 'arukellt')
  const extraArgs = config.get('server.args', [])
  const { command, probe } = discoverBinary(configuredPath)

  if (!probe.ok) {
    updateStatus('$(error) Arukellt: binary missing', 'Failed to find arukellt binary')
    updateLanguageStatus('error', 'Binary not found')
    showRecordedErrorMessage(
      `Arukellt: arukellt binary not found. ${probe.message} ` +
      'See the output channel for details, or set arukellt.server.path in settings.',
      'Open Output', 'Open Settings'
    ).then(sel => {
      if (sel === 'Open Output' && outputChannel) outputChannel.show()
      if (sel === 'Open Settings') vscode.commands.executeCommand('workbench.action.openSettings', 'arukellt.server.path')
    })
    return
  }

  // Warn if selfhost backend is requested but not yet available.
  const useSelfHostBackend = config.get('useSelfHostBackend', false)
  if (useSelfHostBackend) {
    if (outputChannel) {
      outputChannel.appendLine(
        '[arukellt] WARNING: arukellt.useSelfHostBackend=true but selfhost backend requires ' +
        'Stage 2 fixpoint (Issue 459). Continuing with the Rust backend.'
      )
    }
  }

  const serverOptions = {
    run: {
      command,
      args: [...extraArgs, 'lsp'],
      transport: TransportKind.stdio,
    },
    debug: {
      command,
      args: [...extraArgs, 'lsp'],
      transport: TransportKind.stdio,
    },
  }

  const clientOptions = {
    documentSelector: [{ scheme: 'file', language: 'arukellt' }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.ark'),
      configurationSection: 'arukellt',
    },
    outputChannel,
    // Pass current settings as initializationOptions so the LSP server can
    // apply them on startup without waiting for didChangeConfiguration.
    initializationOptions: {
      enableCodeLens: config.get('enableCodeLens', true),
      hoverDetailLevel: config.get('hoverDetailLevel', 'full'),
      // arkTarget: null means auto-detect; non-null value is forwarded as project_target.
      arkTarget: config.get('target', null),
      diagnosticsReportLevel: config.get('diagnostics.reportLevel', 'all'),
      useSelfHostBackend: config.get('useSelfHostBackend', false),
      checkOnSave: config.get('check.onSave', true),
    },
    errorHandler: {
      error(error, message, count) {
        if (count && count <= 3) {
          return { action: 1 /* Continue */ }
        }
        return { action: 2 /* Shutdown */ }
      },
      closed() {
        if (isDeactivating || suppressClientRestart) {
          return { action: 2 /* DoNotRestart */ }
        }
        if (restartCount < 5) {
          restartCount++
          updateLanguageStatus('warning', `Restarting (attempt ${restartCount})…`)
          return { action: 1 /* Restart */ }
        }
        updateLanguageStatus('error', 'Server crashed repeatedly')
        vscode.window.showErrorMessage(
          'Arukellt language server crashed 5 times. Use "Arukellt: Restart Language Server" to try again.',
        )
        return { action: 2 /* DoNotRestart */ }
      },
    },
  }

  updateLanguageStatus('starting')

  client = new LanguageClient('arukellt', 'Arukellt Language Server', serverOptions, clientOptions)
  clientSessionId++

  client.start().then(() => {
    restartCount = 0
    updateStatus('Arukellt: $(check) LSP running', probe.version || command)
    updateLanguageStatus('ready', probe.version || command)

    // Forward configuration changes to the LSP server via workspace/didChangeConfiguration.
    context.subscriptions.push(
      vscode.workspace.onDidChangeConfiguration(e => {
        if (!e.affectsConfiguration('arukellt')) return
        const cfg = getConfiguration()
        const newUseSelfHost = cfg.get('useSelfHostBackend', false)
        if (newUseSelfHost && outputChannel) {
          outputChannel.appendLine(
            '[arukellt] WARNING: arukellt.useSelfHostBackend=true but selfhost backend requires ' +
            'Stage 2 fixpoint (Issue 459). Continuing with the Rust backend.'
          )
        }
        if (client) {
          client.sendNotification('workspace/didChangeConfiguration', {
            settings: {
              arukellt: {
                enableCodeLens: cfg.get('enableCodeLens', true),
                hoverDetailLevel: cfg.get('hoverDetailLevel', 'full'),
                arkTarget: cfg.get('target', null),
                diagnosticsReportLevel: cfg.get('diagnostics.reportLevel', 'all'),
                useSelfHostBackend: cfg.get('useSelfHostBackend', false),
                checkOnSave: cfg.get('check.onSave', true),
              },
            },
          })
        }
      })
    )
  }).catch((err) => {
    updateStatus('$(error) Arukellt: start failed', err.message)
    updateLanguageStatus('error', err.message)
    vscode.window.showErrorMessage(`Arukellt: failed to start language server: ${err.message}`)
  })
}

async function stopClientWithTimeout(currentClient, timeoutMs) {
  let timeoutHandle = null
  try {
    await Promise.race([
      currentClient.stop(timeoutMs),
      new Promise((_, reject) => {
        timeoutHandle = setTimeout(() => {
          reject(new Error(`timed out after ${timeoutMs + 250}ms`))
        }, timeoutMs + 250)
      }),
    ])
  } finally {
    if (timeoutHandle) {
      clearTimeout(timeoutHandle)
    }
  }
}

async function shutdownLanguageClient(options = {}) {
  const timeoutMs = options.timeoutMs ?? 2000
  const logFailure = options.logFailure ?? true
  const logContext = options.logContext ?? 'shutdown'
  const currentClient = client
  client = null
  if (!currentClient) {
    return
  }

  const previousSuppressRestart = suppressClientRestart
  suppressClientRestart = true
  try {
    await stopClientWithTimeout(currentClient, timeoutMs)
  } catch (err) {
    if (logFailure && outputChannel) {
      outputChannel.appendLine(`[arukellt] ${logContext}: stop failed (${err.message})`)
    }
  } finally {
    suppressClientRestart = previousSuppressRestart
  }
}

async function shutdownForTests() {
  await shutdownLanguageClient({ timeoutMs: 5000, logFailure: false, logContext: 'test shutdown' })
}

async function restartLanguageServer(context) {
  restartCount = 0
  if (client) {
    await shutdownLanguageClient({ timeoutMs: 5000, logFailure: true, logContext: 'restart' })
    startLanguageServer(context)
    vscode.window.showInformationMessage('Arukellt language server restarted.')
  } else {
    startLanguageServer(context)
  }
}

function updateStatus(text, tooltip) {
  if (!statusBarItem) return
  statusBarItem.text = text
  statusBarItem.tooltip = tooltip || text
  statusBarItem.show()
}

function appendLanguageServerOutputLine(line) {
  if (outputChannel) {
    outputChannel.appendLine(line)
  }
  testTelemetry.outputChannelLines.push(line)
  if (testTelemetry.outputChannelLines.length > 100) {
    testTelemetry.outputChannelLines.shift()
  }
}

function resetLanguageServerTestTelemetry() {
  testTelemetry.lastErrorMessage = null
  testTelemetry.outputChannelLines = []
  testTelemetry.recordedErrorNotificationCount = 0
}

function showRecordedErrorMessage(message, ...items) {
  testTelemetry.lastErrorMessage = message
  testTelemetry.recordedErrorNotificationCount += 1
  return vscode.window.showErrorMessage(message, ...items)
}

function runCliCommand(kind) {
  const editor = vscode.window.activeTextEditor
  if (!editor) {
    vscode.window.showErrorMessage('Arukellt: no active editor.')
    return
  }
  const file = editor.document.uri.fsPath
  if (!file.endsWith('.ark')) {
    vscode.window.showErrorMessage('Arukellt: current file is not an .ark source file.')
    return
  }

  const config = vscode.workspace.getConfiguration('arukellt', editor.document.uri)
  const target = config.get('target', 'wasm32-wasi-p1')
  const emit = config.get('emit', 'core-wasm')

  const { command } = resolveServerCommand()
  let args = []
  if (kind === 'check') {
    args = ['check', file, '--target', target]
  } else if (kind === 'compile') {
    args = ['compile', file, '--target', target, '--emit', emit]
  } else if (kind === 'run') {
    args = ['run', file, '--target', target]
  }

  compilerChannel.appendLine(`$ ${command} ${args.join(' ')}`)
  compilerChannel.show()

  const child = cp.spawn(command, args)
  child.stdout.on('data', (data) => compilerChannel.append(data.toString()))
  child.stderr.on('data', (data) => compilerChannel.append(data.toString()))
  child.on('close', (code) => {
    compilerChannel.appendLine(`Arukellt ${kind} exited with code ${code}`)
  })
}

// ---------------------------------------------------------------------------
// Component build / run / inspect helpers (Issue #444)
// ---------------------------------------------------------------------------

/**
 * Parse stderr text for diagnostic lines in two supported formats:
 *   (1) error[E001]: message at file.ark:10:5
 *   (2) error[E001]: message\n  --> file.ark:10:5
 * Returns a Map<uriString, {uri, diags[]}> ready to push.
 */
function parseStderrDiagnostics(stderr) {
  const diagnosticsMap = new Map()

  function addEntry(filePath, severity, message, line, col) {
    const resolved = path.isAbsolute(filePath)
      ? filePath
      : (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders[0]
          ? path.join(vscode.workspace.workspaceFolders[0].uri.fsPath, filePath)
          : filePath)
    const uri = vscode.Uri.file(resolved)
    const key = uri.toString()
    if (!diagnosticsMap.has(key)) diagnosticsMap.set(key, { uri, diags: [] })
    const zeroLine = Math.max(0, line - 1)
    const zeroCol = Math.max(0, col - 1)
    const range = new vscode.Range(zeroLine, zeroCol, zeroLine, zeroCol + 1)
    const sev = severity === 'error'
      ? vscode.DiagnosticSeverity.Error
      : severity === 'warning'
        ? vscode.DiagnosticSeverity.Warning
        : vscode.DiagnosticSeverity.Information
    diagnosticsMap.get(key).diags.push(new vscode.Diagnostic(range, message, sev))
  }

  // Pattern 1: inline "error[E001]: msg at file:line:col"
  const inlineRe = /^(error|warning|info)(?:\[[A-Z]\d+\])?:\s*(.+?)\s+at\s+(\S+?):(\d+):(\d+)\s*$/gm
  let m
  while ((m = inlineRe.exec(stderr)) !== null) {
    addEntry(m[3], m[1], m[2], parseInt(m[4], 10), parseInt(m[5], 10))
  }

  // Pattern 2: arrow "error[E001]: msg\n  --> file:line:col"
  const arrowRe = /^(error|warning|info)(?:\[[A-Z]\d+\])?:\s*(.+)\r?\n\s+-->\s+(\S+?):(\d+):(\d+)/gm
  while ((m = arrowRe.exec(stderr)) !== null) {
    addEntry(m[3], m[1], m[2], parseInt(m[4], 10), parseInt(m[5], 10))
  }

  return diagnosticsMap
}

/** Apply parsed diagnostics to the DiagnosticCollection, clearing stale entries. */
function applyDiagnostics(diagMap, fileUri) {
  if (!componentDiagnostics) return
  componentDiagnostics.clear()
  if (diagMap.size > 0) {
    for (const { uri, diags } of diagMap.values()) {
      componentDiagnostics.set(uri, diags)
    }
  } else if (fileUri) {
    componentDiagnostics.set(fileUri, [])
  }
}

/** Resolve the active .ark file or show an error and return null. */
function resolveActiveArkFile() {
  const editor = vscode.window.activeTextEditor
  if (!editor) {
    vscode.window.showErrorMessage('Arukellt: no active editor.')
    return null
  }
  const file = editor.document.uri.fsPath
  if (!file.endsWith('.ark')) {
    vscode.window.showErrorMessage('Arukellt: current file is not an .ark source file.')
    return null
  }
  return { file, uri: editor.document.uri }
}

/**
 * arukellt.buildComponent
 * Compiles the current .ark file as a component (--target wasm32-wasi-p2 --emit all),
 * shows output paths and sizes in the compiler output channel, and pushes diagnostics.
 */
async function buildComponent() {
  const resolved = resolveActiveArkFile()
  if (!resolved) return
  const { file, uri } = resolved

  const { command } = resolveServerCommand()
  const args = ['compile', file, '--target', 'wasm32-wasi-p2', '--emit', 'all']

  compilerChannel.appendLine(`$ ${command} ${args.join(' ')}`)
  compilerChannel.show()

  return new Promise((resolve) => {
    let stderr = ''
    const child = cp.spawn(command, args)
    child.stdout.on('data', (data) => compilerChannel.append(data.toString()))
    child.stderr.on('data', (data) => {
      const text = data.toString()
      stderr += text
      compilerChannel.append(text)
    })
    child.on('close', (code) => {
      const diagMap = parseStderrDiagnostics(stderr)
      applyDiagnostics(diagMap, uri)

      if (code === 0) {
        compilerChannel.appendLine('✓ Component build succeeded')
        // Report output files with their sizes
        const dir = path.dirname(file)
        const base = path.basename(file, '.ark')
        const outputs = [
          { ext: '.wasm', label: 'Component Wasm' },
          { ext: '.wit',  label: 'WIT interface'  },
          { ext: '.wat',  label: 'WAT text'        },
        ]
        for (const out of outputs) {
          const outPath = path.join(dir, base + out.ext)
          try {
            const stat = fs.statSync(outPath)
            compilerChannel.appendLine(`  ${out.label}: ${outPath} (${stat.size} bytes)`)
          } catch (_) { /* file not produced for this emit */ }
        }
      } else {
        compilerChannel.appendLine(`✗ Component build failed (exit ${code})`)
      }
      resolve()
    })
  })
}

/**
 * arukellt.buildComponentWit
 * Compiles the current .ark file with --emit wit and displays the WIT text
 * in a new VS Code document opened beside the current editor.
 */
async function buildComponentWit() {
  const resolved = resolveActiveArkFile()
  if (!resolved) return
  const { file, uri } = resolved

  const { command } = resolveServerCommand()
  const args = ['compile', file, '--target', 'wasm32-wasi-p2', '--emit', 'wit']

  compilerChannel.appendLine(`$ ${command} ${args.join(' ')}`)
  compilerChannel.show()

  return new Promise((resolve) => {
    let stdout = ''
    let stderr = ''
    const child = cp.spawn(command, args)
    child.stdout.on('data', (data) => { stdout += data.toString() })
    child.stderr.on('data', (data) => {
      const text = data.toString()
      stderr += text
      compilerChannel.append(text)
    })
    child.on('close', (code) => {
      const diagMap = parseStderrDiagnostics(stderr)
      applyDiagnostics(diagMap, uri)

      if (code === 0) {
        const witContent = stdout.trim() || '// (no WIT output produced)'
        vscode.workspace.openTextDocument({ content: witContent, language: 'wit' }).then(doc => {
          vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside)
        })
      } else {
        compilerChannel.appendLine(`✗ WIT generation failed (exit ${code})`)
      }
      resolve()
    })
  })
}

/**
 * arukellt.runComponent
 * Runs the current .ark file as a component (--target wasm32-wasi-p2).
 * Stderr is parsed for diagnostics; stdout/stderr are streamed to the compiler channel.
 */
async function runComponent() {
  const resolved = resolveActiveArkFile()
  if (!resolved) return
  const { file, uri } = resolved

  const { command } = resolveServerCommand()
  const args = ['run', file, '--target', 'wasm32-wasi-p2']

  compilerChannel.appendLine(`$ ${command} ${args.join(' ')}`)
  compilerChannel.show()

  return new Promise((resolve) => {
    let stderr = ''
    const child = cp.spawn(command, args)
    child.stdout.on('data', (data) => compilerChannel.append(data.toString()))
    child.stderr.on('data', (data) => {
      const text = data.toString()
      stderr += text
      compilerChannel.append(text)
    })
    child.on('close', (code) => {
      const diagMap = parseStderrDiagnostics(stderr)
      applyDiagnostics(diagMap, uri)
      compilerChannel.appendLine(`arukellt run --target wasm32-wasi-p2 exited with code ${code}`)
      resolve()
    })
  })
}

function normalizePlaygroundBaseUrl(value) {
  if (!value) {
    return ''
  }
  try {
    const parsed = new URL(value)
    parsed.search = ''
    parsed.hash = ''
    if (!parsed.pathname.endsWith('/')) {
      parsed.pathname = `${parsed.pathname}/`
    }
    return parsed.toString()
  } catch (_error) {
    return ''
  }
}

/**
 * arukellt.openInPlayground
 * Opens the current file's source in the repo-proved playground endpoint.
 * For files ≤ 2 000 characters the source is appended as ?src=<encoded>.
 * Larger files open the base URL (user can paste manually).
 */
async function openInPlayground() {
  const editor = vscode.window.activeTextEditor
  if (!editor) {
    vscode.window.showErrorMessage('Arukellt: no active editor.')
    return
  }
  const source = editor.document.getText()
  const config = vscode.workspace.getConfiguration('arukellt', editor.document.uri)
  const playgroundUrl = config.get('playgroundUrl', REPO_PLAYGROUND_URL).trim()
  const normalizedPlaygroundUrl = normalizePlaygroundBaseUrl(playgroundUrl)

  if (!normalizedPlaygroundUrl) {
    vscode.window.showErrorMessage(
      'Arukellt: invalid playground URL. Reset arukellt.playgroundUrl to the default repo endpoint.'
    )
    return
  }

  if (!ALLOWED_PLAYGROUND_BASE_URLS.has(normalizedPlaygroundUrl)) {
    vscode.window.showErrorMessage(
      `Arukellt: unsupported playground URL. Only ${REPO_PLAYGROUND_URL} is supported.`
    )
    return
  }

  const MAX_SRC_LENGTH = 2000
  let uri
  if (source.length <= MAX_SRC_LENGTH) {
    uri = vscode.Uri.parse(normalizedPlaygroundUrl).with({
      query: new URLSearchParams({ src: source }).toString(),
    })
  } else {
    uri = vscode.Uri.parse(normalizedPlaygroundUrl)
    vscode.window.showInformationMessage(
      'Arukellt: source is too large to encode in a URL. Opening playground — paste your code manually.'
    )
  }

  vscode.env.openExternal(uri)
}

function showSetupDoctor() {
  const config = getConfiguration()
  const { command: configuredPath, extraArgs } = resolveServerCommand()
  const { command, probe } = discoverBinary(configuredPath)

  const lines = [
    '# Arukellt Setup Doctor',
    '',
    `- Configured path: ${configuredPath}`,
    `- Resolved binary: ${command}`,
    `- CLI args before lsp: ${extraArgs.join(' ') || '(none)'}`,
    `- CLI probe: ${probe.ok ? `ok (${probe.version || 'version unknown'})` : `failed (${probe.message})`}`,
    `- Default target: ${config.get('target', 'wasm32-wasi-p1')}`,
    `- Default emit: ${config.get('emit', 'core-wasm')}`,
    `- Workspace folders: ${vscode.workspace.workspaceFolders ? vscode.workspace.workspaceFolders.map(f => f.uri.fsPath).join(', ') : '(none)'}`,
  ]
  outputChannel.clear()
  outputChannel.appendLine(lines.join('\n'))
  outputChannel.show()
  vscode.window.showInformationMessage('Arukellt setup doctor written to the output channel.')
}

function showCommandGraph() {
  const lines = [
    '# Arukellt Command Graph',
    '',
    'check -> compile -> run',
    '   \\-> restart language server',
    '',
    '- check: arukellt check <file> --target <target>',
    '- compile: arukellt compile <file> --target <target> --emit <emit>',
    '- run: arukellt run <file> --target <target>',
  ]
  outputChannel.clear()
  outputChannel.appendLine(lines.join('\n'))
  outputChannel.show()
}

function showEnvironmentDiff() {
  const config = getConfiguration()
  const { command, extraArgs } = resolveServerCommand()

  const lines = [
    '# Arukellt Environment Diff',
    '',
    '| Surface | Local | CI/Profile assumption |',
    '|---|---|---|',
    `| arukellt binary | ${command} | ci uses PATH lookup unless overridden |`,
    `| extra args | ${extraArgs.join(' ') || '(none)'} | often empty in CI |`,
    `| target | ${config.get('target', 'wasm32-wasi-p1')} | may differ per task/profile |`,
    `| emit | ${config.get('emit', 'core-wasm')} | may differ per task/profile |`,
  ]
  outputChannel.clear()
  outputChannel.appendLine(lines.join('\n'))
  outputChannel.show()
}

function registerCommands(context) {
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.restartLanguageServer', () => restartLanguageServer(context)))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.checkCurrentFile', () => runCliCommand('check')))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.compileCurrentFile', () => runCliCommand('compile')))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.runCurrentFile', () => runCliCommand('run')))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.showSetupDoctor', showSetupDoctor))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.showCommandGraph', showCommandGraph))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.showEnvironmentDiff', showEnvironmentDiff))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.openDocs', (name) => {
    vscode.window.showInformationMessage(`Opening documentation for ${name}...`)
  }))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.explainCode', (name) => {
    vscode.window.showInformationMessage(`Explaining code for ${name}...`)
  }))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.showPipeline', showPipeline))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.securityReview', showSecurityReview))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.showOutput', () => {
    if (outputChannel) outputChannel.show()
  }))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.refreshProjectTree', () => {
    if (projectTreeProvider) projectTreeProvider.refresh()
  }))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.toggleVerboseLogging', () => {
    const config = getConfiguration()
    const current = config.get('trace.server', 'off')
    const next = current === 'verbose' ? 'off' : 'verbose'
    config.update('trace.server', next, vscode.ConfigurationTarget.Workspace)
    vscode.window.showInformationMessage(`Arukellt: server trace set to '${next}'`)
  }))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.runScript', (name) => {
    const { command } = resolveServerCommand()
    const terminal = vscode.window.createTerminal(`Arukellt: ${name}`)
    terminal.sendText(`${command} script run ${name}`)
    terminal.show()
  }))

  // Component commands (Issue #444)
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.buildComponent', buildComponent))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.buildComponentWit', buildComponentWit))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.runComponent', runComponent))
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.openInPlayground', openInPlayground))

  // CodeLens commands (Issue #458)
  // arukellt.runMain: launched from CodeLens on `fn main()`.
  // Receives the file URI string passed by the LSP server as the first argument.
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.runMain', (fileUri) => {
    const { command } = resolveServerCommand()
    const filePath = fileUri ? vscode.Uri.parse(fileUri).fsPath : (vscode.window.activeTextEditor ? vscode.window.activeTextEditor.document.uri.fsPath : '')
    if (!filePath) {
      vscode.window.showErrorMessage('Arukellt: could not determine file path for Run Main.')
      return
    }
    const terminal = vscode.window.createTerminal('Arukellt: Run Main')
    terminal.sendText(`${command} run "${filePath}"`)
    terminal.show()
  }))

  // arukellt.debugMain: placeholder — full DAP integration is tracked separately.
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.debugMain', (fileUri) => {
    const filePath = fileUri ? vscode.Uri.parse(fileUri).fsPath : (vscode.window.activeTextEditor ? vscode.window.activeTextEditor.document.uri.fsPath : '')
    vscode.debug.startDebugging(undefined, {
      type: 'arukellt',
      request: 'launch',
      name: 'Debug Main',
      program: filePath || '${file}',
    })
  }))

  // arukellt.runTest: launched from CodeLens on a test function.
  // Receives (fileUri, fnName) from the LSP server.
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.runTest', (fileUri, fnName) => {
    const { command } = resolveServerCommand()
    const filePath = fileUri ? vscode.Uri.parse(fileUri).fsPath : (vscode.window.activeTextEditor ? vscode.window.activeTextEditor.document.uri.fsPath : '')
    if (!filePath) {
      vscode.window.showErrorMessage('Arukellt: could not determine file path for Run Test.')
      return
    }
    const terminal = vscode.window.createTerminal(`Arukellt: Run Test${fnName ? ` (${fnName})` : ''}`)
    const filterArg = fnName ? ` --filter "${fnName}"` : ''
    terminal.sendText(`${command} test "${filePath}"${filterArg}`)
    terminal.show()
  }))

  // arukellt.debugTest: placeholder — full DAP integration is tracked separately.
  context.subscriptions.push(vscode.commands.registerCommand('arukellt.debugTest', (fileUri, fnName) => {
    const filePath = fileUri ? vscode.Uri.parse(fileUri).fsPath : (vscode.window.activeTextEditor ? vscode.window.activeTextEditor.document.uri.fsPath : '')
    vscode.debug.startDebugging(undefined, {
      type: 'arukellt',
      request: 'launch',
      name: `Debug Test${fnName ? `: ${fnName}` : ''}`,
      program: filePath || '${file}',
    })
  }))
}

function showSecurityReview() {
  const config = getConfiguration()
  const serverPath = config.get('server.path', 'arukellt')
  const risks = []

  if (serverPath !== 'arukellt' && !serverPath.startsWith('/')) {
    risks.push(`- **Custom server path**: \`${serverPath}\` is a relative path. This could be dangerous if a malicious binary is placed in the workspace root.`)
  }

  if (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0) {
    risks.push('- **Script execution**: Arukellt supports running scripts defined in `ark.toml`. Always review `ark.toml` before running any scripts.')
  }

  const lines = [
    '# Arukellt Extension Security Review',
    '',
    risks.length > 0 ? 'Potential risks identified:' : 'No significant risks identified in current settings.',
    '',
    ...risks,
    '',
    '## Security Best Practices',
    '1. Use absolute paths for `arukellt.server.path`.',
    '2. Review `ark.toml` files in untrusted repositories.',
    '3. Do not pass sensitive information via environment variables used by `arukellt`.',
  ]

  outputChannel.clear()
  outputChannel.appendLine(lines.join('\n'))
  outputChannel.show()
}

async function showPipeline() {
  const editor = vscode.window.activeTextEditor
  if (!editor) return
  const file = editor.document.uri.fsPath
  const { command } = resolveServerCommand()
  const args = ['compile', file, '--time', '--json']

  outputChannel.appendLine(`Analyzing pipeline for ${file}...`)
  outputChannel.show()

  try {
    const stdout = cp.execSync(`${command} ${args.join(' ')}`, { encoding: 'utf8' })
    const result = JSON.parse(stdout)
    if (result.status === 'success' && result.timing) {
      const t = result.timing
      const lines = [
        `# Compiler Pipeline: ${file}`,
        '',
        `| Phase | Duration (ms) |`,
        '|---|---|',
        `| Lex | ${t.lex_ms.toFixed(2)} |`,
        `| Parse | ${t.parse_ms.toFixed(2)} |`,
        `| Resolve | ${t.resolve_ms.toFixed(2)} |`,
        `| Typecheck | ${t.typecheck_ms.toFixed(2)} |`,
        `| Lower | ${t.lower_ms.toFixed(2)} |`,
        `| Optimize | ${t.opt_ms.toFixed(2)} |`,
        `| Emit | ${t.emit_ms.toFixed(2)} |`,
        `| **Total** | **${t.total_ms.toFixed(2)}** |`,
        '',
        `Optimization detail: ${t.opt_detail || 'none'}`,
        `Wasm size: ${result.wasm_size} bytes`,
      ]
      outputChannel.clear()
      outputChannel.appendLine(lines.join('\n'))
    } else {
      outputChannel.appendLine(`Pipeline analysis failed: ${stdout}`)
    }
  } catch (err) {
    outputChannel.appendLine(`Pipeline analysis failed: ${err.message}`)
  }
}

function registerTaskProvider(context) {
  const provider = vscode.tasks.registerTaskProvider('arukellt', {
    provideTasks() {
      const folders = vscode.workspace.workspaceFolders || []
      const { command } = resolveServerCommand()
      const tasks = []

      const definitions = [
        { type: 'arukellt', task: 'check', command: 'check', group: vscode.TaskGroup.Build },
        { type: 'arukellt', task: 'compile', command: 'compile', group: vscode.TaskGroup.Build },
        { type: 'arukellt', task: 'run', command: 'run', group: undefined },
        { type: 'arukellt', task: 'test', command: 'test', group: vscode.TaskGroup.Test },
        { type: 'arukellt', task: 'fmt', command: 'fmt', group: undefined },
        { type: 'arukellt', task: 'fmt-check', command: 'fmt --check', group: undefined },
      ]

      // Generate tasks per workspace folder for multi-root support
      const scopes = folders.length > 0 ? folders : [vscode.TaskScope.Workspace]
      for (const scope of scopes) {
        const folderConfig = scope.uri
          ? vscode.workspace.getConfiguration('arukellt', scope.uri)
          : getConfiguration()
        const target = folderConfig.get('target', 'wasm32-wasi-p1')
        const prefix = folders.length > 1 && scope.name ? `${scope.name}: ` : ''

        for (const def of definitions) {
          const fullCmd = def.command.includes('--target')
            ? `${command} ${def.command}`
            : `${command} ${def.command}`
          const shellExec = new vscode.ShellExecution(fullCmd, {
            cwd: scope.uri ? scope.uri.fsPath : undefined,
          })
          const t = new vscode.Task(def, scope, `${prefix}arukellt:${def.task}`, 'arukellt', shellExec)
          if (def.group) t.group = def.group
          t.problemMatchers = ['$arukellt']
          tasks.push(t)
        }

        // Background watch task
        const watchDef = { type: 'arukellt', task: 'watch', command: 'check --watch', isBackground: true }
        const watchExec = new vscode.ShellExecution(`${command} check --watch`, {
          cwd: scope.uri ? scope.uri.fsPath : undefined,
        })
        const watchTask = new vscode.Task(watchDef, scope, `${prefix}arukellt:watch`, 'arukellt', watchExec)
        watchTask.isBackground = true
        watchTask.problemMatchers = ['$arukellt-watch']
        tasks.push(watchTask)
      }

      return tasks
    },
    resolveTask(task) {
      // Pre-execution validation
      const { command, probe } = discoverBinary(
        getConfiguration().get('server.path', 'arukellt')
      )
      if (!probe.ok) {
        vscode.window.showErrorMessage(
          'Arukellt: binary not found. Run "Arukellt: Setup Doctor" for details.',
          'Run Doctor'
        ).then(sel => {
          if (sel === 'Run Doctor') vscode.commands.executeCommand('arukellt.showSetupDoctor')
        })
        return undefined
      }

      // Validate ark.toml for the task's scope folder
      const folder = task.scope && task.scope.uri
        ? task.scope
        : (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders[0])
      if (folder && folder.uri) {
        const tomlPath = path.join(folder.uri.fsPath, 'ark.toml')
        try {
          if (fs.existsSync(tomlPath)) {
            fs.readFileSync(tomlPath, 'utf8')
          }
        } catch (e) {
          vscode.window.showWarningMessage(`Arukellt: failed to read ark.toml in ${folder.name}: ${e.message}`)
        }
      }

      return task
    },
  })
  context.subscriptions.push(provider)
}

/** Detect which workspace folders have ark.toml and return project info. */
function detectProjects() {
  const folders = vscode.workspace.workspaceFolders || []
  const projects = []
  for (const folder of folders) {
    const tomlPath = path.join(folder.uri.fsPath, 'ark.toml')
    const hasManifest = fs.existsSync(tomlPath)
    const config = vscode.workspace.getConfiguration('arukellt', folder.uri)
    projects.push({
      folder,
      hasManifest,
      target: config.get('target', 'wasm32-wasi-p1'),
      emit: config.get('emit', 'core-wasm'),
    })
  }
  return projects
}

function setupTestController(context) {
  testController = vscode.tests.createTestController('arukellt-tests', 'Arukellt Tests')
  context.subscriptions.push(testController)

  testController.resolveHandler = async (item) => {
    if (!item) {
      await discoverAllTests()
    } else {
      await discoverTestsInFile(item)
    }
  }

  const runHandler = async (request, token) => {
    const run = testController.createTestRun(request)
    const queue = []

    if (request.include) {
      request.include.forEach(test => queue.push(test))
    } else {
      testController.items.forEach(test => queue.push(test))
    }

    while (queue.length > 0 && !token.isCancellationRequested) {
      const test = queue.pop()
      if (request.exclude?.includes(test)) {
        continue
      }

      if (test.children.size > 0) {
        test.children.forEach(child => queue.push(child))
        continue
      }

      await runSingleTest(run, test, token)
    }

    run.end()
  }

  testController.createRunProfile('Run', vscode.TestRunProfileKind.Run, runHandler, true)
}

async function discoverAllTests() {
  const files = await vscode.workspace.findFiles('**/*.ark')
  for (const file of files) {
    const fileName = vscode.workspace.asRelativePath(file)
    const item = testController.createTestItem(file.toString(), fileName, file)
    item.canResolveChildren = true
    testController.items.add(item)
  }
}

async function discoverTestsInFile(item) {
  if (!item.uri) return
  const { command } = resolveServerCommand()
  const args = ['test', item.uri.fsPath, '--list', '--json']

  try {
    const stdout = cp.execSync(`${command} ${args.join(' ')}`, { encoding: 'utf8' })
    const testNames = JSON.parse(stdout)
    item.children.replace([])
    for (const name of testNames) {
      const testItem = testController.createTestItem(`${item.id}:${name}`, name, item.uri)
      item.children.add(testItem)
    }
  } catch (err) {
    outputChannel.appendLine(`Test discovery failed for ${item.uri.fsPath}: ${err.message}`)
  }
}

async function runSingleTest(run, test, token) {
  run.started(test)
  const startTime = Date.now()
  const fileUri = test.uri
  if (!fileUri) {
    run.errored(test, new vscode.TestMessage('Missing URI'))
    return
  }

  const { command } = resolveServerCommand()
  const testName = test.label
  const args = ['test', fileUri.fsPath, '--json'] // Currently runs all tests in file, we might need to filter

  return new Promise((resolve) => {
    const child = cp.spawn(command, args)
    let stdout = ''
    child.stdout.on('data', data => stdout += data.toString())
    child.on('close', (code) => {
      const duration = Date.now() - startTime
      try {
        const result = JSON.parse(stdout)
        const testResult = result.tests.find(t => t.name === testName)
        if (testResult) {
          if (testResult.status === 'pass') {
            run.passed(test, duration)
          } else {
            run.failed(test, new vscode.TestMessage(testResult.message || 'Test failed'), duration)
          }
        } else {
          run.errored(test, new vscode.TestMessage('Test result not found in output'))
        }
      } catch (err) {
        run.errored(test, new vscode.TestMessage(`Failed to parse test output: ${stdout}`))
      }
      resolve()
    })

    token.onCancellationRequested(() => {
      child.kill()
      resolve()
    })
  })
}

function verifyBootstrap() {
  const config = getConfiguration()
  const configuredPath = config.get('server.path', 'arukellt')
  const { extraArgs } = resolveServerCommand()
  const { command, probe } = discoverBinary(configuredPath)
  return {
    command,
    extraArgs,
    probe,
  }
}

function registerDebugAdapter(context) {
  const factory = vscode.debug.registerDebugAdapterDescriptorFactory('arukellt', {
    createDebugAdapterDescriptor(_session) {
      const { command } = resolveServerCommand()
      return new vscode.DebugAdapterExecutable(command, ['debug-adapter'])
    },
  })
  const trackerFactory = vscode.debug.registerDebugAdapterTrackerFactory('arukellt', {
    createDebugAdapterTracker(session) {
      debugSessionState.activeSessionName = session.name
      debugSessionState.activeSessionType = session.type
      debugSessionState.lastStoppedEvent = null
      return {
        onDidSendMessage(message) {
          if (message && message.type === 'event' && message.event === 'stopped') {
            debugSessionState.lastStoppedEvent = message.body || null
            debugSessionState.stoppedEventCount++
          }
        },
      }
    },
  })
  const onDidStartSession = vscode.debug.onDidStartDebugSession(session => {
    if (session.type !== 'arukellt') return
    debugSessionState.startCount++
    debugSessionState.activeSessionName = session.name
    debugSessionState.activeSessionType = session.type
  })
  const onDidTerminateSession = vscode.debug.onDidTerminateDebugSession(session => {
    if (session.type !== 'arukellt') return
    debugSessionState.terminateCount++
    if (vscode.debug.activeDebugSession?.id !== session.id) {
      debugSessionState.activeSessionName = null
      debugSessionState.activeSessionType = null
    }
  })
  context.subscriptions.push(factory)
  context.subscriptions.push(trackerFactory)
  context.subscriptions.push(onDidStartSession)
  context.subscriptions.push(onDidTerminateSession)
}

// --- Language Status Item (#213) ---

function setupLanguageStatus(context) {
  languageStatusItem = vscode.languages.createLanguageStatusItem('arukellt-server', { language: 'arukellt' })
  languageStatusItem.name = 'Arukellt'
  languageStatusItem.text = '$(loading~spin) Starting…'
  languageStatusItem.severity = vscode.LanguageStatusSeverity.Information
  languageStatusItem.command = { title: 'Show Output', command: 'arukellt.showOutput' }
  context.subscriptions.push(languageStatusItem)
}

function updateLanguageStatus(state, detail) {
  if (!languageStatusItem) return
  switch (state) {
    case 'starting':
      languageStatusItem.text = '$(loading~spin) Starting…'
      languageStatusItem.severity = vscode.LanguageStatusSeverity.Information
      languageStatusItem.detail = detail || 'Language server is starting'
      break
    case 'ready':
      languageStatusItem.text = '$(check) Ready'
      languageStatusItem.severity = vscode.LanguageStatusSeverity.Information
      languageStatusItem.detail = detail || 'Language server is running'
      break
    case 'error':
      languageStatusItem.text = '$(error) Error'
      languageStatusItem.severity = vscode.LanguageStatusSeverity.Error
      languageStatusItem.detail = detail || 'Language server encountered an error'
      break
    case 'indexing':
      languageStatusItem.text = '$(sync~spin) Indexing…'
      languageStatusItem.severity = vscode.LanguageStatusSeverity.Information
      languageStatusItem.detail = detail || 'Indexing workspace'
      break
  }
}

// --- Project Tree View (#212) ---

class ProjectTreeProvider {
  constructor() {
    this._onDidChangeTreeData = new vscode.EventEmitter()
    this.onDidChangeTreeData = this._onDidChangeTreeData.event
    this._modules = []
    this._scripts = []
    this._targets = []
  }

  refresh() {
    this._loadProjectData()
    this._onDidChangeTreeData.fire()
  }

  _loadProjectData() {
    this._modules = []
    this._scripts = []
    this._targets = ['wasm32-wasi-p1', 'wasm32-wasi-p2']
    this._projects = []

    const folders = vscode.workspace.workspaceFolders
    if (!folders) return

    // Multi-root: detect projects in each folder
    for (const folder of folders) {
      const rootPath = folder.uri.fsPath
      const manifestPath = path.join(rootPath, 'ark.toml')
      const hasManifest = fs.existsSync(manifestPath)
      const prefix = folders.length > 1 ? `${folder.name}/` : ''

      if (hasManifest) {
        this._projects.push({ name: folder.name, hasManifest: true })
        try {
          const content = fs.readFileSync(manifestPath, 'utf8')
          const scriptMatch = content.match(/\[scripts\]([\s\S]*?)(?=\n\[|$)/m)
          if (scriptMatch) {
            const lines = scriptMatch[1].split('\n')
            for (const line of lines) {
              const m = line.match(/^(\w+)\s*=/)
              if (m) this._scripts.push(prefix + m[1])
            }
          }
        } catch (_) { /* ignore */ }
      }

      // Discover .ark source files
      const srcDir = path.join(rootPath, 'src')
      if (fs.existsSync(srcDir)) {
        this._scanDir(srcDir, this._modules, rootPath, prefix)
      }
      // Also scan root-level .ark files (single-file mode)
      if (!hasManifest) {
        try {
          const entries = fs.readdirSync(rootPath, { withFileTypes: true })
          for (const entry of entries) {
            if (entry.isFile() && entry.name.endsWith('.ark')) {
              this._modules.push({
                name: prefix + entry.name,
                uri: vscode.Uri.file(path.join(rootPath, entry.name)),
              })
            }
          }
        } catch (_) { /* ignore */ }
      }
    }
  }

  _scanDir(dir, modules, rootPath, prefix) {
    try {
      const entries = fs.readdirSync(dir, { withFileTypes: true })
      for (const entry of entries) {
        const fullPath = path.join(dir, entry.name)
        if (entry.isDirectory()) {
          this._scanDir(fullPath, modules, rootPath, prefix)
        } else if (entry.name.endsWith('.ark')) {
          modules.push({
            name: prefix + path.relative(rootPath, fullPath),
            uri: vscode.Uri.file(fullPath),
          })
        }
      }
    } catch (_) { /* ignore */ }
  }

  getTreeItem(element) {
    return element
  }

  getChildren(element) {
    if (!element) {
      // Root — return category nodes
      const items = []
      if (this._modules.length > 0) {
        const modulesItem = new vscode.TreeItem('Modules', vscode.TreeItemCollapsibleState.Expanded)
        modulesItem.iconPath = new vscode.ThemeIcon('symbol-module')
        modulesItem.contextValue = 'category-modules'
        items.push(modulesItem)
      }
      if (this._scripts.length > 0) {
        const scriptsItem = new vscode.TreeItem('Scripts', vscode.TreeItemCollapsibleState.Collapsed)
        scriptsItem.iconPath = new vscode.ThemeIcon('terminal')
        scriptsItem.contextValue = 'category-scripts'
        items.push(scriptsItem)
      }
      const targetsItem = new vscode.TreeItem('Targets', vscode.TreeItemCollapsibleState.Collapsed)
      targetsItem.iconPath = new vscode.ThemeIcon('package')
      targetsItem.contextValue = 'category-targets'
      items.push(targetsItem)
      return items
    }

    if (element.contextValue === 'category-modules') {
      return this._modules.map(m => {
        const item = new vscode.TreeItem(m.name, vscode.TreeItemCollapsibleState.None)
        item.iconPath = new vscode.ThemeIcon('file-code')
        item.resourceUri = m.uri
        item.command = { command: 'vscode.open', arguments: [m.uri], title: 'Open' }
        item.contextValue = 'module'
        return item
      })
    }

    if (element.contextValue === 'category-scripts') {
      return this._scripts.map(name => {
        const item = new vscode.TreeItem(name, vscode.TreeItemCollapsibleState.None)
        item.iconPath = new vscode.ThemeIcon('play')
        item.contextValue = 'script'
        item.command = { command: 'arukellt.runScript', arguments: [name], title: `Run ${name}` }
        return item
      })
    }

    if (element.contextValue === 'category-targets') {
      return this._targets.map(name => {
        const item = new vscode.TreeItem(name, vscode.TreeItemCollapsibleState.None)
        item.iconPath = new vscode.ThemeIcon('circuit-board')
        item.contextValue = 'target'
        return item
      })
    }

    return []
  }
}

function setupProjectTreeView(context) {
  projectTreeProvider = new ProjectTreeProvider()
  projectTreeProvider.refresh()
  const treeView = vscode.window.createTreeView('arukellt-project', {
    treeDataProvider: projectTreeProvider,
    showCollapseAll: true,
  })
  context.subscriptions.push(treeView)

  // Refresh when files change
  const watcher = vscode.workspace.createFileSystemWatcher('**/*.ark')
  watcher.onDidCreate(() => projectTreeProvider.refresh())
  watcher.onDidDelete(() => projectTreeProvider.refresh())
  context.subscriptions.push(watcher)

  const manifestWatcher = vscode.workspace.createFileSystemWatcher('**/ark.toml')
  manifestWatcher.onDidChange(() => projectTreeProvider.refresh())
  manifestWatcher.onDidCreate(() => projectTreeProvider.refresh())
  manifestWatcher.onDidDelete(() => projectTreeProvider.refresh())
  context.subscriptions.push(manifestWatcher)

  // Re-detect on workspace folder add/remove (multi-root)
  context.subscriptions.push(
    vscode.workspace.onDidChangeWorkspaceFolders(() => {
      projectTreeProvider.refresh()
    })
  )
}

function getTestState() {
  return {
    hasClient: !!client,
    clientSessionId,
    restartCount,
    lastErrorMessage: testTelemetry.lastErrorMessage,
    recordedErrorNotificationCount: testTelemetry.recordedErrorNotificationCount,
    outputChannelLines: [...testTelemetry.outputChannelLines],
    statusBarText: statusBarItem ? statusBarItem.text : null,
    statusBarTooltip: statusBarItem ? statusBarItem.tooltip : null,
    languageStatusText: languageStatusItem ? languageStatusItem.text : null,
    languageStatusDetail: languageStatusItem ? languageStatusItem.detail : null,
    languageStatusSeverity: languageStatusItem ? languageStatusItem.severity : null,
    debugSessionStartCount: debugSessionState.startCount,
    debugSessionTerminateCount: debugSessionState.terminateCount,
    activeDebugSessionName: debugSessionState.activeSessionName,
    activeDebugSessionType: debugSessionState.activeSessionType,
    lastStoppedEvent: debugSessionState.lastStoppedEvent,
    debugStoppedEventCount: debugSessionState.stoppedEventCount,
  }
}

module.exports = {
  activate,
  deactivate,
  __getTestState: getTestState,
  verifyBootstrap,
}
