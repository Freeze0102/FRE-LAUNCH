# FRE LAUNCH — Minecraft Java Edition Launcher

**FRE LAUNCH · Your Minecraft. Your Rules.**

Український Minecraft Java launcher для Windows, розробляється на Tauri 2 + Rust + React.

## Завантажити з телефона

1. Відкрий [GitHub Actions](https://github.com/Freeze0102/FRE-LAUNCH/actions).
2. Вибери останній **успішний** запуск `Windows Build` для `main`.
3. Унизу сторінки знайди артефакт `FRE-LAUNCH-Windows-Installer` або `FRE-LAUNCH-Windows-Portable`.
4. Завантаж ZIP, перенеси його на Windows-комп'ютер, розпакуй та запусти інсталятор.

Будь-яка незавершена або червона збірка не є готовим інсталятором.

## Що має працювати у версії 0.3 Alpha

- Перелік усіх версій із маніфесту Mojang із пошуком і фільтрацією.
- Встановлення vanilla Minecraft: JAR, бібліотеки, ресурси, SHA-1 перевірка.
- Перевірка встановленої Java та спроба запуску в офлайн-сумісному режимі.
- Окремі збірки FRE INSTANCES з SQLite: створення, вибір, обране, редагування RAM/Java.
- FRE MODRINTH: реальний каталог проєктів та встановлення перевірених ресурс-паків.
- FRE SKIN STUDIO: 3D-перегляд скіна, імпорт PNG-скінів і плащів.
- FRE THEMES: Midnight, Neon, Ocean, Emerald, Light.

**Не готово:** Microsoft OAuth, модлоадери, менеджер серверів, резервні копії, автоматичне встановлення Java, скіни в онлайн-акаунті.

Увага: сам факт збірки застосунку **не підтверджує** успішний запуск Minecraft. Поведінка гри має бути перевірена на Windows.

## Розробка

Потрібні Node.js 22, Rust stable, Windows WebView2 та залежності Tauri 2.

```bash
npm install
npm run tauri icon src-tauri/icons/app.svg
npm run tauri dev
```

Для Windows installer:

```bash
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

## Поради з використання

- **Збірки:** відкрий «Збірки», створи профіль на вибраній версії Minecraft. Профілі використовують різні папки для світів і ресурсів.
- **Java:** використовуй Java, яку вимагає Minecraft. За потреби в налаштуваннях збірки вкажи шлях до `java.exe`.
- **Ресурс-паки:** обери збірку, перейди в «Моди», встанови сумісний resource pack з Modrinth. Увімкни його в самій грі.
- **Скіни:** імпортуй свій PNG у «Скіни». Це локальний 3D-перегляд, не зміна Microsoft-акаунта.
- **Проблеми:** відкрий «Логи» та скопіюй повідомлення про помилку.

Offline-профіль не замінює придбану ліцензію Minecraft і не відкриває онлайн-сервери з ліцензійною авторизацією.

Проєкт не пов'язаний із Mojang або Microsoft. Ігрові файли не входять до інсталятора.

Розвиток: [PLAN.md](PLAN.md), [PROGRESS.md](PROGRESS.md).
