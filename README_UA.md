# FRE LAUNCH 0.4 Alpha

**Your Minecraft. Your Rules.** — Minecraft Java Edition desktop launcher for Windows 10/11 x64, Tauri 2 + Rust + React.

## Як завантажити з Android
1. Відкрий [GitHub Actions](https://github.com/Freeze0102/FRE-LAUNCH/actions).
2. Вибери останню **успішну** збірку `Windows Build` з гілки `main`.
3. У розділі **Artifacts** завантаж `FRE-LAUNCH-Windows-Installer` або `FRE-LAUNCH-Windows-Portable`.
4. Перенеси ZIP на Windows, розпакуй та запусти інсталятор.

**Увага:** GitHub Actions підтверджує складання, але не повноцінний запуск Minecraft на фізичному Windows-ПК. Для 0.4 CI ще перевіряється, поки зміни в Pull Request.

## Наявні функції
- Перегляд Minecraft release, snapshot, old beta й alpha через офіційний Mojang manifest.
- Встановлення vanilla з client JAR, бібліотеками, assets та SHA-1 перевіркою.
- Спроба запуску офлайн-сумісної гри, логи в реальному часі, перевірка Java.
- SQLite-збірки (окремі світи, папки, RAM і Java), збережені offline-профілі.
- **Fabric Loader**: вибір сумісної версії через Fabric Meta та завантаження бібліотек.
- **Modrinth**: каталог модів, шейдерів, ресурс-паків, модпаків; SHA-512 перевірена установка Fabric-модів з required-залежностями; керування локальними модами.
- **FRE JAVA MANAGER**: пошук установлених Java; завантаження Windows x64 Eclipse Temurin JRE з перевіркою SHA-256.
- **FRE SERVERS**: збережені Minecraft Java сервери, статус-пінг і кількість гравців.
- **FRE SKIN STUDIO**: 3D-перегляд, скіни, плащі, піксельний редактор.
- **FRE THEMES**: Midnight, Neon, Ocean, Emerald і Light.
- **FRE BACKUPS**: ZIP-копії світів та обережне відновлення без перезапису існуючих світів.
- **FRE GALLERY**: перегляд скриншотів PNG вибраної збірки, експорт і видалення з підтвердженням.

## Як грати
1. На сторінці «Акаунти» створи локальний офлайн-профіль.
2. Вибери реліз Minecraft (краще почати з актуального).
3. У «Збірках» створи й обери свій інстанс.
4. У «Налаштуваннях» знайди Java. Якщо сумісної немає, завантаж Eclipse Temurin і прив'яжи до збірки.
5. Якщо потрібен Fabric, відкрий редагування збірки й вибери Fabric Loader.
6. Відкрий «Моди» та встанови сумісні Fabric-моди або ресурс-паки.
7. Натисни «Встановити» або «Грати» на головній сторінці.
8. Якщо виникла помилка, відкрий «Логи».

## Скриншоти Minecraft

Обери збірку в розділі «Збірки», відкрий «Скриншоти». У грі натисни **F2**, щоб створити знімок, а потім онови галерею. Галерея читає лише PNG-файли з папки цієї збірки та перевіряє розмір зображень.

## Розробка
Потрібні Rust stable, Node.js 22, Windows WebView2, системні залежності Tauri 2.

```bash
npm install
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri icon src-tauri/icons/app.svg
npm run tauri build
```

## Обмеження
- Microsoft OAuth та підтвердження Minecraft ownership ще не реалізовані.
- Forge, NeoForge і Quilt поки не підтримуються.
- Не гарантується запуск усіх історичних версій або всіх Fabric-конфігурацій.
- Немає повного i18n, автоматичних оновлень і підтвердженого Windows gameplay smoke test.
- Offline-профілі не є ліцензійною авторизацією і не обходять online-mode сервери.
- Встановлені моди — сторонній виконуваний код. Обирай перевірені проєкти.

Це неофіційна програма, не пов'язана з Mojang або Microsoft. Сам Minecraft не постачається разом із інсталятором.

Подробиці: [PLAN.md](PLAN.md) і [PROGRESS.md](PROGRESS.md).
