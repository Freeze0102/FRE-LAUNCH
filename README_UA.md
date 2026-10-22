# FRE LAUNCH

Minecraft Java Edition launcher — робочий прототип у розробці.

## Запуск для розробки

Потрібні Node.js, Rust, системні залежності Tauri 2 та Windows WebView2.

```bash
npm install
npm run tauri dev
```

## Збірка Windows

```bash
npm install
npm run tauri build
```

GitHub Actions: вкладка **Actions** → **Windows Build** → **Run workflow**. Після успішної збірки завантажте artifact.

## Що працює

- Сучасний інтерфейс із навігацією.
- Завантаження списку офіційних версій Minecraft через Rust.
- Пошук та фільтрація версій.
- Визначення Java через системний PATH.
- Локальні офлайн-профілі (лише збереження в інтерфейсі).

**Minecraft ще не встановлюється і не запускається.** Кнопка запуску чесно позначена як недоступна. Microsoft OAuth, моди, скіни та інші модулі ще не реалізовані.
