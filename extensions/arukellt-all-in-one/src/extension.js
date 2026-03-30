const vscode = require('vscode')
const cp = require('child_process')
const { LanguageClient, TransportKind } = require('vscode-languageclient/node')

let client = null
let outputChannel = null
let statusBarItem = null
let testController = null

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
  outputChannel = vscode.window.createOutputChannel('Arukellt')
  statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100)
  statusBarItem.name = 'Arukellt'

  context.subscriptions.push(outputChannel)
  context.subscriptions.push(statusBarItem)

  registerCommands(context)
  registerTaskProvider(context)
  setupTestController(context)

  startLanguageServer(context)
}

function deactivate() {
  if (!client) {
    return undefined
  }
  return client.stop()
}

function getConfiguration() {
  return vscode.workspace.getConfiguration('arukellt')
}

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

function startLanguageServer(context) {
  const { command, extraArgs } = resolveServerCommand()
  const probe = probeServerBinary(command)

  if (!probe.ok) {
    updateStatus('$(error) Arukellt: binary missing', 'Failed to probe arukellt server binary')
    vscode.window.showErrorMessage(`Arukellt: failed to start language server (${probe.message}). Set arukellt.server.path if needed.`)
    return
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
    },
    outputChannel,
  }

  client = new LanguageClient('arukellt', 'Arukellt Language Server', serverOptions, clientOptions)

  client.start().then(() => {
    updateStatus('Arukellt: $(check) LSP running', probe.version || command)
  }).catch((err) => {
    updateStatus('$(error) Arukellt: start failed', err.message)
    vscode.window.showErrorMessage(`Arukellt: failed to start language server: ${err.message}`)
  })
}

function restartLanguageServer(context) {
  if (client) {
    client.stop().then(() => {
      client = null
      startLanguageServer(context)
      vscode.window.showInformationMessage('Arukellt language server restarted.')
    })
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

  const config = getConfiguration()
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

  outputChannel.appendLine(`$ ${command} ${args.join(' ')}`)
  outputChannel.show()

  const child = cp.spawn(command, args)
  child.stdout.on('data', (data) => outputChannel.append(data.toString()))
  child.stderr.on('data', (data) => outputChannel.append(data.toString()))
  child.on('close', (code) => {
    outputChannel.appendLine(`Arukellt ${kind} exited with code ${code}`)
  })
}

function showSetupDoctor() {
  const config = getConfiguration()
  const { command, extraArgs } = resolveServerCommand()
  const probe = probeServerBinary(command)

  const lines = [
    '# Arukellt Setup Doctor',
    '',
    `- CLI command: ${command}`,
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
  context.subscriptions.push(provider)
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
  const { command, extraArgs } = resolveServerCommand()
  return {
    command,
    extraArgs,
    probe: probeServerBinary(command),
  }
}

module.exports = {
  activate,
  deactivate,
  verifyBootstrap,
}
