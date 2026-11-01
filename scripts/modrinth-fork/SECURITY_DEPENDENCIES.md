# Зафіксовані виправлення залежностей — 01.11.2026

`security-dependencies.json` задає точні overrides і посилання на advisories.
`security-pnpm-lock.yaml` — повний lockfile, отриманий штатним
`pnpm install --lockfile-only --ignore-scripts` поверх незміненого Modrinth
`4ccf4d51804627cd0687ce7dfd5004f20c267acc`. Це multi-document YAML: перший
документ описує package manager, другий — workspace. Обидва потрібні.

`feature_security_dependencies.py` перевіряє SHA-256 початкових lockfile та
workspace, а також кінцевого lockfile до запису. Після підготовки застосовується
`pnpm --filter '@modrinth/app...' install --frozen-lockfile`. Невідповідний pin
або зміна lockfile зупиняють підготовку; audit не вимикається і не фільтрується
списком ігнорування advisories.

Оновлено HTML/Markdown, Vue i18n, архівні та кольорові парсери, а також засоби
збірки й розробки. Overrides для старих major-гілок обмежені діапазоном,
щоб не змінити їхній API непомітно. Storybook і його прямі addons узгоджені на
10.6.2. Desktop Vite — 8.1.6, Vite для Storybook — 6.4.4.

Загальний `minimumReleaseAge: 10080` (7 днів) збережено. Винятки називають
лише **точні** security-релізи `glob@14.0.2`, `minimatch@11.0.1`,
`path-scurry@3.0.0`, `@isaacs/nonrecursive-glob-match@1.0.1`: це виправлення
[скасування обходу файлової системи](https://github.com/advisories/GHSA-3g3m-2fjr-6pw4)
та [надмірного розгортання glob](https://github.com/advisories/GHSA-g3hv-75wg-m249)
і їхні залежності. Версії, registry integrity та дати публікації перевірені;
глобального обходу перевірки віку пакетів немає.

Підсумковий `pnpm --filter '@modrinth/app...' audit --prod --json`:
**0 advisories**. Audit із dev-залежностями: **0 high/critical, 2 moderate**:

- [Vite 6: GHSA-8g5g-mxqh-grgj](https://github.com/advisories/GHSA-8g5g-mxqh-grgj)
  у засобах Storybook; audit не надає виправлення для цієї гілки. Сервер
  Storybook під час аудиту не запускався, desktop release використовує Vite 8.
- [minimatch: GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249)
  у старих API-гілках lint/API Extractor. Безпечні minor/patch-оновлення усувають
  high advisories, але для цього moderate потрібна міграція батьківських
  інструментів на minimatch 11. Примусова глобальна заміна зламала б API v3.

Ці два findings залишаються відкритими. Production-аудит не є перевіркою
всього npm monorepo, Rust crates, бінарних компонентів або гарантією відсутності
шкідливого коду. Попередній інцидент v0.5.5 описано окремо у
[`SECURITY_NOTICE.md`](../../SECURITY_NOTICE.md).

Регресії `test_content_security.cjs` виконують справжні shared Markdown/HTML
renderers і власний ICU compiler з установленими новими бібліотеками:
базове форматування, безпечні посилання, небезпечний HTML, українські plural
форми й англійський fallback. Повна Vue/Tauri збірка додатково перевіряє сумісність.
