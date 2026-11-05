# FRE LAUNCH — Release QA checklist

## Поточний Vue/Tauri + вбудований Minecraft на Rust

Команди з кореня репозиторію:

```powershell
python scripts/modrinth-fork/native_source.py vendor/fre-rust-source
python -m unittest discover -s scripts/modrinth-fork -p test_*.py -v
.\FRE-MODRINTH-LOCAL.ps1
node scripts/modrinth-fork/test_frontend.cjs FRE-Modrinth-Source
node scripts/modrinth-fork/test_build_scripts.cjs FRE-Modrinth-Source
node --test scripts/modrinth-fork/test_release.mjs scripts/modrinth-fork/test_content_security.cjs scripts/modrinth-fork/test_compat_frontend.cjs scripts/modrinth-fork/test_updater.cjs
python scripts/modrinth-fork/native_branding.py
cargo test --locked --release --manifest-path .audit/fre-native-source/current/Cargo.toml --target-dir vendor/fre-rust-source/target -p fre-core -p fre-prepare
# Окремий мережевий тест справжнього JDK downloader: не змінює глобальну Java.
cargo run --locked --manifest-path tests/native-jdk/Cargo.toml -- .audit/managed-jdk-check
.\BUILD-FRE-WINDOWS.ps1 -Preview
```

Остання команда також виконує генерацію локалізацій, повну перевірку Vue TypeScript,
production Vite build і `cargo test --locked -p theseus_gui` у згенерованому workspace,
а потім створює NSIS. Без `-Preview` потрібен наявний приватний ключ updater-каналу.
Ні ця команда, ні commit/push не запускають важкі Actions чи публікацію релізу.

Перевірка реального завантаження ресурсів, Java 25, світу та гри потребує входу
в Microsoft-акаунт власника Minecraft Java Edition. Для тесту використовуй новий
профіль і тестові каталоги; не змінюй наявні світи. Видалення Rust-профілю після
звичайного підтвердження видаляє і його світи; спільний кеш ресурсів зберігається.

Нижче збережено історичний перелік QA старої React-версії. Її `src-tauri` і workflow
`Windows Build` не замінюють поточні Vue/native-перевірки.

Цей документ розділяє **автоматичні перевірки коду** й **ручне тестування Windows/Minecraft**. Жодна галочка CI не означає, що на іншому ПК усе гарантовано працюватиме.

## Автоматичні перевірки (GitHub Actions → Windows Build)

- [ ] `npm run check` — TypeScript без помилок.
- [ ] `npm run build` — фронтенд збирається.
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` — Rust-тести, зокрема для ZIP/JAR.
- [ ] `npm run tauri build` — Windows-лаунчер та інсталятор збираються.
- [ ] Перевірка запуску встановленого лаунчера у Windows runner.
- [ ] Антивірусна та безпекова перевірка інсталяторів за правилами CI.

**Відмічай як успішні тільки після перегляду завершеного Windows Build саме потрібного commit SHA.** Workflow «Clean merged branches» не є збіркою. Скасований workflow теж не є успіхом.

## Ручні перевірки на Windows 10/11

### Запуск та закриття

- [ ] Vanilla Minecraft завантажується з Mojang і запускається з правильним Java.
- [ ] Закриття FRE LAUNCH після запуску не завершує Minecraft.
- [ ] Закриття консолі/термінала, з якого запущено лаунчер, не завершує гру.
- [ ] Повторний запуск лаунчера не пошкоджує світи й конфігурацію.
- [ ] При помилці журнал діагностики показує причину.

### Бібліотека

- [ ] Нова збірка відразу створює `mods`, `saves`, `resourcepacks`, `shaderpacks`.
- [ ] **Мої збірки → Файли** відкриває коректну папку саме цієї збірки.
- [ ] Java, RAM, ім'я збірки й тема зберігаються після повного перезапуску.
- [ ] Видалення збірки **зі списку** не видаляє світи.
- [ ] Резервна копія світу створюється та відновлюється лише після явного підтвердження.

### Моди та ресурси

- [ ] Пошук `Zoomify`, `Sodium` та інших назв працює через реальні результати Modrinth.
- [ ] У каталозі перемикаються сторінки та фільтри версій.
- [ ] Деталі проєкту показують сумісні версії під обрану збірку.
- [ ] Fabric-мод встановлюється разом із потрібними залежностями.
- [ ] Власний коректний `.jar` у `mods` проходить перевірку структури й може завантажитися, якщо сумісний.
- [ ] Пошкоджений, небезпечний або symlink-JAR блокує запуск із зрозумілою помилкою.
- [ ] Ресурс-пак перевіряє SHA-512, формат ZIP і розмір.
- [ ] Шейдер-пак встановлюється в `shaderpacks` і з'являється у вкладці «Моя бібліотека».
- [ ] Моди, ресурс-паки та шейдери можуть вимикатися та вмикатися повторно.
- [ ] Кнопка «Перевірити оновлення» для Fabric знаходить сумісні Modrinth-оновлення за SHA-512, але не переписує JAR автоматично.
- [ ] Пошук Modrinth сортує результати та фільтрує Minecraft-версію і Fabric-збірку.
- [ ] Немає автоматичних Windows Build після простого push: запуск тільки через ручний workflow_dispatch.
- [ ] Видалення/відключення модів не видаляє світи.
- [ ] CurseForge відкриває **сайт**, а не видається за інтегроване встановлення.

### Інтерфейс

- [ ] Головна, бібліотека, моди, версії, файли, налаштування та діагностика без обрізаних кнопок.
- [ ] Прокрутка та анімації не заважають запуску й навігації.
- [ ] Інтерфейс нормально масштабується при 100%, 125%, 150% у Windows.
- [ ] `prefers-reduced-motion` вимикає декоративні ефекти.

## Критерії випуску

1. **Розслідування v0.5.5 завершене**; є задокументований висновок щодо повідомлення Defender.
2. Windows Build для фінального SHA завершився **success**, а не «queued», «pending» або «cancelled».
3. Ключові ручні сценарії перевірені на реальному Windows-комп'ютері.
4. Новий номер версії, окремі SHA-256 файли та рішення про публічний реліз.
5. Ніколи не рекомендувати вимикати антивірус чи відновлювати підозрілий інсталятор з карантину.

Поточний безпековий статус — [SECURITY_NOTICE.md](SECURITY_NOTICE.md).
