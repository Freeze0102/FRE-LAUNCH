"""Complete missing Ukrainian translations in the pinned Modrinth desktop fork.

Existing Ukrainian strings are never overwritten. English is used only as
a fallback for keys not supplied here, preserving ICU variable placeholders.
"""
import json
from pathlib import Path

APP = {
"app.file-download.canceling":"Скасування завантаження…",
"app.file-download.preparing":"Підготовка завантаження…",
"app.file-download.waiting":"Очікування відповіді сервера…",
"app.instance.confirm-delete.bulk-admonition-body":"Усі дані цих {count} збірок буде назавжди видалено, включно зі світами, налаштуваннями та встановленим вмістом.",
"app.instance.confirm-delete.bulk-delete-button":"Видалити {count} збірок",
"app.instance.confirm-delete.bulk-header":"Видалити {count} збірок",
"app.modal.install-to-play.unavailable-description":"Версія збірки, яку використовує сервер, зараз недоступна для завантаження. Можливо, її приховано або вона проходить перевірку. Спробуйте пізніше або зверніться до власника сервера.",
"app.modal.install-to-play.unavailable-header":"Збірка цього сервера недоступна",
"app.settings.sidebar.label.instance-defaults":"Типові налаштування збірок",
"app.settings.sidebar.label.system":"Система",
"app.settings.tabs.java-and-memory":"Java та пам’ять",
"app.settings.tabs.launch-hooks":"Команди запуску",
"app.settings.tabs.resources":"Ресурси",
"app.settings.tabs.syncing":"Синхронізація",
"app.settings.tabs.troubleshooting":"Усунення проблем",
"app.settings.tabs.window":"Вікно",
"app.settings.troubleshooting.clear-cache":"Очистити кеш",
"app.settings.troubleshooting.clear-cache.confirm.title":"Очистити кеш застосунку?",
"app.settings.troubleshooting.report-ad.copied":"Скопійовано",
"app.settings.troubleshooting.report-ad.copy":"Скопіювати дані реклами",
"app.settings.troubleshooting.report-ad.description":"Побачили недоречну або образливу рекламу? Скопіюйте її дані, доки вона відображається, і надішліть у <support>підтримку Modrinth</support>. Ми перевіримо її та приберемо.",
"app.settings.troubleshooting.report-ad.no-ad.text":"Дочекайтеся завантаження реклами й повторіть спробу.",
"app.settings.troubleshooting.report-ad.no-ad.title":"Рекламу не знайдено",
"app.settings.troubleshooting.report-ad.title":"Повідомити про рекламу",
"app.shared-instance-publish.preparing":"Підготовка оновлення…",
"app.shared-instance-publish.pushed":"Оновлення надіслано",
"app.shared-instance-publish.uploading":"Надсилання оновлення…",
"instance.settings.defaults.open-app-settings":"Керувати типовими налаштуваннями",
"instance.settings.sidebar.label.game":"Гра",
"instance.settings.sidebar.label.instance":"Збірка",
"instance.settings.tabs.hooks.default-settings-description":"Для цієї збірки використовуються типові команди запуску, доки ви їх не зміните.",
"instance.settings.tabs.hooks.label":"Команди запуску",
"instance.settings.tabs.java.default-settings-description":"Для цієї збірки використовуються типові налаштування Java та пам’яті, доки ви їх не зміните.",
"instance.settings.tabs.java.label":"Java та пам’ять",
"instance.settings.tabs.syncing":"Синхронізація",
"instance.settings.tabs.window.default-settings-description":"Для цієї збірки використовуються типові налаштування вікна, доки ви їх не зміните.",
"instance.settings.tabs.window.label":"Вікно",
"instance.shared-instance.file-not-allowed.title":"Цей тип файлу заборонено",
"instance.shared-instance.publish-review.changes-description":"Перегляньте зміни, виявлені після останнього оновлення. Вони автоматично ввійдуть до публікації.",
"instance.shared-instance.publish-review.changes-tab":"Зміни",
"instance.shared-instance.publish-review.file-type-not-allowed":"Файли цього типу не можна надсилати.",
"instance.shared-instance.publish-review.files-description":"Ви також можете додати окремі файли. Виберіть додаткові файли для публікації.",
"instance.shared-instance.publish-review.files-tab":"Файли",
"instance.shared-instance.publish-review.storage-over-limit":"Перевищення ліміту на {size}. Зніміть вибір із частини файлів.",
"instance.shared-instance.publish-review.storage-title":"Ліміт сховища",
"instance.shared-instance.publish-review.storage-usage":"Використано {used} із {limit}"
}

