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

function startServer(context) {
  const { command, extraArgs } = resolveServerCommand()
  const probe = probeServerBinary(command)
  if (!probe.ok) {
    notifyError(`Arukellt: failed to start language server (${probe.message}). Set arukellt.server.path if needed.`)
    if (outputChannel) {
      outputChannel.appendLine(`Arukellt server probe failed: ${probe.message}`)
    }
    return
  }

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
  pushDisposable(realContext, outputChannel)
  registerRestartCommand(realContext)
  startServer(realContext)
  return realContext
}

function deactivate() {
  if (currentClient && !currentClient.killed) {
    currentClient.kill()
  }
  currentClient = null
}

module.exports = {
  activate,
  deactivate,
  verifyBootstrap,
  resolveServerCommand,
  probeServerBinary,
}
