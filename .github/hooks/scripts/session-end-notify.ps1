param(
  [string]$Title = "Copilot CLI"
)

$ErrorActionPreference = "Stop"

$inputRaw = [Console]::In.ReadToEnd()
$payload = $null

if (-not [string]::IsNullOrWhiteSpace($inputRaw)) {
  try {
    $payload = $inputRaw | ConvertFrom-Json
  } catch {
    $payload = $null
  }
}

$cwd = if ($payload -and $payload.cwd) {
  [string]$payload.cwd
} else {
  (Get-Location).Path
}

$projectName = Split-Path -Leaf $cwd
if ([string]::IsNullOrWhiteSpace($projectName)) {
  $projectName = "workspace"
}

$reason = if ($payload -and $payload.reason) {
  [string]$payload.reason
} else {
  "complete"
}

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

$tipIcon = [System.Windows.Forms.ToolTipIcon]::Info

switch ($reason) {
  "complete" {
    $message = "${projectName}: task completed."
  }
  "error" {
    $message = "${projectName}: task ended with an error."
    $tipIcon = [System.Windows.Forms.ToolTipIcon]::Error
  }
  "abort" {
    $message = "${projectName}: task was aborted."
    $tipIcon = [System.Windows.Forms.ToolTipIcon]::Warning
  }
  "timeout" {
    $message = "${projectName}: task timed out."
    $tipIcon = [System.Windows.Forms.ToolTipIcon]::Warning
  }
  "user_exit" {
    $message = "${projectName}: session ended."
  }
  default {
    $message = "${projectName}: session ended ($reason)."
  }
}

$notify = New-Object System.Windows.Forms.NotifyIcon
$notify.Icon = [System.Drawing.SystemIcons]::Information
$notify.BalloonTipIcon = $tipIcon
$notify.BalloonTipTitle = $Title
$notify.BalloonTipText = $message
$notify.Visible = $true

try {
  $notify.ShowBalloonTip(5000)
  Start-Sleep -Milliseconds 5500
} finally {
  $notify.Dispose()
}