UI = {
"color-picker.hex-value":"Шістнадцяткове значення: {label}",
"color-picker.hue":"Відтінок: {label}",
"color-picker.saturation-and-brightness":"Насиченість і яскравість: {label}",
"content.confirm-deletion.delete-button":"Видалити {count, number} {itemType}",
"content.page-layout.empty.hint":"Перегляньте або завантажте {contentType}, щоб почати",
"content.page-layout.search-placeholder":"Пошук серед {count, number} {contentType}…",
"content.selection-bar.bulk.deleting":"Видалення {progress}/{total}: {contentType}…",
"content.selection-bar.bulk.deleting-count":"Видалення {count, number}: {contentType}",
"content.selection-bar.bulk.deleting-waiting":"Видалення: {contentType}…",
"content.selection-bar.bulk.disabling":"Вимкнення {progress}/{total}: {contentType}…",
"content.selection-bar.bulk.disabling-count":"Вимкнення {count, number}: {contentType}",
"content.selection-bar.bulk.disabling-waiting":"Вимкнення: {contentType}…",
"content.selection-bar.bulk.enabling":"Увімкнення {progress}/{total}: {contentType}…",
"content.selection-bar.bulk.enabling-count":"Увімкнення {count, number}: {contentType}",
"content.selection-bar.bulk.enabling-waiting":"Увімкнення: {contentType}…",
"content.selection-bar.bulk.updating":"Оновлення {progress}/{total}: {contentType}…",
"content.selection-bar.bulk.updating-count":"Оновлення {count, number}: {contentType}",
"content.selection-bar.bulk.updating-waiting":"Оновлення: {contentType}…",
"content.selection-bar.selected-count":"Вибрано {count, number}: {contentType}",
"external-files.permissions-card.file-database-id":"Ідентифікатор у базі даних",
"hosting.save-banner.saved":"Збережено",
"profile.button.edit-restrictions":"Змінити обмеження",
"profile.button.edit-user":"Редагувати користувача",
"profile.button.restrict":"Обмежити користувача",
"profile.label.restricted":"Обмежений доступ",
"profile.restrict-user.admonition-body":"{username} залишиться в акаунті, але запити, що потребують вилучених дозволів, відхилятимуться до зняття обмежень.",
"profile.restrict-user.admonition-title":"Вибрані дозволи буде вилучено",
"profile.restrict-user.error-description":"Під час оновлення користувача виникла помилка. Спробуйте ще раз.",
"profile.restrict-user.header":"Обмежити користувача {username}",
"profile.restrict-user.private-reason-label":"Приватна примітка (лише для працівників)",
"profile.restrict-user.private-reason-placeholder":"Необов’язково",
"profile.restrict-user.reason-label":"Причина (видима користувачеві)",
"profile.restrict-user.reason-placeholder":"Необов’язково",
"profile.restrict-user.remove-button":"Зняти обмеження",
"profile.restrict-user.remove-error-title":"Не вдалося зняти обмеження",
"profile.restrict-user.remove-success-description":"Обмеження користувача {username} знято.",
"profile.restrict-user.remove-success-title":"Обмеження знято",
"profile.restrict-user.save-button":"Зберегти обмеження",
"profile.restrict-user.save-error-title":"Не вдалося зберегти обмеження",
"profile.restrict-user.save-success-description":"Обмеження користувача {username} оновлено.",
"profile.restrict-user.save-success-title":"Обмеження збережено",
"project.settings.environment.bulk.apply-button":"Застосувати до всіх версій",
"project.settings.environment.bulk.description":"Це замінить середовище для кожної версії цього проєкту.",
"project.settings.environment.bulk.title":"Застосувати середовище",
"project.settings.environment.modal.title":"Редагувати середовище проєкту",
"servers.backups.create.in-progress":"Створення резервної копії вже заплановано або триває",
"servers.backups.create.slots-full":"Усі {limit, plural, one {# слот ручних копій зайнятий} few {# слоти ручних копій зайняті} other {# слотів ручних копій зайнято}}",
"servers.backups.delete.error":"Не вдалося видалити {count, plural, one {резервну копію} few {# резервні копії} other {# резервних копій}}",
"servers.backups.history.auto-badge":"Автоматично",
"servers.backups.history.column.created":"Створено",
"servers.backups.history.column.created-by":"Автор",
"servers.backups.history.column.name":"Назва",
"servers.backups.history.creator-avatar-alt":"Аватар користувача {username}",
"servers.backups.history.creator.support":"Підтримка",
"servers.backups.history.more-options":"Додаткові дії для {name}",
"servers.backups.history.no-results":"За запитом резервних копій не знайдено.",
"servers.backups.history.rename":"Перейменувати",
"servers.backups.history.restore":"Відновити",
"servers.backups.history.search-label":"Пошук резервних копій",
"servers.backups.history.search-placeholder":"Шукати серед {count, plural, one {# резервної копії} few {# резервних копій} other {# резервних копій}}…",
"servers.backups.history.title":"Історія резервних копій",
"servers.backups.interval.days":"{days, plural, one {# день} few {# дні} other {# днів}}",
"servers.backups.interval.every-days":"Кожні {days, plural, one {# день} few {# дні} other {# днів}}",
"servers.backups.interval.every-hours":"Кожні {hours} год",
"servers.backups.interval.hours":"{hours} год",
"servers.backups.interval.weekly":"Щотижня",
"servers.backups.load-error.description":"Не вдалося завантажити резервні копії сервера.",
"servers.backups.load-error.title":"Помилка завантаження резервних копій",
"servers.backups.restore.operation-in-progress":"Операція з резервними копіями вже запланована або виконується",
"servers.backups.restore.server-running":"Перед відновленням зупиніть сервер",
"servers.backups.rotation-modal.admonition-body":"Зменшення кількості слотів означає, що {count, plural, one {найстарішу автоматичну копію} few {# найстаріші автоматичні копії} other {# найстаріших автоматичних копій}} буде безповоротно видалено. Цю дію не можна скасувати.",
"servers.backups.rotation-modal.admonition-header":"Попередження про видалення",
"servers.backups.rotation-modal.backups-label":"{count, plural, one {Резервна копія} few {Копії ({count})} other {Копії ({count})}}",
"servers.backups.rotation-modal.confirm":"Зберегти й видалити",
"servers.backups.rotation-modal.header":"Видалити автоматичні копії?",
"servers.backups.slots.auto.next":"Наступна: {time}",
"servers.backups.slots.auto.next-soon":"Незабаром почнеться резервне копіювання",
"servers.backups.slots.auto.off":"Вимкнено",
"servers.backups.slots.auto.off-description":"Увімкніть автоматичні копії в налаштуваннях",
"servers.backups.slots.auto.title":"Автоматичні копії",
"servers.backups.slots.limit":"/ {limit, plural, one {# слот} few {# слоти} other {# слотів}}",
"servers.backups.slots.manual.description":"Створені вручну",
"servers.backups.slots.manual.title":"Ручні резервні копії",
"servers.backups.toolbar.more-create-options":"Інші способи резервного копіювання",
"servers.region.out-of-stock":"(Немає в наявності)",
"servers.region.ping.failed":"Не вдалося виміряти затримку",
"servers.region.ping.latency":"{ping} мс",
"servers.region.ping.testing":"Перевірка з’єднання…",
"servers.settings.backups.auto.description":"Автоматично створювати резервні копії сервера через визначені проміжки часу.",
"servers.settings.backups.auto.title":"Автоматичні резервні копії",
"servers.settings.backups.interval.title":"Інтервал резервного копіювання",
"servers.settings.backups.manual-slots.description":"Визначте, скільки з {limit} слотів використовувати для ручних і автоматичних копій.",
"servers.settings.backups.manual-slots.title":"Слоти ручних копій",
"servers.settings.backups.save.error-title":"Не вдалося зберегти налаштування резервного копіювання",
"user-restrictions.group.collections":"Колекції",
"user-restrictions.group.create-projects":"Створення проєктів",
"user-restrictions.group.edit-projects":"Редагування проєктів",
"user-restrictions.group.organizations":"Організації",
"user-restrictions.group.profile":"Профіль",
"user-restrictions.group.versions":"Версії",
"user-restrictions.scope.collection-create":"Створювати колекції",
"user-restrictions.scope.collection-delete":"Видаляти колекції",
"user-restrictions.scope.collection-write":"Редагувати колекції",
"user-restrictions.scope.organization-create":"Створювати організації",
"user-restrictions.scope.organization-delete":"Видаляти організації",
"user-restrictions.scope.organization-write":"Редагувати організації",
"user-restrictions.scope.project-create":"Створювати нові проєкти",
"user-restrictions.scope.project-delete":"Видаляти проєкти",
"user-restrictions.scope.project-write":"Редагувати проєкти, керувати командою й приймати запрошення",
"user-restrictions.scope.user-write":"Редагувати профіль, відстежувати проєкти та змінювати налаштування",
"user-restrictions.scope.version-create":"Завантажувати нові версії",
"user-restrictions.scope.version-delete":"Видаляти версії та їхні файли",
"user-restrictions.scope.version-write":"Редагувати версії та додавати файли"
}
NEW = {
"fre-launch.cf.source":("Джерело модів","Mod source"),
"fre-launch.cf.intro":("Моди з CurseForge можна встановлювати в ту саму збірку, що й моди з Modrinth.","Mods from CurseForge can be installed into the same game instance as Modrinth mods."),
"fre-launch.cf.pack-intro":("Шукай модпаки CurseForge та відкривай їхні сторінки. Автоматичне встановлення ZIP-модпаків поки недоступне.","Find CurseForge modpacks and open their project pages. Installing CurseForge ZIP packs is not available yet."),
"fre-launch.cf.pack-search":("Знайти модпаки","Find modpacks"),
"fre-launch.cf.api-key":("Схвалений ключ CurseForge API (не зберігається)","Approved CurseForge API key (not saved)"),
"fre-launch.cf.key-help":("Пошук на сайті працює без ключа. Режим API вмикай лише зі схваленим доступом; ключ не зберігається.","Website search needs no key. Only enable API mode if you have approved access; the key is not saved."),
"fre-launch.cf.direct-intro":("Шукай на офіційному сайті CurseForge без API-ключа. Файли завантажуються через браузер.","Search on the official CurseForge website without an API key. Downloads are handled by your browser."),
"fre-launch.cf.mode-api":("Пошук через API прямо в лаунчері (додатково, потрібен схвалений доступ)","Use in-app API search (advanced, requires approved access)"),
"fre-launch.cf.site-search":("Шукати на сайті CurseForge","Search CurseForge website"),
"fre-launch.cf.site-opened":("Пошук відкрито у браузері. Завантаж сумісний мод із CurseForge й імпортуй JAR тут.","Opened search results in your browser. Download a compatible file from CurseForge, then import the JAR here."),
"fre-launch.cf.pack-opened":("Модпаки CurseForge відкрито у браузері. Імпорт цілих модпаків у цій панелі поки недоступний.","Opened CurseForge modpacks in your browser. Importing full modpacks is not supported in this panel."),
"fre-launch.cf.import-file":("Імпортувати завантажений JAR у цю збірку","Import downloaded JAR to this instance"),
"fre-launch.cf.importing-file":("Імпортуємо файл...","Importing file..."),
"fre-launch.cf.imported-file":("JAR імпортовано. Перевір версію Minecraft, модлоадер і залежності перед запуском.","JAR imported. Check the Minecraft version, loader and dependencies before playing."),
"fre-launch.cf.import-file-error":("Не вдалося імпортувати JAR: {reason}","Cannot import downloaded JAR: {reason}"),
"fre-launch.cf.import-file-help":("Завантаж потрібний JAR із CurseForge й додай його до вибраної Java-збірки. Залежності та сумісність не перевіряються.","Download the correct JAR on CurseForge, then import it to the selected Java instance. This does not check dependencies or mod compatibility."),
"fre-launch.cf.key-link":("Як отримати доступ","How to request access"),
"fre-launch.cf.instance":("Установити в збірку","Install to game instance"),
"fre-launch.cf.select-instance":("Вибери збірку","Select a game instance"),
"fre-launch.cf.search":("Пошук CurseForge","Search CurseForge"),
"fre-launch.cf.for-instance":("Знайти моди CurseForge для цієї збірки", "Find CurseForge mods for this instance"),
"fre-launch.cf.placeholder":("Знайти мод для Minecraft...","Find a Minecraft mod..."),
"fre-launch.cf.search-button":("Знайти моди","Find mods"),
"fre-launch.cf.searching":("Шукаємо...","Searching..."),
"fre-launch.cf.compatible":("Установлюються лише файли, сумісні з вибраною версією Minecraft і модлоадером.","Only compatible files will be installed for the selected Minecraft version and mod loader."),
"fre-launch.cf.no-instance":("Створи збірку з Fabric, Forge, NeoForge або Quilt, щоб установлювати моди.","Create a Fabric, Forge, NeoForge or Quilt instance to install mods."),
"fre-launch.cf.results":("Результати","Results"),
"fre-launch.cf.no-results":("Модів не знайдено. Спробуй інший запит або версію.","No mods found. Try another search or version."),
"fre-launch.cf.search-error":("Пошук CurseForge недоступний: {reason}","CurseForge search is unavailable: {reason}"),
"fre-launch.cf.install":("Установити в збірку","Install to instance"),
"fre-launch.cf.installing":("Установлюємо...","Installing..."),
"fre-launch.cf.success":("Мод {name} установлено у вибрану збірку. Перед запуском перевір потрібні залежності.","Installed {name} into the selected instance. Check required dependencies before playing."),
"fre-launch.cf.install-error":("Не вдалося встановити: {reason}","Install failed: {reason}"),
"fre-launch.cf.website":("Відкрити на CurseForge","Open on CurseForge"),
"fre-launch.cf.distribution":("Автор заборонив стороннє розповсюдження","Author has disabled third-party distribution"),
"fre-launch.cf.warning":("Моди з різних джерел можуть бути в одній папці mods, але залежності й конфлікти не вирішуються автоматично. Спершу зроби резервну копію збірки.","Different sources can share one mods folder, but dependencies and mod conflicts are not automatically resolved. Back up your instance first."),
"fre-launch.cf.browse-link":("Відкрити каталог CurseForge","Open CurseForge catalog"),
"fre-launch.welcome-screen.title":("Вітаємо у FRE LAUNCH","Welcome to FRE LAUNCH"),
"fre-launch.skins.local-notice":("Локальний нік не має Microsoft-авторизації. Ти можеш вибрати скін або завантажити PNG (64×64 чи 64×32); FRE LAUNCH збереже його для цього ніку та підготує для запуску гри.","This local nickname has no Microsoft authentication. You can select a skin or upload a PNG (64x64 or 64x32); FRE LAUNCH will save it for this nickname and prepare it when launching Minecraft."),
"fre-launch.skins.local-upload":("Вибрати PNG-скін","Choose PNG skin"),
"fre-launch.skins.show-help":("Як це працює?","How does this work?"),
"fre-launch.skins.hide-help":("Сховати","Hide help"),
"fre-launch.skins.local-saved":("Локальний скін збережено для цього ніку","Local skin saved for this nickname"),
"fre-launch.skins.local-mod":("Щоб скін з’явився в Minecraft, встанови сумісний клієнтський мод (наприклад CustomSkinLoader) у цю збірку. Звичайна Vanilla не показує непідписані офлайн-скіни. Інші гравці можуть його не бачити.","For the skin to appear in Minecraft, install a compatible client-side skin mod (such as CustomSkinLoader) in this game instance. Vanilla Minecraft does not show unsigned offline skins. Other players may not see it."),
"fre-launch.offline.title":("Локальний нік","Local nickname"),
"fre-launch.offline.placeholder":("Введи нік","Your nickname"),
"fre-launch.offline.create":("Увійти за ніком","Use nickname"),
"fre-launch.offline.short":("Локальний","Local"),
"fre-launch.offline.microsoft":("Microsoft","Microsoft"),
"fre-launch.offline.confirm-remove":("Видалити профіль {nickname} із лаунчера?","Remove Minecraft profile {nickname} from this launcher?"),
"fre-launch.guest.use-account":("Використовувати збережений акаунт","Use saved account"),
"fre-launch.guest.nickname":("Нік (без акаунта)","Nickname (without account)"),
"fre-launch.guest.upload-skin":("Завантажити PNG-скін для цього ніку","Upload PNG skin for this nickname"),
"fre-launch.guest.skin-saved":("Скін збережено. Встанови CustomSkinLoader у цю збірку та перезапусти Minecraft.","Saved. Install CustomSkinLoader in this instance, then restart Minecraft."),
"fre-launch.guest.skin-invalid":("Обери PNG-скін Minecraft (64×64 або 64×32) розміром до 512 КіБ.","Choose a PNG Minecraft skin (64x64 or 64x32) under 512 KiB."),
"fre-launch.guest.placeholder":("Введи нік","Enter nickname"),
"fre-launch.guest.notice":("Microsoft-вхід не потрібен. На серверах без перевірки ліцензії введи свій раніше зареєстрований нік. Якщо сервер попросить пароль, увійди через його /login.","No Microsoft login needed. For servers allowing non-premium players, use the same nickname you registered there and enter your server password when prompted."),
"fre-launch.guest.invalid":("Дозволено 1–16 англійських літер, цифр або символів _.","Use 1-16 English letters, numbers, or underscores."),
"fre-launch.guest.format-ok":("Нік підходить для офлайн-запуску.","Nickname is valid for offline launch."),
"fre-launch.guest.already-saved":("Цей нік є серед твоїх збережених локальних профілів. Гостьовий запуск не створить і не вибере такий акаунт.","This nickname is also in your saved local profiles. Guest launch will not create or use that saved account."),
"fre-launch.guest.server-check":("Чи зареєстрований нік на MineBlaze, дізнаєшся при вході: сервер запропонує /register або /login.","Server registration cannot be checked here: MineBlaze will tell you to register or log in after connecting."),
"fre-launch.guest.java-checking":("Перевірка ніку Minecraft Java...","Checking Minecraft Java name..."),
"fre-launch.guest.java-used":("Цей нік уже належить чинному профілю Minecraft Java.","This name belongs to an active Minecraft Java profile."),
"fre-launch.guest.java-no-profile":("Чинний Java-профіль із таким ніком не знайдено. Це не гарантує доступність імені.","No active Java profile found. Name availability is not guaranteed."),
"fre-launch.guest.java-unavailable":("Зараз неможливо перевірити нік Minecraft.","Minecraft name lookup is temporarily unavailable."),
"fre-launch.offline.disclaimer":("Для одиночної гри та серверів без перевірки ліцензії. Це не Microsoft-акаунт.","For single-player and offline-mode servers only. Not a Microsoft account.")
}

