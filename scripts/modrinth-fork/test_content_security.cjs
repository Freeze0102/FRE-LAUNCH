// Execute the real shared Markdown and ICU translation code with installed dependencies.
// No launcher process, user settings, or Minecraft files are opened.
const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const { createRequire } = require('node:module')
const source = path.resolve(process.argv[2] || 'FRE-Modrinth-Source')
const app = createRequire(path.join(source, 'apps/app-frontend/package.json'))
const ui = createRequire(path.join(source, 'packages/ui/package.json'))
const utils = createRequire(path.join(source, 'packages/utils/package.json'))
const ts = app('typescript')

function load(file, require) {
  const output = ts.transpileModule(fs.readFileSync(path.join(source, file), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, esModuleInterop: true },
  }).outputText
  const exports = {}
  new Function('exports', 'require', output)(exports, require)
  return exports
}

const markdown = load('packages/utils/parse.ts', utils)

test('Updated Markdown preserves mod descriptions, tables, code, and safe external links', () => {
  const html = markdown.renderString('## Опис\n\n**Fabric** [документація](https://example.org/docs)\n\n| Мод | Версія |\n| --- | --- |\n| Test | 1 |\n\n```java\nSystem.out.println("test");\n```')
  assert.match(html, /<h2>Опис<\/h2>/)
  assert.match(html, /<strong>Fabric<\/strong>/)
  assert.match(html, /rel="noopener nofollow ugc"/)
  assert.match(html, /<table>/)
  assert.match(html, /language-java/)
})

test('Real description and inline renderers remove executable HTML and javascript links', () => {
  const hostile = '<img src="https://cdn.modrinth.com/image.png" onerror="alert(1)"><script>alert(1)</script><iframe src="https://evil.test/" srcdoc="<script>alert(1)</script>"></iframe><a href="javascript:alert(1)">bad</a>'
  const html = markdown.renderString(hostile)
  assert.doesNotMatch(html, /<(?:script|iframe)[^>]*\b(?:src|srcdoc)=|<script>|\bonerror=|\bhref="javascript:/i)
  const inline = markdown.renderBasicInlineMarkdown('[bad](javascript:alert(1))\n\n**Safe**', { target: '_blank' })
  assert.doesNotMatch(inline, /href="javascript:/i)
  assert.match(inline, /<strong>Safe<\/strong>/)
})

test('Vue i18n 11 works with the real custom ICU compiler and Ukrainian plural forms', () => {
  const messages = load('packages/ui/src/composables/i18n.ts', name => {
    if (name.startsWith('../providers/') || name === './i18n-debug') return {}
    return ui(name)
  })
  const i18n = app('vue-i18n').createI18n({
    legacy: false, locale: 'uk-UA', fallbackLocale: 'en-US',
    messageCompiler: messages.createMessageCompiler(),
    messages: messages.buildLocaleMessages({
      './app/uk-UA/index.json': { default: { files: { defaultMessage: '{count, plural, one {# файл} few {# файли} many {# файлів} other {# файлу}}' } } },
      './app/en-US/index.json': { default: { fallback: 'Ready {name}' } },
    }),
    missingWarn: false, fallbackWarn: false,
  })
  assert.equal(i18n.global.t('files', { count: 1 }), '1 файл')
  assert.equal(i18n.global.t('files', { count: 2 }), '2 файли')
  assert.equal(i18n.global.t('files', { count: 5 }), '5 файлів')
  assert.equal(i18n.global.t('fallback', { name: 'Player' }), 'Ready Player')
  i18n.dispose()
})
