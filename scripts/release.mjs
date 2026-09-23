import { readFileSync, writeFileSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// Repository-specific settings. Every other line is shared with the sibling Tauri apps.
const PACKAGE_NAME = 'ytdown'
const CRATE_NAME = 'ytdown'
// Other committed version fields: [file, pattern capturing (prefix)(version), expected match count].
const EXTRA_TARGETS = []

const level = process.argv[2]
if (!['patch', 'minor', 'major'].includes(level)) throw new Error('Usage: node scripts/release.mjs patch|minor|major')
const root = process.cwd()
const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim()
if (git('status', '--porcelain')) throw new Error('先に作業内容をコミットしてください。')
const startHead = git('rev-parse', 'HEAD')
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'))
if (pkg.name !== PACKAGE_NAME) throw new Error(`${PACKAGE_NAME} リポジトリで実行してください。`)
const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(pkg.version)
if (!match) throw new Error('安定版の x.y.z 形式のバージョンが必要です。')
const [major, minor, patch] = match.slice(1).map(Number)
// Keep normal SemVer behavior even for 0.x (changelogen otherwise remaps minor to patch).
const version = level === 'major' ? `${major + 1}.0.0` : level === 'minor' ? `${major}.${minor + 1}.0` : `${major}.${minor}.${patch + 1}`
if (git('tag', '--list', `v${version}`)) throw new Error(`タグ v${version} は既に存在します。`)

// Each locator returns the [start, end) offsets of the version values it finds.
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
  // Cargo.lock is machine-written, so its layout is fixed apart from line endings.
  ['src-tauri/Cargo.lock', matchValues(new RegExp(`^(\\[\\[package\\]\\]\\r?\\nname = "${CRATE_NAME}"\\r?\\nversion = ")([^"]+)"`, 'gm'))],
  ...EXTRA_TARGETS.map(([file, pattern, expected]) => [file, matchValues(pattern), expected]),
]
// Validate all files before changelogen updates package.json.
const updates = targets.map(([file, locate, expected = 1]) => {
  const content = readFileSync(join(root, file), 'utf8')
  const found = locate(content)
  if (found.length !== expected) throw new Error(`${file} のバージョン欄を確認してください（${expected} 箇所のはずが ${found.length} 箇所に一致）。`)
  // Replace from the end so earlier offsets stay valid.
  const next = found.toSorted(([a], [b]) => b - a).reduce((text, [from, to]) => text.slice(0, from) + version + text.slice(to), content)
  return [file, next]
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
  git('tag', '-a', `v${version}`, '-m', `v${version}`)
} catch (error) {
  // The tree was clean at start, so returning to the starting commit undoes only this run.
  try {
    git('reset', '-q', '--hard', startHead)
  } catch (rollbackError) {
    throw new AggregateError([error, rollbackError], `リリースに失敗し、巻き戻しもできませんでした。git status と git log で ${startHead.slice(0, 7)} からの変化を確認してください。`)
  }
  throw error
}
console.log(`${PACKAGE_NAME} v${version}: changelog、各バージョン、ローカルのコミットとタグを更新しました。`)