# Finish untranslated proper UI labels and visible skin-series names.
# Never overwrite a Ukrainian value that differs from the upstream English.
POLISH_EXISTING_APP = {
    "app.skins.section.builders-and-biomes": "Будівельники й біоми",
    "app.skins.section.chaos-cubed": "Кубічний хаос",
    "app.skins.section.chase-the-skies": "Назустріч небесам",
    "app.skins.section.mounts-of-mayhem": "Верхи хаосу",
    "app.skins.section.striding-hero": "Герой-мандрівник",
    "app.skins.section.the-copper-age": "Мідна доба",
    "app.skins.section.the-garden-awakens": "Пробудження саду",
    "app.skins.section.tiny-takeover": "Маленький переворот",
    "app.skins.section.wilderness-bound": "Назустріч дикій природі",
}
POLISH_EXISTING_UI = {
    "changelog.product.app": "Застосунок",
    "changelog.product.hosting": "Хостинг",
    "tag.category.oneblock": "Один блок",
}


def _polish_untranslated_labels(english, ukrainian, suggested, field):
    changed = 0
    for key, translation in suggested.items():
        if key not in english or key not in ukrainian:
            raise RuntimeError(f"Ukrainian polish key not found: {key}")
        # Preserve all Ukrainian translations already supplied by contributors.
        if ukrainian[key][field] == english[key][field]:
            ukrainian[key][field] = translation
            changed += 1
    return changed


