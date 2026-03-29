let vscode
try {
  vscode = require('vscode')
} catch (_error) {
  vscode = null
}
const cp = require('child_process')

function hasVscodeHost() {
  return vscode !== null
}

function fallbackConfig() {
  return {
    get(key, defaultValue) {
      const envKey = key === 'server.path' ? 'ARUKELLT_SERVER_PATH' : 'ARUKELLT_SERVER_ARGS'
      const value = process.env[envKey]
      if (value === undefined || value === '') {
        return defaultValue
      }
      if (key === 'server.args') {
        return value.split(' ').filter(Boolean)
      }
      return value
    },
  }
}

function notifyError(message) {
  if (hasVscodeHost()) {
    vscode.window.showErrorMessage(message)
  } else if (outputChannel) {
    outputChannel.appendLine(message)
  }
}

function notifyInfo(message) {
  if (hasVscodeHost()) {
    vscode.window.showInformationMessage(message)
  } else if (outputChannel) {
    outputChannel.appendLine(message)
  }
}

function createOutputChannel() {
  if (hasVscodeHost()) {
    return vscode.window.createOutputChannel('Arukellt')
  }
  return {
    append(value) { process.stderr.write(value) },
    appendLine(value) { process.stderr.write(`${value}\n`) },
    clear() {},
    show() {},
    dispose() {},
  }
}

function createStatusBar() {
  if (hasVscodeHost()) {
    const item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100)
    item.name = 'Arukellt'
    return item
  }
  return {
    text: '',
    tooltip: '',
    show() {},
    hide() {},
    dispose() {},
  }
}

function pushDisposable(context, disposable) {
  if (context && context.subscriptions) {
    context.subscriptions.push(disposable)
  }
}

function registerRestartCommand(context) {
  if (!hasVscodeHost()) {
    return
  }
  pushDisposable(context, vscode.commands.registerCommand('arukellt.restartLanguageServer', () => {
    restartServer(context)
  }))
}

function getConfiguration() {
  if (hasVscodeHost()) {
    return vscode.workspace.getConfiguration('arukellt')
  }
  return fallbackConfig()
}

function createContext(context) {
  return context || { subscriptions: [] }
}

function verifyBootstrap() {
  const { command, extraArgs } = resolveServerCommand()
  return {
    command,
    extraArgs,
    probe: probeServerBinary(command),
  }
}

let currentClient = null
let outputChannel = null
let statusBarItem = null

function resolveServerCommand() {
  const config = getConfiguration()
  const command = config.get('server.path', 'arukellt')
  const extraArgs = config.get('server.args', [])
  return { command, extraArgs }
}

