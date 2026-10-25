# FRE LAUNCH — локальна перевірка справжнього рушія Modrinth

Починаючи з цієї версії, **`FRE-LAUNCH-LOCAL.cmd` запускає форк оригінального застосунку Modrinth (Vue + Tauri + Rust)**. Це не попередня React-версія FRE LAUNCH, яка могла завершуватися з `STATUS_STACK_OVERFLOW` на потоці `tokio-rt-worker`.

Повний вихідний код оригінального Modrinth під'єднано як Git submodule `vendor/modrinth` з фіксованим комітом `4ccf4d51804627cd0687ce7dfd5004f20c267acc`. Якщо проєкт завантажено через ZIP, PowerShell-скрипт сам отримує саме цей код із `https://github.com/modrinth/code.git`, перевіряє SHA й створює окрему папку `FRE-Modrinth-Source`. **Це оригінальний код, а не відтворення інтерфейсу за скриншотами.**

У копії зберігаються всі файли вихідного коду й залежності монорепозиторію Modrinth. Скрипт додає брендинг FRE LAUNCH, локальні ніки та український переклад. Оригінальні GPLv3-повідомлення й атрибуція залишаються.

## Підготуй інструменти Windows

На ПК потрібно встановити **Git**, **Python 3** із командою `py`, **Node.js 24.15+**, **pnpm 12.11.2+**, **Rust 1.95+** (MSVC), **Visual Studio C++ Build Tools** (Desktop development with C++) та **Microsoft Edge WebView2**.

Перевір у PowerShell:

```powershell
git --version
py -3 --version
node --version
pnpm --version
rustc --version
```

Для pnpm після встановлення Node.js 24 можна використати `npm install -g pnpm@12.11.2`. Обов'язково онови Node, якщо залишився Node 22 від старого FRE LAUNCH.

## Що запускати

Витягни свіжий ZIP репозиторію або зроби `git pull --ff-only origin main`. Двічі клацни по **`FRE-LAUNCH-LOCAL.cmd`**:

- **1 — Запуск Modrinth-based FRE LAUNCH наживо**, у режимі розробки без інсталятора.
- **2 — Створення NSIS-інсталятора Windows** на основі тієї самої кодової бази.
- **3 — Стара React-версія** через `FRE-LAUNCH-LEGACY.cmd`, лише для діагностики; її `STATUS_STACK_OVERFLOW` ще не локалізовано.
- **4 — Вихід**.

Перший запуск завантажить вихідний код Modrinth та встановить залежності. Наступні запуски повторно використають уже підготовлену `FRE-Modrinth-Source`. Якщо патчі оновляться, а згенерована папка залишилася від попередньої версії, **перейменуй** `FRE-Modrinth-Source` в архівну назву перед повторною підготовкою; не видаляй свої Minecraft-папки.

Інсталятор після успішного створення шукай у `FRE-Modrinth-Source\target\release\bundle\nsis\`. Код окремого застосунку має власний ідентифікатор `app.frelaunch.modrinthfork` та **не переносить автоматично** світи/акаунти зі старого `app.frelaunch.desktop`.

### Якщо виникла помилка

Скопіюй **перші рядки помилки** з термінала. Сам факт `cargo build` або `pnpm build` не доводить, що реальний запуск гри успішний. Можливі також перешкоди входу через Microsoft: треба окремо перевірити OAuth на Windows.

Якщо ти запускаєш саме *старий React-лаунчер* і знову бачиш `tokio-rt-worker has overflowed its stack`, це збій процесу FRE LAUNCH, а не доведений збій Java. Переглянь `%LOCALAPPDATA%\FRE-LAUNCH\startup.log` та журнал встановлення для визначення стадії; не збільшуй стек навмання і не вимикай захист Windows.

Перш ніж тестувати встановлення модпаків, **збережи резервні копії світів**. Жодні сценарії збірки не потребують запусків GitHub Actions.