def _complete(english, ukrainian, curated, field):
    unexpected=set(curated)-set(english)
    if unexpected:
        raise RuntimeError(f"Translation key mismatch: {sorted(unexpected)}")
    added=0
    english_fallback=0
    for key, entry in english.items():
        if key in ukrainian and str(ukrainian[key].get(field,"")).strip():
            continue
        if key in curated:
            ukrainian[key]={field:curated[key]}
            added+=1
        else:
            ukrainian[key]={field:entry[field]}
            english_fallback+=1
    return added, english_fallback

# Distinct from Java game instance rows: Rust profiles appear as independent
# cards use FRE-owned local profile records, not a second launcher.
NEW.update({
"fre-launch.rust.library.title":("Rust — мої збірки","Rust — my instances"),
"fre-launch.rust.library.subtitle":("Збірки Minecraft на Rust із власними світами та налаштуваннями у FRE LAUNCH.","Rust Minecraft instances with separate worlds and settings in FRE LAUNCH."),
"fre-launch.rust.library.version":("Версія {version}","Version {version}"),
"fre-launch.rust.library.play":("Грати","Play"),
"fre-launch.rust.library.playing":("Запускаємо…","Starting..."),
"fre-launch.rust.library.manage":("Налаштувати","Manage"),
"fre-launch.rust.library.files":("Файли","Files"),
"fre-launch.rust.library.add":("Нова Rust-збірка","New Rust profile"),
"fre-launch.rust.library.needs-prepare":("Потрібно встановити гру або підготувати ресурси","Installation or resources required"),
"fre-launch.rust.library.empty":("Збірок Minecraft на Rust поки немає.", "No Rust Minecraft instances yet."),
"fre-launch.rust.library.ready":("Готово до запуску","Ready to play"),
"fre-launch.rust.library.error":("Rust: {reason}","Rust: {reason}"),
})