function probeServerBinary(command) {
  try {
    const result = cp.spawnSync(command, ['--version'], { encoding: 'utf8' })
    if (result.error) {
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

function updateStatus(text, tooltip) {
  if (!statusBarItem) {
    return
  }
  statusBarItem.text = text
  statusBarItem.tooltip = tooltip || text
  statusBarItem.show()
}

function commandArgsForCurrentFile(kind) {
  if (!hasVscodeHost()) {
    return null
  }
  const editor = vscode.window.activeTextEditor
  if (!editor) {
    notifyError('Arukellt: no active editor.')
    return null
  }
  const file = editor.document.uri.fsPath
  if (!file.endsWith('.ark')) {
    notifyError('Arukellt: current file is not an .ark source file.')
    return null
  }
  const config = getConfiguration()
  const target = config.get('target', 'wasm32-wasi-p1')
  const emit = config.get('emit', 'core-wasm')
  if (kind === 'check') {
    return ['check', file, '--target', target]
  }
  if (kind === 'compile') {
    return ['compile', file, '--target', target, '--emit', emit]
  }
  return ['run', file, '--target', target]
}

function runCliCommand(kind) {
  const args = commandArgsForCurrentFile(kind)
  if (!args) {
    return
  }
  const { command } = resolveServerCommand()
  if (outputChannel) {
    outputChannel.appendLine(`$ ${command} ${args.join(' ')}`)
    outputChannel.show()
  }
  updateStatus(`Arukellt: ${kind}`, `Running ${kind} for current .ark file`)
  const child = cp.spawn(command, args, { stdio: ['ignore', 'pipe', 'pipe'] })
  child.stdout.on('data', (chunk) => {
    if (outputChannel) {
      outputChannel.append(chunk.toString())
    }
  })
  child.stderr.on('data', (chunk) => {
    if (outputChannel) {
      outputChannel.append(chunk.toString())
    }
  })
  child.on('close', (code) => {
    updateStatus('Arukellt: idle', `Last ${kind} exit code: ${code}`)
    if (outputChannel) {
      outputChannel.appendLine(`Arukellt ${kind} exited with code ${code}`)
    }
  })
}

function environmentSummary() {
  const config = getConfiguration()
  const { command, extraArgs } = resolveServerCommand()
  const probe = probeServerBinary(command)
  return {
    command,
    extraArgs,
    probe,
    target: config.get('target', 'wasm32-wasi-p1'),
    emit: config.get('emit', 'core-wasm'),
    workspaceFolders: hasVscodeHost() && vscode.workspace.workspaceFolders
      ? vscode.workspace.workspaceFolders.map((folder) => folder.uri.fsPath)
      : [],
  }
}

function showSetupDoctor() {
  const summary = environmentSummary()
  const lines = [
    '# Arukellt Setup Doctor',
    '',
    `- CLI command: ${summary.command}`,
    `- CLI args before lsp: ${summary.extraArgs.join(' ') || '(none)'}`,
    `- CLI probe: ${summary.probe.ok ? `ok (${summary.probe.version || 'version unknown'})` : `failed (${summary.probe.message})`}`,
    `- Default target: ${summary.target}`,
    `- Default emit: ${summary.emit}`,
    `- Workspace folders: ${summary.workspaceFolders.length > 0 ? summary.workspaceFolders.join(', ') : '(none)'}`,
  ]
  if (outputChannel) {
    outputChannel.clear()
    outputChannel.appendLine(lines.join('\n'))
    outputChannel.show()
  }
  notifyInfo('Arukellt setup doctor written to the output channel.')
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
  if (outputChannel) {
    outputChannel.clear()
    outputChannel.appendLine(lines.join('\n'))
    outputChannel.show()
  }
  updateStatus('Arukellt: graph ready', 'Command graph written to output channel')
}

function showEnvironmentDiff() {
  const summary = environmentSummary()
  const lines = [
    '# Arukellt Environment Diff',
    '',
    '| Surface | Local | CI/Profile assumption |',
    '|---|---|---|',
    `| arukellt binary | ${summary.command} | ci uses PATH lookup unless overridden |`,
    `| extra args | ${summary.extraArgs.join(' ') || '(none)'} | often empty in CI |`,
    `| target | ${summary.target} | may differ per task/profile |`,
    `| emit | ${summary.emit} | may differ per task/profile |`,
    `| workspace folders | ${summary.workspaceFolders.length > 0 ? summary.workspaceFolders.join(', ') : '(none)'} | usually repository root only |`,
  ]
  if (outputChannel) {
    outputChannel.clear()
    outputChannel.appendLine(lines.join('\n'))
    outputChannel.show()
  }
  updateStatus('Arukellt: env diff ready', 'Environment diff written to output channel')
}

function registerCommandSurfaces(context) {
  if (!hasVscodeHost()) {
    return
  }
  pushDisposable(context, vscode.commands.registerCommand('arukellt.checkCurrentFile', () => runCliCommand('check')))
  pushDisposable(context, vscode.commands.registerCommand('arukellt.compileCurrentFile', () => runCliCommand('compile')))
  pushDisposable(context, vscode.commands.registerCommand('arukellt.runCurrentFile', () => runCliCommand('run')))
  pushDisposable(context, vscode.commands.registerCommand('arukellt.showSetupDoctor', showSetupDoctor))
  pushDisposable(context, vscode.commands.registerCommand('arukellt.showCommandGraph', showCommandGraph))
  pushDisposable(context, vscode.commands.registerCommand('arukellt.showEnvironmentDiff', showEnvironmentDiff))
}

function registerTaskProvider(context) {
  if (!hasVscodeHost()) {
    return
  }
  const provider = vscode.tasks.registerTaskProvider('arukellt', {
    provideTasks() {
      const workspaceFolder = vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders[0]
      const scope = workspaceFolder || vscode.TaskScope.Workspace
      const definitions = [
        { task: 'check', command: 'check' },
        { task: 'compile', command: 'compile' },
        { task: 'run', command: 'run' },
      ]
      return definitions.map((definition) => {
        const shellExecution = new vscode.ShellExecution(`arukellt ${definition.command}`)
        return new vscode.Task(definition, scope, `arukellt:${definition.task}`, 'arukellt', shellExecution)
      })
    },
    resolveTask(task) {
      return task
    },
  })
  pushDisposable(context, provider)
}

function startServer(context) {
  const { command, extraArgs } = resolveServerCommand()
  const probe = probeServerBinary(command)
  if (!probe.ok) {
    updateStatus('Arukellt: server missing', 'Failed to probe arukellt server binary')
    notifyError(`Arukellt: failed to start language server (${probe.message}). Set arukellt.server.path if needed.`)
    if (outputChannel) {
      outputChannel.appendLine(`Arukellt server probe failed: ${probe.message}`)
    }
    return
  }

  updateStatus('Arukellt: LSP running', probe.version || command)
  if (outputChannel) {
    outputChannel.appendLine(`Using ${command} (${probe.version || 'version unknown'})`)
  }

  const args = [...extraArgs, 'lsp']
  const child = cp.spawn(command, args, { stdio: ['pipe', 'pipe', 'pipe'] })

  child.on('error', (error) => {
    notifyError(`Arukellt language server failed: ${error.message}`)
    if (outputChannel) {
      outputChannel.appendLine(`Server error: ${error.message}`)
    }
  })

  child.stderr.on('data', (chunk) => {
    if (outputChannel) {
      outputChannel.append(chunk.toString())
    }
  })

  currentClient = child
  pushDisposable(context, {
    dispose() {
      if (currentClient && !currentClient.killed) {
        currentClient.kill()
      }
      currentClient = null
    }
  })
}

function restartServer(context) {
  if (currentClient && !currentClient.killed) {
    currentClient.kill()
  }
  currentClient = null
  startServer(context)
  notifyInfo('Arukellt language server restarted.')
}

function activate(context) {
  const realContext = createContext(context)
  outputChannel = createOutputChannel()
  statusBarItem = createStatusBar()
  pushDisposable(realContext, outputChannel)
  pushDisposable(realContext, statusBarItem)
  registerRestartCommand(realContext)
  registerCommandSurfaces(realContext)
  registerTaskProvider(realContext)
  updateStatus('Arukellt: starting', 'Starting language server')
  startServer(realContext)
  return realContext
}

function deactivate() {
  if (currentClient && !currentClient.killed) {
    currentClient.kill()
  }
  currentClient = null
  if (statusBarItem) {
    statusBarItem.dispose()
  }
  statusBarItem = null
}

module.exports = {
  activate,
  deactivate,
  verifyBootstrap,
  resolveServerCommand,
  probeServerBinary,
}
