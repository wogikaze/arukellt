const vscode = require('vscode')
const path = require('path')
const fs = require('fs')

const COMMAND_GRAPH_WORKFLOW = [
  {
    id: 'check',
    label: 'Check',
    detail: 'Type-check the active .ark file',
    command: 'arukellt.checkCurrentFile',
    icon: 'check',
    flow: 'check → compile → run',
  },
  {
    id: 'compile',
    label: 'Compile',
    detail: 'Emit Wasm for the active .ark file',
    command: 'arukellt.compileCurrentFile',
    icon: 'gear',
    flow: 'check → compile → run',
  },
  {
    id: 'run',
    label: 'Run',
    detail: 'Execute the active .ark file',
    command: 'arukellt.runCurrentFile',
    icon: 'run',
    flow: 'check → compile → run',
  },
  {
    id: 'test',
    label: 'Test',
    detail: 'Run the workspace arukellt:test task',
    command: 'arukellt.runWorkspaceTests',
    icon: 'beaker',
    flow: 'check → test',
  },
]

const COMMAND_GRAPH_OPS = [
  {
    id: 'restart-lsp',
    label: 'Restart Language Server',
    detail: 'Reload the LSP after changing server.path or args',
    command: 'arukellt.restartLanguageServer',
    icon: 'refresh',
    category: 'ops',
  },
  {
    id: 'doctor',
    label: 'Setup Doctor',
    detail: 'Inspect binary discovery, workspace, and dependency health',
    command: 'arukellt.showSetupDoctor',
    icon: 'heart',
    category: 'ops',
  },
  {
    id: 'env-diff',
    label: 'Environment Diff',
    detail: 'Compare local, CI, and profile assumptions',
    command: 'arukellt.showEnvironmentDiff',
    icon: 'diff',
    category: 'ops',
  },
]

const CI_PROFILE = {
  label: 'CI (.github/workflows/ci.yml)',
  binary: 'PATH lookup (arukellt)',
  extraArgs: '(none)',
  target: 'auto from ark.toml',
  emit: 'core-wasm',
  selfhostWasm: 'bootstrap/arukellt-selfhost.wasm (required)',
  traceServer: 'off',
}

