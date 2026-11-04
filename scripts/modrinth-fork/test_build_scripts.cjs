// Check the actual YAML documents and every embedded PowerShell block.
const fs = require('node:fs')
const path = require('node:path')
const { createRequire } = require('node:module')
const { spawnSync } = require('node:child_process')
const assert = require('node:assert/strict')
const repo = path.resolve(__dirname, '../..')
const source = path.resolve(process.argv[2] || path.join(repo, 'FRE-Modrinth-Source'))
const yaml = createRequire(path.join(source, 'package.json'))('yaml')
const workflows = path.join(repo, '.github/workflows')
const blocks = []
for (const name of fs.readdirSync(workflows).filter(x => x.endsWith('.yml'))) {
  const doc = yaml.parseDocument(fs.readFileSync(path.join(workflows, name), 'utf8'), { uniqueKeys: true })
  assert.deepEqual(doc.errors, [], name)
  const data = doc.toJS()
  if (name !== 'fre-rust-pr-validation.yml') {
    assert.deepEqual(Object.keys(data.on), ['workflow_dispatch'], `${name} must run only when requested`)
  }
  for (const [jobName, job] of Object.entries(data.jobs)) {
    for (const [index, step] of (job.steps || []).entries()) {
      if (step.run && (step.shell === 'pwsh' || (!step.shell && job['runs-on']?.startsWith('windows')))) {
        blocks.push({ name: `${name}/${jobName}/${step.name || index}`, code: step.run.replace(/\$\{\{[\s\S]*?\}\}/g, 'CI_VALUE') })
      }
    }
  }
}
for (const name of ['BUILD-FRE-RUST.ps1', 'BUILD-FRE-WINDOWS.ps1', 'FRE-MODRINTH-LOCAL.ps1',
  'scripts/modrinth-fork/ci-build-native-engine.ps1', 'scripts/modrinth-fork/vendor-native-rust-source.ps1']) {
  blocks.push({ name, code: fs.readFileSync(path.join(repo, name), 'utf8') })
}
const check = spawnSync('pwsh', ['-NoProfile', '-NonInteractive', '-Command', `
  $ErrorActionPreference = 'Stop'
  $blocks = [Console]::In.ReadToEnd() | ConvertFrom-Json
  $failed = $false
  foreach ($block in $blocks) {
    $errors = $null
    $null = [System.Management.Automation.Language.Parser]::ParseInput($block.code, [ref]$null, [ref]$errors)
    if ($errors) { $failed = $true; Write-Output $block.name; $errors | ForEach-Object { Write-Output $_.Message } }
  }
  if ($failed) { exit 1 }
`], { input: JSON.stringify(blocks), encoding: 'utf8', windowsHide: true })
assert.equal(check.status, 0, check.error?.message || check.stdout + check.stderr)
console.log(`Validated all workflows and ${blocks.length} PowerShell scripts/blocks; heavy builds and publication require manual dispatch.`)