# Rust game is integrated into FRE LAUNCH's native profile library.
NEW.update({
"fre-rust.setup.title":("Minecraft на Rust — FRE LAUNCH", "Minecraft on Rust — FRE LAUNCH"),
"fre-rust.setup.intro":("Твої власні Rust-збірки. FRE LAUNCH запускає Minecraft напряму, без іншого лаунчера.", "Your own Rust Minecraft profiles, installed and launched directly by FRE LAUNCH. No second launcher."),
"fre-rust.setup.name":("Назва збірки", "Profile name"),
"fre-rust.setup.create":("Створити Rust-збірку", "Create Rust profile"),
"fre-rust.setup.creating":("Створюємо…", "Creating..."),
"fre-rust.setup.play":("Грати", "Play"),
"fre-rust.setup.playing":("Запускаємо…", "Starting..."),
"fre-rust.setup.close":("Закрити", "Close"),
"fre-rust.setup.installed":("Rust-гру вбудовано", "Native Rust game bundled"),
"fre-rust.setup.missing":("У цій збірці FRE LAUNCH ще немає Rust-гри.", "Native Rust game is not installed in this FRE LAUNCH build."),
"fre-rust.setup.ready":("Ресурси Minecraft готові", "Minecraft resources prepared"),
"fre-rust.setup.not-ready":("Ресурси Minecraft не підготовлені", "Minecraft resources not prepared"),
"fre-rust.setup.details":("FRE LAUNCH сам підготує Minecraft 26.3 через власний кеш і перевірені ресурси Mojang для власника Java Edition. Інший лаунчер не потрібен.", "FRE LAUNCH prepares Minecraft 26.3 using its own cache and verified Mojang assets for a Java Edition owner. No second launcher is needed."),
"fre-rust.setup.source":("Папка підготованих ресурсів (необов'язково — залиш порожнім для автопошуку)", "Prepared game resources folder (optional; leave blank for automatic detection)"),
"fre-rust.setup.browse":("Вибрати папку", "Choose folder"),
"fre-rust.setup.import":("Підготувати або завантажити Minecraft", "Prepare or download Minecraft"),
"fre-rust.setup.importing":("Підготовка Minecraft…", "Preparing Minecraft…"),
"fre-rust.setup.import-help":("Використай свій Microsoft-акаунт із Minecraft Java Edition у FRE LAUNCH. Якщо є підготовлені ресурси, їх також можна імпортувати.", "Use your Minecraft Java Edition Microsoft account in FRE LAUNCH. You can also import previously prepared resources."),
"fre-rust.setup.sign-in-horizon":("Увійти через FRE LAUNCH", "Sign in with FRE LAUNCH"),
"fre-rust.setup.none":("Rust-збірок ще немає.", "No Rust profiles yet."),
"fre-rust.setup.logs":("Показати лог гри", "Show game log"),
"fre-rust.setup.files":("Файли", "Files"),
"fre-rust.setup.worlds":("Світи", "Worlds"),
"fre-rust.setup.mods":("Нативні моди", "Native mods"),
"fre-rust.setup.resourcepacks":("Ресурс-паки", "Resource packs"),
"fre-rust.setup.shaderpacks":("Шейдери", "Shader packs"),
"fre-rust.setup.error":("Помилка: {reason}", "Error: {reason}"),
"fre-rust.library.title":("Minecraft на Rust", "Minecraft on Rust"),
"fre-rust.library.new":("Створити Rust-збірку", "Create Rust profile"),
"fre-rust.library.empty":("Rust-збірок ще немає. Створи збірку, і вона з’явиться в бібліотеці.", "No Rust profiles yet. Create one to see it in your library."),
"fre-rust.library.ready":("Готово", "Ready"),
"fre-rust.library.install":("Потрібні файли гри", "Game files required"),
"fre-rust.library.play":("Грати", "Play"),
"fre-rust.library.stop":("Зупинити", "Stop"),
"fre-rust.library.playing":("Запускаємо…", "Starting..."),
"fre-rust.library.manage":("Налаштувати", "Manage"),
"fre-rust.library.files":("Файли", "Files"),
"fre-rust.library.mods-info":("Minecraft на Rust використовує нативні моди .hmod; Java-моди (.jar) несумісні.", "Rust Minecraft uses native .hmod content mods; Java .jar mods are incompatible."),
"fre-rust.library.error":("Rust-гра: {reason}", "Rust game: {reason}"),
})