function createOpsSurfaces(deps) {
  const {
    discoverBinary,
    resolveServerCommand,
    getConfiguration,
    detectProjects,
    getLanguageServerState,
    appendLanguageServerOutputLine,
  } = deps

  let commandGraphProvider = null

  function collectSetupDoctorReport() {
    const config = getConfiguration()
    const configuredPath = config.get('server.path', 'arukellt')
    const { command: configuredCommand, extraArgs } = resolveServerCommand()
    const { command, probe } = discoverBinary(configuredPath)
    const folders = vscode.workspace.workspaceFolders || []
    const projects = detectProjects()
    const lsp = getLanguageServerState()

    const checks = [
      {
        id: 'binary',
        label: 'Arukellt binary',
        status: probe.ok ? 'pass' : 'fail',
        detail: probe.ok
          ? `${command} (${probe.version || 'version unknown'})`
          : probe.message,
        action: probe.ok ? undefined : 'openServerPathSettings',
      },
      {
        id: 'configured-path',
        label: 'Configured server.path',
        status: configuredPath === command || probe.ok ? 'pass' : 'warn',
        detail: `${configuredCommand}${configuredPath !== command ? ` → resolved ${command}` : ''}`,
        action: 'openServerPathSettings',
      },
      {
        id: 'server-args',
        label: 'server.args before lsp',
        status: 'pass',
        detail: extraArgs.length > 0 ? extraArgs.join(' ') : '(none)',
      },
      {
        id: 'lsp',
        label: 'Language server',
        status: lsp.ready ? 'pass' : lsp.starting ? 'warn' : 'fail',
        detail: lsp.detail,
        action: lsp.ready ? 'showOutput' : 'restartLanguageServer',
      },
      {
        id: 'workspace',
        label: 'Workspace folders',
        status: folders.length > 0 ? 'pass' : 'warn',
        detail: folders.length > 0
          ? folders.map(folder => folder.uri.fsPath).join(', ')
          : 'No workspace folder open',
      },
      {
        id: 'manifest',
        label: 'Project manifests',
        status: projects.some(project => project.hasManifest) || folders.length === 0 ? 'pass' : 'warn',
        detail: projects.length > 0
          ? projects.map(project => `${project.folder.name}: ${project.hasManifest ? 'ark.toml found' : 'single-file mode'}`).join('; ')
          : 'No workspace folders to inspect',
      },
      {
        id: 'target',
        label: 'Default target',
        status: 'pass',
        detail: String(config.get('target', 'wasm32-wasi-p1')),
      },
      {
        id: 'emit',
        label: 'Default emit',
        status: 'pass',
        detail: String(config.get('emit', 'core-wasm')),
      },
    ]

    return {
      checks,
      summary: {
        pass: checks.filter(check => check.status === 'pass').length,
        warn: checks.filter(check => check.status === 'warn').length,
        fail: checks.filter(check => check.status === 'fail').length,
      },
    }
  }

  function collectEnvironmentDiff() {
    const config = getConfiguration()
    const { command, extraArgs } = resolveServerCommand()
    const { command: resolvedBinary } = discoverBinary(config.get('server.path', 'arukellt'))
    const projects = detectProjects()
    const profile = projects[0] || {
      target: config.get('target', 'wasm32-wasi-p1'),
      emit: config.get('emit', 'core-wasm'),
    }

    const local = {
      label: 'Local (VS Code)',
      binary: resolvedBinary,
      extraArgs: extraArgs.length > 0 ? extraArgs.join(' ') : '(none)',
      target: String(config.get('target', 'wasm32-wasi-p1')),
      emit: String(config.get('emit', 'core-wasm')),
      selfhostWasm: process.env.ARUKELLT_SELFHOST_WASM || '(unset)',
      traceServer: String(config.get('trace.server', 'off')),
    }

    const profileSurface = {
      label: projects.length > 1 ? `Profile (${projects[0].folder.name})` : 'Profile (workspace)',
      binary: resolvedBinary,
      extraArgs: extraArgs.length > 0 ? extraArgs.join(' ') : '(none)',
      target: String(profile.target ?? 'wasm32-wasi-p1'),
      emit: String(profile.emit ?? 'core-wasm'),
      selfhostWasm: process.env.ARUKELLT_SELFHOST_WASM || '(unset)',
      traceServer: String(config.get('trace.server', 'off')),
    }

    const dimensions = ['binary', 'extraArgs', 'target', 'emit', 'selfhostWasm', 'traceServer']
    const rows = dimensions.map(key => {
      const values = {
        local: local[key],
        ci: CI_PROFILE[key],
        profile: profileSurface[key],
      }
      const mismatch = new Set([values.local, values.ci, values.profile]).size > 1
      return {
        key,
        label: key,
        ...values,
        mismatch,
      }
    })

    return { local, ci: CI_PROFILE, profile: profileSurface, rows }
  }

  function statusIcon(status) {
    switch (status) {
      case 'pass':
        return '$(check)'
      case 'warn':
        return '$(warning)'
      default:
        return '$(error)'
    }
  }

  async function runDoctorAction(action) {
    switch (action) {
      case 'openServerPathSettings':
        await vscode.commands.executeCommand('workbench.action.openSettings', 'arukellt.server.path')
        break
      case 'showOutput':
        await vscode.commands.executeCommand('arukellt.showOutput')
        break
      case 'restartLanguageServer':
        await vscode.commands.executeCommand('arukellt.restartLanguageServer')
        break
      case 'showEnvironmentDiff':
        await vscode.commands.executeCommand('arukellt.showEnvironmentDiff')
        break
      case 'showCommandGraph':
        await vscode.commands.executeCommand('arukellt.showCommandGraph')
        break
      default:
        break
    }
  }

  async function presentSetupDoctor(report = collectSetupDoctorReport(), options = {}) {
    if (options.headless || deps.isHeadlessOpsUi?.()) {
      return report
    }
    const items = report.checks.map(check => ({
      label: `${statusIcon(check.status)} ${check.label}`,
      description: check.detail,
      detail: check.action ? 'Select to run the suggested action' : undefined,
      check,
    }))

    items.push({
      label: '$(list-tree) Open Command Graph',
      description: 'Run check → compile → run → test from the workflow view',
      action: 'showCommandGraph',
    })
    items.push({
      label: '$(diff) Compare Environments',
      description: 'Inspect local vs CI vs profile assumptions',
      action: 'showEnvironmentDiff',
    })
    items.push({
      label: '$(output) Write full report to output',
      description: 'Append the doctor report to the language-server output channel',
      action: 'writeOutput',
    })

    const selection = await vscode.window.showQuickPick(items, {
      title: `Arukellt Setup Doctor (${report.summary.pass} pass, ${report.summary.warn} warn, ${report.summary.fail} fail)`,
      placeHolder: 'Select a check to inspect or run a follow-up action',
      matchOnDescription: true,
    })
    if (!selection) {
      return report
    }

    if (selection.action === 'writeOutput') {
      const lines = [
        '# Arukellt Setup Doctor',
        '',
        ...report.checks.map(check => `- [${check.status}] ${check.label}: ${check.detail}`),
      ]
      appendLanguageServerOutputLine(lines.join('\n'))
      await vscode.commands.executeCommand('arukellt.showOutput')
      return report
    }

    if (selection.action) {
      await runDoctorAction(selection.action)
      return report
    }

    if (selection.check?.action) {
      await runDoctorAction(selection.check.action)
    }
    return report
  }

  async function presentEnvironmentDiff(diff = collectEnvironmentDiff(), options = {}) {
    if (options.headless || deps.isHeadlessOpsUi?.()) {
      return diff
    }
    const items = diff.rows.map(row => ({
      label: `${row.mismatch ? '$(alert)' : '$(check)'} ${row.label}`,
      description: `local=${row.local}`,
      detail: `ci=${row.ci} | profile=${row.profile}`,
      row,
    }))

    items.unshift({
      label: '$(info) Surfaces',
      description: `${diff.local.label} vs ${diff.ci.label} vs ${diff.profile.label}`,
      kind: vscode.QuickPickItemKind.Separator,
    })

    const selection = await vscode.window.showQuickPick(items, {
      title: 'Arukellt Environment Diff',
      placeHolder: 'Mismatching dimensions are marked with $(alert)',
      matchOnDescription: true,
      matchOnDetail: true,
    })
    if (!selection || !selection.row) {
      return diff
    }

    await vscode.window.showInformationMessage(
      `${selection.row.label}: local=${selection.row.local}; ci=${selection.row.ci}; profile=${selection.row.profile}`,
      'Open Settings'
    ).then(choice => {
      if (choice === 'Open Settings') {
        vscode.commands.executeCommand('workbench.action.openSettings', 'arukellt')
      }
    })
    return diff
  }

  class CommandGraphTreeProvider {
    constructor() {
      this._onDidChangeTreeData = new vscode.EventEmitter()
      this.onDidChangeTreeData = this._onDidChangeTreeData.event
    }

    refresh() {
      this._onDidChangeTreeData.fire()
    }

    getTreeItem(element) {
      return element
    }

    getChildren(element) {
      if (!element) {
        const workflow = new vscode.TreeItem('Workflow', vscode.TreeItemCollapsibleState.Expanded)
        workflow.contextValue = 'category-workflow'
        workflow.description = 'check → compile → run; test branches from check'
        workflow.iconPath = new vscode.ThemeIcon('type-hierarchy-sub')
        const ops = new vscode.TreeItem('Operations', vscode.TreeItemCollapsibleState.Expanded)
        ops.contextValue = 'category-ops'
        ops.description = 'Diagnostics and environment tools'
        ops.iconPath = new vscode.ThemeIcon('tools')
        return [workflow, ops]
      }

      if (element.contextValue === 'category-workflow') {
        return COMMAND_GRAPH_WORKFLOW.map(node => {
          const item = new vscode.TreeItem(node.label, vscode.TreeItemCollapsibleState.None)
          item.description = node.detail
          item.tooltip = `${node.flow}\nRuns: ${node.command}`
          item.iconPath = new vscode.ThemeIcon(node.icon)
          item.contextValue = `graph-node-${node.id}`
          item.command = { command: node.command, title: node.label }
          return item
        })
      }

      if (element.contextValue === 'category-ops') {
        return COMMAND_GRAPH_OPS.map(node => {
          const item = new vscode.TreeItem(node.label, vscode.TreeItemCollapsibleState.None)
          item.description = node.detail
          item.tooltip = node.detail
          item.iconPath = new vscode.ThemeIcon(node.icon)
          item.contextValue = `graph-node-${node.id}`
          item.command = { command: node.command, title: node.label }
          return item
        })
      }

      return []
    }
  }

  function setupCommandGraphView(context) {
    commandGraphProvider = new CommandGraphTreeProvider()
    const treeView = vscode.window.createTreeView('arukellt-command-graph', {
      treeDataProvider: commandGraphProvider,
      showCollapseAll: true,
    })
    context.subscriptions.push(treeView)
    context.subscriptions.push(
      vscode.commands.registerCommand('arukellt.refreshCommandGraph', () => {
        commandGraphProvider.refresh()
      })
    )
    context.subscriptions.push(
      vscode.commands.registerCommand('arukellt.runWorkspaceTests', async () => {
        const tasks = await vscode.tasks.fetchTasks({ type: 'arukellt' })
        const testTask = tasks.find(task => task.definition.task === 'test')
        if (!testTask) {
          vscode.window.showWarningMessage('Arukellt: no arukellt:test task found in this workspace.')
          return
        }
        await vscode.tasks.executeTask(testTask)
      })
    )
  }

  async function revealCommandGraph(options = {}) {
    if (commandGraphProvider) {
      commandGraphProvider.refresh()
    }
    if (options.headless || deps.isHeadlessOpsUi?.()) {
      return
    }
    await vscode.commands.executeCommand('arukellt-command-graph.focus')
  }

  function getCommandGraphSnapshotForTests() {
    return {
      workflow: COMMAND_GRAPH_WORKFLOW.map(node => ({
        id: node.id,
        command: node.command,
      })),
      ops: COMMAND_GRAPH_OPS.map(node => ({
        id: node.id,
        command: node.command,
      })),
      hasProvider: !!commandGraphProvider,
    }
  }

  return {
    collectSetupDoctorReport,
    collectEnvironmentDiff,
    presentSetupDoctor,
    presentEnvironmentDiff,
    setupCommandGraphView,
    revealCommandGraph,
    getCommandGraphSnapshotForTests,
    COMMAND_GRAPH_WORKFLOW,
    COMMAND_GRAPH_OPS,
  }
}

module.exports = {
  createOpsSurfaces,
  COMMAND_GRAPH_WORKFLOW,
  COMMAND_GRAPH_OPS,
  CI_PROFILE,
}
