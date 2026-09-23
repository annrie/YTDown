import { readFileSync, writeFileSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// Repository-specific settings. Every other line is shared with the sibling Tauri apps.
const PACKAGE_NAME = 'ytdown'
const CRATE_NAME = 'ytdown'

const level = process.argv[2]
if (!['patch', 'minor', 'major'].includes(level)) throw new Error('Usage: node scripts/release.mjs patch|minor|major')
const root = process.cwd()
const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim()
if (git('status', '--porcelain')) throw new Error('先に作業内容をコミットしてください。')
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'))
if (pkg.name !== PACKAGE_NAME) throw new Error(`${PACKAGE_NAME} リポジトリで実行してください。`)
const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(pkg.version)
if (!match) throw new Error('安定版の x.y.z 形式のバージョンが必要です。')
const [major, minor, patch] = match.slice(1).map(Number)
// Keep normal SemVer behavior even for 0.x (changelogen otherwise remaps minor to patch).
const version = level === 'major' ? `${major + 1}.0.0` : level === 'minor' ? `${major}.${minor + 1}.0` : `${major}.${minor}.${patch + 1}`
if (git('tag', '--list', `v${version}`)) throw new Error(`タグ v${version} は既に存在します。`)

// Each locator returns the [start, end) offsets of the version values it finds; exactly one is required.
const matchValues = (pattern) => (content) => [...content.matchAll(pattern)].map((m) => [m.index + m[1].length, m.index + m[1].length + m[2].length])
// Walk Cargo.toml table by table so a version key in any other table can never be picked up.
const packageVersion = (content) => {
  const found = []
  let offset = 0
  let inPackage = false
  for (const line of content.split('\n')) {
    const header = /^\s*\[/.test(line)
    if (header) inPackage = line.trim().startsWith('[package]')
    else if (inPackage) {
      const m = /^(\s*version\s*=\s*")([^"]+)"/.exec(line)
      if (m) found.push([offset + m[1].length, offset + m[1].length + m[2].length])
    }
    offset += line.length + 1
  }
  return found
}
const targets = [
  ['src-tauri/tauri.conf.json', matchValues(/^(\s*"version": ")([^"]+)"/gm)],
  ['src-tauri/Cargo.toml', packageVersion],
  // Cargo.lock is machine-written, so its layout is fixed.
  ['src-tauri/Cargo.lock', matchValues(new RegExp(`^(\\[\\[package\\]\\]\\nname = "${CRATE_NAME}"\\nversion = ")([^"]+)"`, 'gm'))],
]
// Validate all files before changelogen updates package.json.
const updates = targets.map(([file, locate]) => {
  const content = readFileSync(join(root, file), 'utf8')
  const found = locate(content)
  if (found.length !== 1) throw new Error(`${file} のバージョン欄を確認してください（${found.length} 箇所に一致）。`)
  const [[from, to]] = found
  return [file, content.slice(0, from) + version + content.slice(to)]
})
const heading = new RegExp(`^## v${version.replaceAll('.', '\\.')}(\\s|$)`, 'm')
if (heading.test(readFileSync(join(root, 'CHANGELOG.md'), 'utf8'))) throw new Error(`CHANGELOG.md に v${version} の見出しが既にあります。`)
const cli = join(dirname(fileURLToPath(import.meta.resolve('changelogen'))), 'cli.mjs')
const files = ['package.json', 'CHANGELOG.md', ...updates.map(([file]) => file)]
try {
  execFileSync(process.execPath, [cli, '--bump', '-r', version], { cwd: root, stdio: 'inherit' })
  // changelogen logs its errors but still exits 0, so check what it was supposed to write.
  if (JSON.parse(readFileSync(join(root, 'package.json'), 'utf8')).version !== version) throw new Error('changelogen が package.json を更新できませんでした。')
  if (!heading.test(readFileSync(join(root, 'CHANGELOG.md'), 'utf8'))) throw new Error(`changelogen が CHANGELOG.md に v${version} の見出しを書けませんでした。`)
  for (const [file, contents] of updates) writeFileSync(join(root, file), contents)
  git('add', '--', ...files)
  const message = (pkg.changelog?.templates?.commitMessage || 'chore(release): v{{newVersion}}').replaceAll('{{newVersion}}', version)
  git('commit', '-m', message)
} catch (error) {
  // The tree was clean at start, so restoring these paths undoes only this run.
  try {
    git('checkout', 'HEAD', '--', ...files)
  } catch (rollbackError) {
    throw new AggregateError([error, rollbackError], `リリースに失敗し、巻き戻しもできませんでした。git status で ${files.join(' ')} を確認してください。`)
  }
  throw error
}
git('tag', '-a', `v${version}`, '-m', `v${version}`)
console.log(`${PACKAGE_NAME} v${version}: changelog、各バージョン、ローカルのコミットとタグを更新しました。`)