def apply(root):
    app_uk=root/"apps/app-frontend/src/locales/uk-UA/index.json"
    app_en=root/"apps/app-frontend/src/locales/en-US/index.json"
    ui_uk=root/"packages/ui/src/locales/uk-UA/index.json"
    ui_en=root/"packages/ui/src/locales/en-US/index.json"
    ea=json.loads(app_en.read_text(encoding="utf-8"))
    ua=json.loads(app_uk.read_text(encoding="utf-8"))
    eu=json.loads(ui_en.read_text(encoding="utf-8"))
    uu=json.loads(ui_uk.read_text(encoding="utf-8"))
    app,_a=_complete(ea,ua,APP,"message")
    ui,_u=_complete(eu,uu,UI,"defaultMessage")
    polished_app=_polish_untranslated_labels(ea,ua,POLISH_EXISTING_APP,"message")
    polished_ui=_polish_untranslated_labels(eu,uu,POLISH_EXISTING_UI,"defaultMessage")
    for key,(uk,en) in NEW.items():
        if key in ua or key in ea:
            raise RuntimeError(f"Locale name collision: {key}")
        ua[key]={"message":uk}
        ea[key]={"message":en}
    # A nickname-only profile completes onboarding without a Microsoft login;
    # name this as choosing a profile, not signing into official services.
    ea["onboarding-checklist.login-minecraft"]={"message":"Set up Minecraft profile"}
    ua["onboarding-checklist.login-minecraft"]={"message":"Налаштувати профіль Minecraft"}
    # App settings must show the fork name rather than the upstream trademark.
    for catalog in (ea, ua):
        if "app.settings.app-version" in catalog:
            catalog["app.settings.app-version"]["message"] = "FRE LAUNCH {version}"
    for path, content in [(app_en,ea),(app_uk,ua),(ui_en,eu),(ui_uk,uu)]:
        path.write_text(json.dumps(content,ensure_ascii=False,indent=2,sort_keys=True)+"\n",encoding="utf-8")
    print(f"Ukrainian translations completed: app={app}, UI={ui}, new profiles={len(NEW)}. "
          f"English-only fallback strings app={_a}, UI={_u}. "
          f"Existing untranslated labels polished: app={polished_app}, UI={polished_ui}.")
